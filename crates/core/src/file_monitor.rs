//! 本地文件修改监控：监听用户配置的工作文件夹，将文件事件（新建/修改/删除/重命名）
//! 防抖聚合后入库，供概览页「文件活动」卡片展示。
//!
//! 隐私边界：只记录文件路径、事件类型、时间戳、文件大小四类字段，
//! 绝不读取或记录文件内容。
//!
//! 分层约定：`is_path_excluded` / `map_event_kind` 为纯函数可单测；
//! `FileMonitorHandle` 负责 notify watcher 生命周期与事件到回调的分发。

use crate::database::FileEventRow;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use notify_debouncer_full::{new_debouncer, Debouncer};
use std::path::Path;
use std::time::Duration;

/// 同一路径同一事件的防抖窗口（秒）：窗口内的重复事件只记一条
pub const DEBOUNCE_SECONDS: u64 = 2;

/// 文件监控运行句柄：drop 或 [`Self::stop`] 即停止监听并结束工作线程
pub struct FileMonitorHandle {
    debouncer: Option<Debouncer<RecommendedWatcher, notify_debouncer_full::FileIdMap>>,
    worker: std::thread::JoinHandle<()>,
}

impl FileMonitorHandle {
    /// 启动监控：`folders` 为递归监听的文件夹列表；`excludes` 为排除规则；
    /// `on_events` 在防抖窗口结束后收到一批待入库事件（按时间升序）。
    /// 任一文件夹监听失败只记日志不阻断（其余文件夹继续工作）。
    pub fn start(
        folders: &[String],
        excludes: &[String],
        on_events: impl Fn(Vec<FileEventRow>) + Send + 'static,
    ) -> Result<Self, String> {
        let (tx, rx) = std::sync::mpsc::channel::<notify_debouncer_full::DebounceEventResult>();
        let mut debouncer = new_debouncer(Duration::from_secs(DEBOUNCE_SECONDS), None, tx)
            .map_err(|e| format!("创建文件监听器失败: {e}"))?;

        let mut watched = 0usize;
        for folder in folders {
            let trimmed = folder.trim();
            if trimmed.is_empty() || !Path::new(trimmed).exists() {
                log::warn!("文件监控跳过无效文件夹: {trimmed:?}");
                continue;
            }
            match debouncer
                .watcher()
                .watch(Path::new(trimmed), RecursiveMode::Recursive)
            {
                Ok(()) => watched += 1,
                Err(e) => log::warn!("文件监控无法监听文件夹 {trimmed:?}: {e}"),
            }
        }
        if watched == 0 {
            return Err("没有任何文件夹被成功监听".to_string());
        }
        log::info!("文件监控已启动，监听 {watched} 个文件夹");

        let excludes = excludes.to_vec();
        let worker = std::thread::spawn(move || {
            while let Ok(batch) = rx.recv() {
                // Err 通道携带 watch 失效/后端错误：静默吞掉会丢事件且零痕迹
                let events = match batch {
                    Ok(events) => events,
                    Err(errs) => {
                        log::warn!(
                            "文件监控事件通道错误（本批事件丢弃）: {} 条, 首条: {:?}",
                            errs.len(),
                            errs.first()
                        );
                        continue;
                    }
                };
                let mut rows: Vec<FileEventRow> = Vec::new();
                // 去重索引：同批内同路径+同类型只保留一条。用 HashSet 而非对 rows
                // 线性扫描：移动含数千文件的目录时单批事件上万，O(n²) 比较会卡住
                // worker 线程（首个命中的语义与原实现一致）
                let mut seen: std::collections::HashSet<(String, String)> =
                    std::collections::HashSet::new();
                for event in events {
                    {
                        let Some(event_type) = map_event_kind(&event.event.kind) else {
                            continue;
                        };
                        for path in event.paths.iter().map(|p| p as &Path) {
                            let path = path.to_string_lossy().to_string();
                            if is_path_excluded(&path, &excludes) {
                                continue;
                            }
                            if !seen.insert((path.clone(), event_type.to_string())) {
                                continue;
                            }
                            let size = if event_type == "deleted" {
                                None
                            } else {
                                std::fs::metadata(path.as_str())
                                    .ok()
                                    .map(|m| m.len() as i64)
                            };
                            let now = chrono::Utc::now();
                            rows.push(FileEventRow {
                                path,
                                event_type: event_type.to_string(),
                                size,
                                occurred_at: now.timestamp(),
                                date: now
                                    .with_timezone(&chrono::Local)
                                    .format("%Y-%m-%d")
                                    .to_string(),
                            });
                        }
                    }
                }
                if !rows.is_empty() {
                    on_events(rows);
                }
            }
        });

        Ok(Self {
            debouncer: Some(debouncer),
            worker,
        })
    }

    /// 停止监听：drop debouncer 关闭事件通道，工作线程随之自然退出
    pub fn stop(mut self) {
        self.debouncer = None;
        let _ = &self.worker;
    }
}

/// notify 事件类型 → 入库事件名；无法分类的事件返回 None（跳过）
pub fn map_event_kind(kind: &notify::EventKind) -> Option<&'static str> {
    use notify::event::{CreateKind, DataChange, EventKind, ModifyKind, RemoveKind};
    match kind {
        EventKind::Create(CreateKind::File | CreateKind::Folder | CreateKind::Other) => {
            Some("created")
        }
        EventKind::Modify(ModifyKind::Data(DataChange::Any)) => Some("modified"),
        EventKind::Modify(ModifyKind::Data(_)) => Some("modified"),
        EventKind::Modify(ModifyKind::Metadata(_)) => Some("modified"),
        EventKind::Modify(ModifyKind::Name(_)) => Some("renamed"),
        EventKind::Modify(ModifyKind::Any) => Some("modified"),
        EventKind::Modify(ModifyKind::Other) => Some("modified"),
        EventKind::Remove(RemoveKind::File | RemoveKind::Folder | RemoveKind::Other) => {
            Some("deleted")
        }
        _ => None,
    }
}

/// 路径是否命中排除规则。规则语义（不区分大小写）：
/// - 含 `*`：对每个路径段做前后缀通配匹配（如 `*.log`、`build*`、`*temp*`）
/// - 不含 `*`：与任一路径段整体相等（如 `.git`、`node_modules`）
pub fn is_path_excluded(path: &str, excludes: &[String]) -> bool {
    let normalized = path.replace('\\', "/").to_lowercase();
    for rule in excludes {
        let rule = rule.trim().to_lowercase();
        if rule.is_empty() {
            continue;
        }
        let hit = normalized.split('/').any(|segment| {
            if rule.contains('*') {
                match_simple_pattern(segment, &rule)
            } else {
                segment == rule
            }
        });
        if hit {
            return true;
        }
    }
    false
}

/// 极简通配：仅支持 `*`（任意长度）前后缀/双星形态，如 `*.log`、`build*`、`*temp*`
fn match_simple_pattern(name: &str, pattern: &str) -> bool {
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        return name == pattern;
    };
    if let Some((p2, _p3)) = suffix.split_once('*') {
        // 两个星号：包含前缀且包含中段（中间任意）
        name.contains(prefix) && name.contains(p2)
    } else {
        name.starts_with(prefix) && name.ends_with(suffix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 排除规则目录段匹配不区分大小写() {
        let excludes = vec![".git".to_string(), "node_modules".to_string()];
        assert!(is_path_excluded(
            "D:/work/project/.git/objects/ab/cd",
            &excludes
        ));
        assert!(is_path_excluded(
            "D:/work/Project/NODE_MODULES/pkg/index.js",
            &excludes
        ));
        assert!(!is_path_excluded("D:/work/project/src/main.rs", &excludes));
        // 子串不算命中（src 不应匹配 srcs）
        assert!(!is_path_excluded(
            "D:/work/project/srcs/a.txt",
            &["src".to_string()]
        ));
    }

    #[test]
    fn 排除规则支持星号通配文件名() {
        let excludes = vec![
            "*.log".to_string(),
            "build*".to_string(),
            "*temp*".to_string(),
        ];
        assert!(is_path_excluded("D:/work/app/debug.log", &excludes));
        assert!(is_path_excluded("D:/work/build_out/x.txt", &excludes));
        assert!(is_path_excluded("D:/work/a/tempfile.txt", &excludes));
        assert!(!is_path_excluded("D:/work/app/main.rs", &excludes));
    }

    #[test]
    fn 排除规则空列表与空条目不影响结果() {
        assert!(!is_path_excluded("D:/a/b.txt", &[]));
        assert!(!is_path_excluded("D:/a/b.txt", &["  ".to_string()]));
    }

    #[test]
    fn 反斜杠路径按段匹配() {
        let excludes = vec![".git".to_string()];
        assert!(is_path_excluded("D:\\work\\.git\\config", &excludes));
    }
}
