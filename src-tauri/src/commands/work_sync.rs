//! 多机工作记录同步命令：状态查询、导出/导入批次、游标推进。
//! 编排在 TS 引擎（src/lib/utils/workSync.ts）；这里只暴露本地库操作。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tauri::State;

use crate::work_sync::{self, SyncActivityPayload, WorkSyncState};
use crate::AppState;
use work_review_core::database::RemoteImportOutcome;

fn lock_err<T>(e: std::sync::PoisonError<T>) -> String {
    e.to_string()
}

#[derive(Serialize)]
pub struct WorkSyncExportBatch {
    pub activities: Vec<SyncActivityPayload>,
    /// 已导出的本地行 id（推送成功后回传 work_sync_mark_synced 清脏标记）
    pub ids: Vec<i64>,
    /// 批内最大 rowid（推送成功后作为新推送游标）
    pub max_id: i64,
    /// 批次打满=可能还有更多（前端循环取直到 false）
    pub has_more: bool,
}

/// 游标推进（三段可选，只前进不后退）：
/// - pushed_seq：本机在云端的块序号（推送成功后上报）
/// - pull_cursors：各远端机器已拉到的 seq（按机器取大者合并）
/// - report_pull_time：日报拉取游标（云端 updatedAt 毫秒）
#[derive(Deserialize, Default)]
pub struct WorkSyncCursorUpdate {
    #[serde(default)]
    pub pushed_seq: Option<u64>,
    #[serde(default)]
    pub pull_cursors: Option<HashMap<String, u64>>,
    #[serde(default)]
    pub report_pull_time: Option<i64>,
}

#[tauri::command]
pub fn work_sync_get_state(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<WorkSyncState, String> {
    let app = state.lock().map_err(lock_err)?;
    work_sync::ensure_machine_id(&app.data_dir)?;
    Ok(work_sync::load_state(&app.data_dir))
}

#[tauri::command]
pub fn work_sync_export_batch(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<WorkSyncExportBatch, String> {
    let app = state.lock().map_err(lock_err)?;
    let (activities, ids, max_id, has_more) =
        work_sync::export_push_batch(&app.database, &app.data_dir)?;
    Ok(WorkSyncExportBatch {
        activities,
        ids,
        max_id,
        has_more,
    })
}

#[tauri::command]
pub fn work_sync_mark_synced(
    state: State<'_, Arc<Mutex<AppState>>>,
    ids: Vec<i64>,
    max_id: i64,
) -> Result<(), String> {
    let app = state.lock().map_err(lock_err)?;
    app.database
        .mark_activities_synced(&ids)
        .map_err(|e| e.to_string())?;
    let mut ws = work_sync::load_state(&app.data_dir);
    ws.last_pushed_activity_id = ws.last_pushed_activity_id.max(max_id);
    work_sync::save_state(&app.data_dir, &ws)
}

#[tauri::command]
pub fn work_sync_import_batch(
    state: State<'_, Arc<Mutex<AppState>>>,
    machine_id: String,
    activities: Vec<SyncActivityPayload>,
) -> Result<RemoteImportOutcome, String> {
    let machine_id = machine_id.trim().to_string();
    if machine_id.is_empty() {
        return Err("machine_id 不能为空".to_string());
    }
    let app = state.lock().map_err(lock_err)?;
    let custom = app.config.remote_control_apps.clone();
    work_sync::import_remote_batch(&app.database, &machine_id, &activities, &custom)
}

#[tauri::command]
pub fn work_sync_advance_cursors(
    state: State<'_, Arc<Mutex<AppState>>>,
    update: WorkSyncCursorUpdate,
) -> Result<(), String> {
    let app = state.lock().map_err(lock_err)?;
    let mut ws = work_sync::load_state(&app.data_dir);
    if let Some(seq) = update.pushed_seq {
        ws.last_pushed_seq = ws.last_pushed_seq.max(seq);
    }
    if let Some(cursors) = update.pull_cursors {
        for (machine, seq) in cursors {
            let entry = ws.pull_cursors.entry(machine).or_insert(0);
            *entry = (*entry).max(seq);
        }
    }
    if let Some(t) = update.report_pull_time {
        ws.last_report_pull_time = ws.last_report_pull_time.max(t);
    }
    work_sync::save_state(&app.data_dir, &ws)
}

/// 日报区间读取（同步专用：带 updated_at/origin_machine 元数据）
#[tauri::command]
pub fn get_reports_in_range_for_sync(
    state: State<'_, Arc<Mutex<AppState>>>,
    start_date: String,
    end_date: String,
    locale: Option<String>,
) -> Result<Vec<work_review_core::database::DailyReport>, String> {
    let app = state.lock().map_err(lock_err)?;
    app.database
        .get_reports_in_range(&start_date, &end_date, locale.as_deref())
        .map_err(|e| e.to_string())
}

/// 落地云端日报（LWW 判定在服务端/引擎侧完成）
#[tauri::command]
pub fn apply_remote_report(
    state: State<'_, Arc<Mutex<AppState>>>,
    report: work_review_core::database::DailyReport,
    updated_at: i64,
    origin_machine: String,
) -> Result<(), String> {
    let app = state.lock().map_err(lock_err)?;
    app.database
        .apply_remote_report(&report, updated_at, &origin_machine)
        .map_err(|e| e.to_string())
}
