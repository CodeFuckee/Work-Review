//! 本地文件监控命令：概览页「文件活动」数据与运行状态查询。

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::State;
use work_review_core::database::FileEventRow;

use crate::AppState;

/// 概览页文件活动统计（区间总数 + 最近事件列表）
#[derive(Debug, Clone, Serialize)]
pub struct FileMonitorStats {
    /// 区间内事件总数
    pub total: i64,
    /// 最近事件（按发生时间倒序，最多 limit 条）
    pub events: Vec<FileEventRow>,
}

/// 文件监控运行状态（设置页展示）
#[derive(Debug, Clone, Serialize)]
pub struct FileMonitorStatus {
    /// 监听器是否在运行（已启用且至少一个文件夹监听成功）
    pub running: bool,
    /// 配置是否启用
    pub enabled: bool,
    /// 配置的监听文件夹
    pub folders: Vec<String>,
}

/// 概览页「文件活动」：区间内事件总数 + 最近事件列表（默认 20 条）
#[tauri::command]
pub fn get_file_monitor_stats(
    date_from: String,
    date_to: String,
    limit: Option<i64>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<FileMonitorStats, String> {
    let limit = limit.unwrap_or(20).clamp(1, 200);
    let state = state.lock().map_err(|e| e.to_string())?;
    let events = state
        .database
        .get_recent_file_events(&date_from, &date_to, limit)
        .map_err(|e| e.to_string())?;
    let total = state
        .database
        .count_file_events(&date_from, &date_to)
        .map_err(|e| e.to_string())?;
    Ok(FileMonitorStats { total, events })
}

/// 文件监控运行状态（设置页展示）
#[tauri::command]
pub fn file_monitor_status(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<FileMonitorStatus, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(FileMonitorStatus {
        running: state.file_monitor.is_some(),
        enabled: state.config.file_monitor.enabled,
        folders: state.config.file_monitor.folders.clone(),
    })
}
