//! 本地文件监控运行时：管理 notify watcher 生命周期并把事件写入数据库。
//!
//! 调用时机：应用启动（配置已启用）与设置保存（`persist_app_config`）。
//! 注意必须在已持有 AppState 锁的上下文里调用 `apply_config_locked`，
//! 事件回调线程稍后再短暂加锁入库——无重入，无死锁环。

use std::sync::{Arc, Mutex};

use work_review_core::config::FileMonitorConfig;
pub use work_review_core::file_monitor::FileMonitorHandle;

use crate::AppState;

/// 按配置启动/停止文件监控。返回是否处于运行状态。
/// 必须在已持有 state 锁的上下文调用（guard 传入，避免重入死锁）。
pub fn apply_config_locked(
    state: &mut AppState,
    state_arc: &Arc<Mutex<AppState>>,
    config: &FileMonitorConfig,
) -> bool {
    // 先停掉旧实例（drop debouncer 停止监听）
    if let Some(old) = state.file_monitor.take() {
        old.stop();
    }

    let should_run = config.enabled && !config.folders.is_empty();
    if !should_run {
        return false;
    }

    let state_for_events = state_arc.clone();
    match FileMonitorHandle::start(&config.folders, &config.excludes, move |rows| {
        // 锁中毒也恢复而非静默丢事件（与全仓 unwrap_or_else(into_inner) 口径一致）
        let state = state_for_events.lock().unwrap_or_else(|e| e.into_inner());
        let count = rows.len();
        match state.database.insert_file_events(rows.as_slice()) {
            Ok(_) => log::info!("文件监控写入 {count} 条事件"),
            Err(e) => log::warn!("文件监控事件入库失败: {e}"),
        }
    }) {
        Ok(handle) => {
            state.file_monitor = Some(handle);
            log::info!("文件监控已启用，监听 {} 个文件夹", config.folders.len());
            true
        }
        Err(e) => {
            log::warn!("文件监控启动失败: {e}");
            false
        }
    }
}
