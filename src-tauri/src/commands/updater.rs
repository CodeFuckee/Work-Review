//! Auto-extracted from the historical `commands.rs`. Behavior unchanged.

use crate::AppState;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::UpdaterExt;
use work_review_core::error::AppError;

// 更新源已从 GitHub 切换到自建 GitLab：安装包与 updater.json 都在
// Generic Package Registry（匿名不可读，客户端用编译期注入的只读
// Project Access Token 以 PRIVATE-TOKEN 头拉取）。
const GITLAB_API_BASE: &str = "https://home.chenkaidi.top:509/api/v4";
const GITLAB_PROJECT_PATH_ENCODED: &str = "chenkaidi%2Fwork-review";

const GITLAB_LATEST_RELEASE_API: &str =
    "https://home.chenkaidi.top:509/api/v4/projects/chenkaidi%2Fwork-review/releases?per_page=1";

const GITLAB_RELEASE_PAGE: &str =
    "https://home.chenkaidi.top:509/chenkaidi/work-review/-/releases/permalink/latest";

const GITLAB_UPDATE_SOURCE_LABEL: &str = "home.chenkaidi.top:509";

/// 用户可见的更新文案不得暴露自建 GitLab 域名：所有形式（带/不带协议）
/// 统一替换为中性「更新服务器」。事件消息、错误消息、来源标签都经此处理。
fn sanitize_update_text(text: impl std::fmt::Display) -> String {
    let host = GITLAB_UPDATE_SOURCE_LABEL;
    text.to_string()
        .replace(&format!("https://{host}"), "更新服务器")
        .replace(&format!("http://{host}"), "更新服务器")
        .replace(host, "更新服务器")
}

/// 构造用户可见的更新错误：统一经域名脱敏。
fn update_error(message: impl std::fmt::Display) -> AppError {
    AppError::Unknown(sanitize_update_text(message))
}

/// 更新清单所属的 GitLab 项目 ID 与包名（与 publish-release 上传路径一致）。
const GITLAB_PACKAGE_SEGMENT: &str = "packages/generic/yue-review";

const UPDATE_STATUS_EVENT: &str = "update-status";

const DEFAULT_UPDATE_CHECK_INTERVAL_HOURS: u64 = 24;

const UPDATE_REQUEST_TIMEOUT_SECS: u64 = 35;

const UPDATE_CONNECT_TIMEOUT_SECS: u64 = 12;

/// 编译期注入的只读 Project Access Token（CI 变量 UPDATER_GITLAB_TOKEN）。
/// 未注入（如本地开发构建）时更新检查按"未配置更新源"优雅降级。
fn updater_gitlab_token() -> Option<&'static str> {
    option_env!("WORK_REVIEW_UPDATER_GITLAB_TOKEN")
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
}

/// 由目标版本直接构造更新清单地址（Generic Package Registry 固定版本直链，
/// GitLab 19 不支持 `latest` 别名，最新 tag 由 releases API 先行查询）。
fn build_gitlab_manifest_url(expected_version: Option<&str>) -> Option<String> {
    let normalized = expected_version.map(normalize_version).unwrap_or("");
    if normalized.is_empty() {
        return None;
    }
    Some(format!(
        "{GITLAB_API_BASE}/projects/{GITLAB_PROJECT_PATH_ENCODED}/{GITLAB_PACKAGE_SEGMENT}/{normalized}/updater.json"
    ))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GithubUpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub auto_update_ready: bool,
    pub release_url: String,
    pub body: Option<String>,
    pub source: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GithubUpdateInstallResult {
    pub updated: bool,
    pub available: bool,
    pub version: Option<String>,
    pub source: Option<String>,
    pub message: String,
    pub attempted_sources: Vec<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
struct GithubUpdateStatusPayload {
    stage: String,
    message: String,
    source: Option<String>,
    version: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    percent: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSettings {
    pub auto_check: bool,
    pub last_check_time: u64,
    #[serde(default = "default_update_check_interval")]
    pub check_interval_hours: u64,
}

#[derive(Deserialize, Debug)]
struct GitlabReleaseResponse {
    tag_name: String,
    #[serde(default)]
    description: Option<String>,
}

fn default_update_check_interval() -> u64 {
    DEFAULT_UPDATE_CHECK_INTERVAL_HOURS
}

/// 查询自建 GitLab 最新发布（releases 列表默认按发布时间倒序，首个即最新）。
async fn fetch_latest_gitlab_release(
    client: &reqwest::Client,
    token: &str,
) -> Result<GitlabReleaseResponse, AppError> {
    // 错误字符串（reqwest 错误含完整 URL）统一脱敏后再透出
    let releases = client
        .get(GITLAB_LATEST_RELEASE_API)
        .header(reqwest::header::USER_AGENT, "WorkReview-Updater")
        .header("PRIVATE-TOKEN", token)
        .send()
        .await
        .map_err(update_error)?
        .error_for_status()
        .map_err(update_error)?
        .json::<Vec<GitlabReleaseResponse>>()
        .await
        .map_err(update_error)?;

    releases
        .into_iter()
        .next()
        .ok_or_else(|| update_error("更新服务器尚无任何发布".to_string()))
}

/// 用编译期 token 对指定版本的更新清单做可安装性检查（含签名与版本校验）。
async fn check_installable_update(
    app: &AppHandle,
    expected_version: &str,
    token: &str,
) -> Option<GithubUpdateInfo> {
    let manifest_url = build_gitlab_manifest_url(Some(expected_version))?;
    let endpoint_url = match Url::parse(&manifest_url) {
        Ok(url) => url,
        Err(error) => {
            log::warn!("解析更新清单地址失败: {error}");
            return None;
        }
    };

    let updater = match app
        .updater_builder()
        .endpoints(vec![endpoint_url])
        .map(|builder| {
            builder
                .timeout(Duration::from_secs(UPDATE_REQUEST_TIMEOUT_SECS))
                .configure_client(|client| {
                    client
                        .connect_timeout(Duration::from_secs(UPDATE_CONNECT_TIMEOUT_SECS))
                        .user_agent("WorkReview-Updater")
                })
        })
        .and_then(|builder| builder.header("PRIVATE-TOKEN", token))
        .and_then(|builder| builder.build())
    {
        Ok(updater) => updater,
        Err(error) => {
            log::warn!("构建更新器失败: {error}");
            return None;
        }
    };

    match updater.check().await {
        Ok(Some(update)) => Some(GithubUpdateInfo {
            current_version: update.current_version,
            latest_version: update.version,
            available: true,
            auto_update_ready: true,
            release_url: GITLAB_RELEASE_PAGE.to_string(),
            body: update.body,
            source: Some(sanitize_update_text(GITLAB_UPDATE_SOURCE_LABEL)),
        }),
        Ok(None) => None,
        Err(error) => {
            log::warn!("检查可安装更新失败（版本 {expected_version}）: {error}");
            None
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_update_status(
    app: &AppHandle,
    stage: &str,
    message: impl Into<String>,
    source: Option<String>,
    version: Option<String>,
    downloaded_bytes: Option<u64>,
    total_bytes: Option<u64>,
    percent: Option<u64>,
) {
    let _ = app.emit(
        UPDATE_STATUS_EVENT,
        GithubUpdateStatusPayload {
            stage: stage.to_string(),
            message: sanitize_update_text(message.into()),
            source: source.map(sanitize_update_text),
            version,
            downloaded_bytes,
            total_bytes,
            percent,
        },
    );
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            auto_check: true,
            last_check_time: 0,
            check_interval_hours: DEFAULT_UPDATE_CHECK_INTERVAL_HOURS,
        }
    }
}

fn normalize_version(version: &str) -> &str {
    version.trim().trim_start_matches(['v', 'V'])
}

fn parse_version_parts(version: &str) -> Vec<u64> {
    normalize_version(version)
        .split('.')
        .map(|segment| {
            segment
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .unwrap_or(0)
        })
        .collect()
}

fn compare_versions(current: &str, latest: &str) -> Ordering {
    let current_parts = parse_version_parts(current);
    let latest_parts = parse_version_parts(latest);
    let max_len = current_parts.len().max(latest_parts.len());

    for index in 0..max_len {
        let current_value = *current_parts.get(index).unwrap_or(&0);
        let latest_value = *latest_parts.get(index).unwrap_or(&0);
        match current_value.cmp(&latest_value) {
            Ordering::Equal => continue,
            other => return other,
        }
    }

    Ordering::Equal
}

fn update_settings_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("update_settings.json")
}

fn load_update_settings_from_dir(data_dir: &Path) -> Result<UpdateSettings, AppError> {
    let settings_path = update_settings_path(data_dir);

    if !settings_path.exists() {
        return Ok(UpdateSettings::default());
    }

    let content = std::fs::read_to_string(&settings_path)
        .map_err(|e| update_error(format!("读取更新设置失败: {e}")))?;

    serde_json::from_str(&content).map_err(|e| update_error(format!("解析更新设置失败: {e}")))
}

fn save_update_settings_to_dir(data_dir: &Path, settings: &UpdateSettings) -> Result<(), AppError> {
    let settings_path = update_settings_path(data_dir);
    let content = serde_json::to_string_pretty(settings)
        .map_err(|e| update_error(format!("序列化更新设置失败: {e}")))?;

    std::fs::write(&settings_path, content)
        .map_err(|e| update_error(format!("保存更新设置失败: {e}")))?;

    Ok(())
}

fn should_check_for_updates(settings: &UpdateSettings) -> bool {
    if !settings.auto_check {
        return false;
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let interval_hours = if settings.check_interval_hours > 0 {
        settings.check_interval_hours
    } else {
        DEFAULT_UPDATE_CHECK_INTERVAL_HOURS
    };
    let elapsed_hours = now.saturating_sub(settings.last_check_time) / 3600;

    elapsed_hours >= interval_hours
}

/// 获取更新检查设置
#[tauri::command]
pub async fn get_update_settings(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<UpdateSettings, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| update_error(e.to_string()))?;
        state.data_dir.clone()
    };

    load_update_settings_from_dir(&data_dir)
}

/// 保存更新检查设置
#[tauri::command]
pub async fn save_update_settings(
    settings: UpdateSettings,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| update_error(e.to_string()))?;
        state.data_dir.clone()
    };

    save_update_settings_to_dir(&data_dir, &settings)
}

/// 判断当前是否应自动检查更新
#[tauri::command]
pub async fn should_check_updates(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| update_error(e.to_string()))?;
        state.data_dir.clone()
    };
    let settings = load_update_settings_from_dir(&data_dir)?;

    Ok(should_check_for_updates(&settings))
}

/// 更新时间检查时间戳
#[tauri::command]
pub async fn update_last_check_time(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let data_dir = {
        let state = state.lock().map_err(|e| update_error(e.to_string()))?;
        state.data_dir.clone()
    };
    let mut settings = load_update_settings_from_dir(&data_dir)?;
    settings.last_check_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    save_update_settings_to_dir(&data_dir, &settings)
}

#[tauri::command]
pub async fn quit_app_for_update(app: AppHandle) -> Result<(), AppError> {
    app.exit(0);
    Ok(())
}

/// 基于自建 GitLab 检查更新：先查最新发布 tag 做版本比较，再对更新清单
/// （包仓库固定版本直链）做可安装性检查；清单不可用时回退为提示手动下载。
#[tauri::command]
pub async fn check_github_update(app: AppHandle) -> Result<GithubUpdateInfo, AppError> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let Some(token) = updater_gitlab_token() else {
        log::warn!("未注入 WORK_REVIEW_UPDATER_GITLAB_TOKEN，按未配置更新源处理");
        return Ok(GithubUpdateInfo {
            current_version: current_version.clone(),
            latest_version: current_version,
            available: false,
            auto_update_ready: false,
            release_url: GITLAB_RELEASE_PAGE.to_string(),
            body: None,
            source: Some("gitlab-unconfigured".to_string()),
        });
    };

    let client = reqwest::Client::builder()
        .user_agent("WorkReview-Updater")
        .timeout(Duration::from_secs(UPDATE_REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(UPDATE_CONNECT_TIMEOUT_SECS))
        .build()
        .map_err(|e| update_error(format!("创建更新检查客户端失败: {e}")))?;

    let release = fetch_latest_gitlab_release(&client, token).await?;
    let latest_version = normalize_version(&release.tag_name).to_string();
    let has_update = compare_versions(&current_version, &latest_version) == Ordering::Less;

    if !has_update {
        return Ok(GithubUpdateInfo {
            current_version,
            latest_version,
            available: false,
            auto_update_ready: true,
            release_url: GITLAB_RELEASE_PAGE.to_string(),
            body: release.description,
            source: Some(sanitize_update_text(GITLAB_UPDATE_SOURCE_LABEL)),
        });
    }

    let auto_update_ready = check_installable_update(&app, &latest_version, token)
        .await
        .is_some();

    Ok(GithubUpdateInfo {
        current_version,
        latest_version,
        available: true,
        auto_update_ready,
        release_url: GITLAB_RELEASE_PAGE.to_string(),
        body: release.description,
        source: Some(sanitize_update_text(GITLAB_UPDATE_SOURCE_LABEL)),
    })
}

/// 对指定版本执行在线更新：从自建 GitLab 包仓库拉取更新清单并下载安装，
/// 全程携带 PRIVATE-TOKEN 认证头，进度通过 update-status 事件上报。
#[tauri::command]
pub async fn download_and_install_github_update(
    app: AppHandle,
    expected_version: Option<String>,
) -> Result<GithubUpdateInstallResult, AppError> {
    let Some(token) = updater_gitlab_token() else {
        return Err(update_error(
            "未配置更新令牌（WORK_REVIEW_UPDATER_GITLAB_TOKEN），无法在线更新".to_string(),
        ));
    };

    let mut attempted_sources = Vec::new();
    let mut failures = Vec::new();

    // 目标版本优先取调用方传入（与检查阶段一致），缺省时现查最新发布。
    let target_version = match expected_version.as_deref() {
        Some(raw) if !normalize_version(raw).is_empty() => normalize_version(raw).to_string(),
        _ => {
            let client = reqwest::Client::builder()
                .user_agent("WorkReview-Updater")
                .timeout(Duration::from_secs(UPDATE_REQUEST_TIMEOUT_SECS))
                .connect_timeout(Duration::from_secs(UPDATE_CONNECT_TIMEOUT_SECS))
                .build()
                .map_err(|e| update_error(format!("创建更新客户端失败: {e}")))?;
            normalize_version(&fetch_latest_gitlab_release(&client, token).await?.tag_name)
                .to_string()
        }
    };

    let Some(manifest_url) = build_gitlab_manifest_url(Some(&target_version)) else {
        return Err(update_error(format!(
            "无法为版本 {target_version} 构造更新清单地址"
        )));
    };

    let source_label = GITLAB_UPDATE_SOURCE_LABEL.to_string();
    attempted_sources.push(source_label.clone());

    emit_update_status(
        &app,
        "checking",
        format!("正在检查更新源 {source_label}..."),
        Some(source_label.clone()),
        Some(target_version.clone()),
        None,
        None,
        None,
    );

    let endpoint_url = Url::parse(&manifest_url)
        .map_err(|e| update_error(format!("解析更新源失败 ({manifest_url}): {e}")))?;

    let updater = match app
        .updater_builder()
        .endpoints(vec![endpoint_url])
        .map(|builder| {
            builder
                .timeout(Duration::from_secs(UPDATE_REQUEST_TIMEOUT_SECS))
                .configure_client(|client| {
                    client
                        .connect_timeout(Duration::from_secs(UPDATE_CONNECT_TIMEOUT_SECS))
                        .user_agent("WorkReview-Updater")
                })
        })
        .and_then(|builder| builder.header("PRIVATE-TOKEN", token))
        .and_then(|builder| builder.build())
    {
        Ok(updater) => updater,
        Err(error) => {
            return Err(update_error(format!(
                "{source_label}: 构建更新器失败: {error}"
            )));
        }
    };

    let update = match updater.check().await {
        Ok(Some(found)) => found,
        Ok(None) => {
            let message =
                format!("在线更新失败，已尝试全部更新源：{source_label}: 未返回可安装的更新包");
            emit_update_status(
                &app,
                "failed",
                message.clone(),
                None,
                Some(target_version.clone()),
                None,
                None,
                None,
            );
            return Err(update_error(message));
        }
        Err(error) => {
            let message =
                format!("在线更新失败，已尝试全部更新源：{source_label}: 检查更新失败: {error}");
            emit_update_status(
                &app,
                "failed",
                message.clone(),
                None,
                Some(target_version.clone()),
                None,
                None,
                None,
            );
            return Err(update_error(message));
        }
    };

    if compare_versions(&update.version, &target_version) == Ordering::Less {
        let message = format!(
            "在线更新失败，已尝试全部更新源：{source_label}: 返回版本 {}，低于目标版本 {target_version}",
            update.version
        );
        emit_update_status(
            &app,
            "failed",
            message.clone(),
            None,
            Some(target_version.clone()),
            None,
            None,
            None,
        );
        return Err(update_error(message));
    }

    emit_update_status(
        &app,
        "found",
        format!(
            "发现新版本 {}，准备从 {source_label} 下载...",
            update.version
        ),
        Some(source_label.clone()),
        Some(update.version.clone()),
        None,
        None,
        None,
    );

    let progress_app = app.clone();
    let progress_source = source_label.clone();
    let progress_version = update.version.clone();
    let mut downloaded_bytes = 0_u64;

    let finish_app = app.clone();
    let finish_source = source_label.clone();
    let finish_version = update.version.clone();

    match update
        .download_and_install(
            move |chunk_length, total_bytes| {
                downloaded_bytes += chunk_length as u64;
                let percent = total_bytes.and_then(|total| {
                    downloaded_bytes
                        .saturating_mul(100)
                        .checked_div(total)
                        .map(|percent| percent.min(100))
                });

                let message = if let Some(percent) = percent {
                    format!("正在下载更新 {percent}%（{progress_source}）")
                } else {
                    let mb = ((downloaded_bytes as f64) / 1024.0 / 1024.0).max(0.1);
                    format!("正在下载更新 {mb:.1} MB（{progress_source}）")
                };

                emit_update_status(
                    &progress_app,
                    "downloading",
                    message,
                    Some(progress_source.clone()),
                    Some(progress_version.clone()),
                    Some(downloaded_bytes),
                    total_bytes,
                    percent,
                );
            },
            move || {
                emit_update_status(
                    &finish_app,
                    "installing",
                    format!("下载完成，正在安装（{finish_source}）..."),
                    Some(finish_source.clone()),
                    Some(finish_version.clone()),
                    None,
                    None,
                    Some(100),
                );
            },
        )
        .await
    {
        Ok(()) => {
            emit_update_status(
                &app,
                "completed",
                format!("更新安装完成，来源 {source_label}"),
                Some(source_label.clone()),
                Some(update.version.clone()),
                None,
                None,
                Some(100),
            );

            return Ok(GithubUpdateInstallResult {
                updated: true,
                available: true,
                version: Some(update.version),
                source: Some(sanitize_update_text(&source_label)),
                message: "在线更新已完成".to_string(),
                attempted_sources: attempted_sources
                    .iter()
                    .map(sanitize_update_text)
                    .collect(),
            });
        }
        Err(error) => {
            failures.push(format!("{source_label}: 下载或安装失败: {error}"));
            emit_update_status(
                &app,
                "retrying",
                format!("源 {source_label} 更新失败，准备尝试下一个源..."),
                Some(source_label),
                Some(update.version.clone()),
                None,
                None,
                None,
            );
        }
    }

    let message = if failures.is_empty() {
        format!("未找到可用于版本 {target_version} 的在线更新源")
    } else {
        format!("在线更新失败，已尝试全部更新源：{}", failures.join("；"))
    };

    emit_update_status(
        &app,
        "failed",
        message.clone(),
        None,
        Some(target_version.clone()),
        None,
        None,
        None,
    );

    Err(update_error(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn 更新文案脱敏应替换所有形式的自建域名() {
        let sanitized = sanitize_update_text(
            "检查 https://home.chenkaidi.top:509/api/v4/x 失败（home.chenkaidi.top:509）",
        );
        assert_eq!(
            sanitized, "检查 更新服务器/api/v4/x 失败（更新服务器）",
            "带协议与裸域名都应替换"
        );
        assert_eq!(
            sanitize_update_text("http://home.chenkaidi.top:509/y"),
            "更新服务器/y",
            "http 协议同样替换"
        );
        assert_eq!(
            sanitize_update_text(GITLAB_UPDATE_SOURCE_LABEL),
            "更新服务器",
            "更新源标签本身即域名，应被替换"
        );
        assert_eq!(
            sanitize_update_text("无需脱敏的普通文案"),
            "无需脱敏的普通文案",
            "不含域名的文案应原样保留"
        );
    }

    #[test]
    fn 更新清单地址应由目标版本构造且去除v前缀() {
        assert_eq!(
            build_gitlab_manifest_url(Some("v1.1.35")).as_deref(),
            Some(
                "https://home.chenkaidi.top:509/api/v4/projects/chenkaidi%2Fwork-review/packages/generic/yue-review/1.1.35/updater.json"
            )
        );
        assert_eq!(
            build_gitlab_manifest_url(Some(" 1.1.35 ")).as_deref(),
            Some(
                "https://home.chenkaidi.top:509/api/v4/projects/chenkaidi%2Fwork-review/packages/generic/yue-review/1.1.35/updater.json"
            )
        );
    }

    #[test]
    fn 更新清单地址缺少有效版本时应返回空() {
        assert_eq!(build_gitlab_manifest_url(None), None);
        assert_eq!(build_gitlab_manifest_url(Some("")), None);
        assert_eq!(build_gitlab_manifest_url(Some("   ")), None);
        assert_eq!(build_gitlab_manifest_url(Some("vvv")), None);
    }

    #[test]
    fn 最新版本查询应指向自建gitlab的releases接口() {
        assert_eq!(
            GITLAB_LATEST_RELEASE_API,
            "https://home.chenkaidi.top:509/api/v4/projects/chenkaidi%2Fwork-review/releases?per_page=1"
        );
    }

    #[test]
    fn 发布页应指向自建gitlab最新发布永久链接() {
        assert_eq!(
            GITLAB_RELEASE_PAGE,
            "https://home.chenkaidi.top:509/chenkaidi/work-review/-/releases/permalink/latest"
        );
    }

    #[test]
    fn 更新相关地址不应再包含github() {
        for raw in [
            GITLAB_LATEST_RELEASE_API,
            GITLAB_RELEASE_PAGE,
            build_gitlab_manifest_url(Some("v9.9.9"))
                .unwrap_or_default()
                .as_str(),
        ] {
            assert!(
                !raw.to_ascii_lowercase().contains("github"),
                "更新地址不应包含 github: {raw}"
            );
        }
    }

    #[test]
    fn 版本比较应处理v前缀与进位边界() {
        assert_eq!(compare_versions("1.1.9", "1.1.10"), Ordering::Less);
        assert_eq!(compare_versions("v1.1.34", "1.1.34"), Ordering::Equal);
        assert_eq!(compare_versions("1.2", "1.1.99"), Ordering::Greater);
        assert_eq!(compare_versions("1.1", "1.1.0"), Ordering::Equal);
    }

    #[test]
    fn 更新令牌缺失时应能优雅降级() {
        // updater_gitlab_token 是 option_env! 的包装：本地/CI 未注入该变量时必须
        // 返回 None（调用方按"未配置更新源"降级），而不是 panic 或空串误判为已配置。
        let token = updater_gitlab_token();
        if let Some(raw) = token {
            assert!(!raw.trim().is_empty());
        }
    }
}
