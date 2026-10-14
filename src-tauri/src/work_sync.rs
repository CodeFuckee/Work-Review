// 多机工作记录同步：本机身份/游标状态 + 推拉数据命令层。
// 编排（定时、AES-GCM 加密、HTTP 调用）在前端 TS（workSync.ts）——JWT 与
// 派生密钥都活在 WebView；Rust 侧只做本地库的导出/导入与跨轮状态持久化，
// 状态文件模式参照 node_gateway.rs 的数据目录 JSON 身份文件。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use work_review_core::categorize::REMOTE_CONTROL_APP_PATTERNS;
use work_review_core::database::{Database, RemoteImportOutcome};

/// 单块活动条数上限：配合 5 分钟轮询，单块明文 ~0.5MB、密文 b64 后 ~0.7MB
pub const EXPORT_BATCH_LIMIT: usize = 200;

/// 跨轮同步状态（数据目录 work-sync-state.json）
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct WorkSyncState {
    /// 本机机器身份：首次启用同步时生成，此后终身不变（重装应用不换、
    /// 清数据也不换——它是"这台机器"的标识，不是数据库的标识）
    #[serde(default)]
    pub machine_id: String,
    /// 推送游标：已推送到云端的本地最大活动 rowid
    #[serde(default)]
    pub last_pushed_activity_id: i64,
    /// 本机在云端的块序号（重复推送防冲突；状态文件丢失时从服务端 latestSeq 恢复）
    #[serde(default)]
    pub last_pushed_seq: u64,
    /// 各远端机器的拉取游标（machineId → 已拉到的最大 seq）
    #[serde(default)]
    pub pull_cursors: HashMap<String, u64>,
    /// 日报拉取游标（云端 updatedAt 毫秒时间戳）
    #[serde(default)]
    pub last_report_pull_time: i64,
}

pub fn state_file_path(data_dir: &Path) -> PathBuf {
    data_dir.join("work-sync-state.json")
}

/// 读取同步状态：文件缺失/损坏一律回退默认值（状态可从服务端游标恢复，
/// 不能因状态文件损坏阻断采集主流程）
pub fn load_state(data_dir: &Path) -> WorkSyncState {
    let path = state_file_path(data_dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_default(),
        Err(_) => WorkSyncState::default(),
    }
}

pub fn save_state(data_dir: &Path, state: &WorkSyncState) -> Result<(), String> {
    let path = state_file_path(data_dir);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    // 临时文件 + 原子改名：断电不落半截 JSON（load 对损坏已兜底，但别制造损坏）
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 确保机器身份存在：缺失时生成 UUID 并落盘（幂等）
pub fn ensure_machine_id(data_dir: &Path) -> Result<String, String> {
    let mut state = load_state(data_dir);
    if !state.machine_id.trim().is_empty() {
        return Ok(state.machine_id);
    }
    state.machine_id = uuid::Uuid::new_v4().to_string();
    save_state(data_dir, &state)?;
    Ok(state.machine_id)
}

/// 推送载荷：剥离无跨机意义的本地字段——screenshot_path / executable_path
/// 是本机文件系统路径（泄露目录结构且对方机器上无效），screenshot_url 属
/// 截图外置通道（用户已选不同步截图），id/sync_dirty 是纯本地概念。
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SyncActivityPayload {
    pub uid: String,
    pub timestamp: i64,
    pub app_name: String,
    pub window_title: String,
    pub ocr_text: Option<String>,
    pub category: String,
    pub duration: i64,
    pub browser_url: Option<String>,
    pub semantic_category: Option<String>,
    pub semantic_confidence: Option<i32>,
}

pub fn build_payload(activity: &work_review_core::database::Activity) -> SyncActivityPayload {
    SyncActivityPayload {
        uid: activity.uid.clone(),
        timestamp: activity.timestamp,
        app_name: activity.app_name.clone(),
        window_title: activity.window_title.clone(),
        ocr_text: activity.ocr_text.clone(),
        category: activity.category.clone(),
        duration: activity.duration,
        browser_url: activity.browser_url.clone(),
        semantic_category: activity.semantic_category.clone(),
        semantic_confidence: activity.semantic_confidence,
    }
}

/// 导出一批待推送活动：先回填机器归属（旧行 machine_id='' → 本机 id），
/// 再按游标取 dirty/新行。返回 (载荷, 已取行 id, 批内最大 rowid, 是否还有更多)。
pub fn export_push_batch(
    db: &Database,
    data_dir: &Path,
) -> Result<(Vec<SyncActivityPayload>, Vec<i64>, i64, bool), String> {
    let machine_id = ensure_machine_id(data_dir)?;
    let backfilled = db
        .backfill_local_machine_id(&machine_id)
        .map_err(|e| e.to_string())?;
    if backfilled > 0 {
        log::info!("work-sync: 回填 {backfilled} 行归属到本机 {machine_id}");
    }
    let state = load_state(data_dir);
    let rows = db
        .get_local_activities_for_sync(state.last_pushed_activity_id, EXPORT_BATCH_LIMIT)
        .map_err(|e| e.to_string())?;
    let ids: Vec<i64> = rows.iter().filter_map(|r| r.id).collect();
    let max_id = rows.iter().filter_map(|r| r.id).max().unwrap_or(0);
    let has_more = rows.len() == EXPORT_BATCH_LIMIT;
    Ok((
        rows.iter().map(build_payload).collect(),
        ids,
        max_id,
        has_more,
    ))
}

/// 导入一批远端活动并跑影子标记：内置远控模式 + 用户自定义模式一起生效。
pub fn import_remote_batch(
    db: &Database,
    machine_id: &str,
    rows: &[SyncActivityPayload],
    custom_patterns: &[String],
) -> Result<RemoteImportOutcome, String> {
    // 载荷 → Activity（is_shadow 由标记阶段统一判定，导入时一律未标记）
    let activities: Vec<work_review_core::database::Activity> = rows
        .iter()
        .map(|p| work_review_core::database::Activity {
            id: None,
            uid: p.uid.clone(),
            machine_id: String::new(),
            origin: "cloud".to_string(),
            is_shadow: false,
            timestamp: p.timestamp,
            app_name: p.app_name.clone(),
            window_title: p.window_title.clone(),
            screenshot_path: String::new(),
            ocr_text: p.ocr_text.clone(),
            category: p.category.clone(),
            duration: p.duration,
            browser_url: p.browser_url.clone(),
            executable_path: None,
            semantic_category: p.semantic_category.clone(),
            semantic_confidence: p.semantic_confidence,
            screenshot_url: None,
        })
        .collect();
    let outcome = db
        .insert_activities_remote(&activities, machine_id)
        .map_err(|e| e.to_string())?;
    let mut patterns: Vec<String> = REMOTE_CONTROL_APP_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect();
    patterns.extend(custom_patterns.iter().cloned());
    let shadowed = db
        .mark_remote_control_shadows(&patterns)
        .map_err(|e| e.to_string())?;
    if shadowed > 0 {
        log::info!("work-sync: 影子标记 {shadowed} 条远控壳记录");
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use work_review_core::database::Activity;

    fn temp_dir(name: &str) -> PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("work-review-ws-{name}-{unique}"))
    }

    fn cleanup(path: &Path) {
        let _ = std::fs::remove_dir_all(path);
    }

    #[test]
    fn 状态文件往返一致() {
        let dir = temp_dir("roundtrip");
        let mut state = WorkSyncState {
            machine_id: "machA".to_string(),
            last_pushed_activity_id: 42,
            last_pushed_seq: 7,
            pull_cursors: HashMap::from([("machB".to_string(), 3)]),
            last_report_pull_time: 1_760_000_000_000,
        };
        save_state(&dir, &state).expect("保存状态失败");
        let loaded = load_state(&dir);
        assert_eq!(loaded, state);
        state.last_pushed_seq = 8;
        save_state(&dir, &state).expect("二次保存失败");
        assert_eq!(load_state(&dir).last_pushed_seq, 8);
        cleanup(&dir);
    }

    #[test]
    fn 缺失或损坏的状态文件回退默认值() {
        let dir = temp_dir("corrupt");
        assert_eq!(
            load_state(&dir),
            WorkSyncState::default(),
            "缺失文件应回默认"
        );
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(state_file_path(&dir), "{not json").unwrap();
        assert_eq!(
            load_state(&dir),
            WorkSyncState::default(),
            "损坏文件应回默认"
        );
        cleanup(&dir);
    }

    #[test]
    fn 机器身份生成幂等() {
        let dir = temp_dir("machine-id");
        let first = ensure_machine_id(&dir).expect("首次生成失败");
        let second = ensure_machine_id(&dir).expect("二次读取失败");
        assert_eq!(first, second, "机器身份终身不变");
        assert!(!first.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn 推送载荷剥离本地路径与影子字段() {
        let activity = Activity {
            id: Some(7),
            timestamp: 1_000,
            app_name: "Code".to_string(),
            window_title: "编辑器".to_string(),
            screenshot_path: "/home/ckd/screenshots/x.png".to_string(),
            ocr_text: Some("文本".to_string()),
            category: "development".to_string(),
            duration: 60,
            browser_url: Some("https://example.com".to_string()),
            executable_path: Some("/usr/bin/code".to_string()),
            semantic_category: Some("开发".to_string()),
            semantic_confidence: Some(90),
            screenshot_url: Some("https://s3.example/x.png".to_string()),
            uid: "u1".to_string(),
            machine_id: "machA".to_string(),
            origin: "local".to_string(),
            is_shadow: true,
        };
        let payload = build_payload(&activity);
        let json = serde_json::to_string(&payload).unwrap();
        assert!(!json.contains("screenshots"), "不得携带本机截图路径");
        assert!(
            !json.contains("/usr/bin/code"),
            "不得携带本机可执行文件路径"
        );
        assert!(!json.contains("s3.example"), "用户已选不同步截图");
        assert_eq!(payload.uid, "u1");
        assert_eq!(payload.duration, 60);
    }

    #[test]
    fn 导出批次只含本机行且分批报告has_more() {
        let dir = temp_dir("export");
        let db_path = dir.join("workreview.db");
        std::fs::create_dir_all(&dir).unwrap();
        let db = Database::new(&db_path).expect("建库失败");

        let mk = |ts: i64, title: &str| Activity {
            id: None,
            uid: String::new(),
            machine_id: String::new(),
            origin: "local".to_string(),
            is_shadow: false,
            timestamp: ts,
            app_name: "Code".to_string(),
            window_title: title.to_string(),
            screenshot_path: String::new(),
            ocr_text: None,
            category: "development".to_string(),
            duration: 60,
            browser_url: None,
            executable_path: None,
            semantic_category: None,
            semantic_confidence: None,
            screenshot_url: None,
        };
        for i in 0..3 {
            db.insert_activity(&mk(1_000 + i, &format!("t{i}")))
                .unwrap();
        }
        // 云端行不得进入导出（走 import_remote_batch 真实入库路径）
        import_remote_batch(
            &db,
            "machB",
            &[super::SyncActivityPayload {
                uid: "machB:u1".to_string(),
                timestamp: 2_000,
                app_name: "IDEA".to_string(),
                window_title: "远端".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 60,
                browser_url: None,
                semantic_category: None,
                semantic_confidence: None,
            }],
            &[],
        )
        .unwrap();

        let (payloads, ids, max_id, has_more) = export_push_batch(&db, &dir).unwrap();
        assert_eq!(payloads.len(), 3, "只导出本机行");
        assert_eq!(ids.len(), 3);
        assert_eq!(max_id, 3, "批内最大 rowid（云端行占用的 rowid 除外不计）");
        assert!(!has_more);
        assert!(payloads.iter().all(|p| !p.uid.is_empty()));

        // 首次导出完成回填：状态未推游标，但行已属本机、未脏 → 下一批为空
        db.mark_activities_synced(&ids).unwrap();
        save_state(
            &dir,
            &WorkSyncState {
                last_pushed_activity_id: max_id,
                ..load_state(&dir)
            },
        )
        .unwrap();
        let (next, _, _, _) = export_push_batch(&db, &dir).unwrap();
        assert!(next.is_empty(), "已同步行不应重复导出");
        cleanup(&dir);
    }

    #[test]
    fn 导入批次触发影子标记() {
        let dir = temp_dir("import-shadow");
        let db_path = dir.join("workreview.db");
        std::fs::create_dir_all(&dir).unwrap();
        let db = Database::new(&db_path).expect("建库失败");

        // 本机远控行 + 对方同时段真实活动 → 导入后本机行应被影子化
        let mut local = Activity {
            id: None,
            uid: String::new(),
            machine_id: String::new(),
            origin: "local".to_string(),
            is_shadow: false,
            timestamp: 1_760_000_000,
            app_name: "SunloginClient".to_string(),
            window_title: "远程桌面".to_string(),
            screenshot_path: String::new(),
            ocr_text: None,
            category: "other".to_string(),
            duration: 3_600,
            browser_url: None,
            executable_path: None,
            semantic_category: None,
            semantic_confidence: None,
            screenshot_url: None,
        };
        local.timestamp = chrono::Local::now().timestamp();
        db.insert_activity(&local).unwrap();

        let outcome = import_remote_batch(
            &db,
            "machB",
            &[super::SyncActivityPayload {
                uid: "machB:u1".to_string(),
                timestamp: chrono::Local::now().timestamp() + 60,
                app_name: "IDEA".to_string(),
                window_title: "远端写代码".to_string(),
                ocr_text: None,
                category: "development".to_string(),
                duration: 600,
                browser_url: None,
                semantic_category: None,
                semantic_confidence: None,
            }],
            &[],
        )
        .unwrap();
        assert_eq!(outcome.inserted, 1);

        let rows = db.get_local_activities_for_sync(0, 10).unwrap();
        assert!(
            rows.iter().any(|r| r.is_shadow),
            "导入后应立即对本机远控行跑影子标记"
        );
        cleanup(&dir);
    }
}
