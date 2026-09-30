//! Plane 集成命令（设置页「测试连接」/「工作项预览」与时间线页工作项同步）。
//!
//! 时间线/日报联动（二期）：时间线页按日补采入库（`plane_sync_work_items_for_date`），
//! 日报小节直接读库构建（不现拉 API）。端点与响应字段依据官方文档
//! <https://developers.plane.so>（api-reference）：
//! - `GET /api/v1/workspaces/{workspace_slug}/projects/`（projects:read，
//!   分页信封 `results`，项目对象含 `id` / `name`）；
//! - `GET /api/v1/workspaces/{workspace_slug}/projects/{project_id}/work-items/`
//!   （`order_by` 支持 `-` 前缀倒序、`per_page` 每页条数、`expand=state` 把状态
//!   UUID 展开为 `{id, name, group}` 对象——文档示例响应即此形态）；
//! - 鉴权：请求头 `X-API-Key: <Personal Access Token>`。
//!
//! 工作区级「近期 issues」列表端点在 api/v1 无文档（仅有必须携带搜索词的
//! search 端点），预览/同步按「列项目 → 聚合各项目近期工作项」实现。

use crate::AppState;
use chrono::TimeZone as _;
use serde::Serialize;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::State;
use work_review_core::analysis::AppLocale;
use work_review_core::database::PlaneWorkItemRow;
use work_review_core::plane_match::{
    PlaneAttribution, PlaneAttributionSource, PlaneCandidate, AI_FALLBACK_SCORE,
};

/// 连接测试超时（秒）
const TEST_CONNECTION_TIMEOUT_SECS: u64 = 8;
/// 预览拉取总超时（秒）：1 次项目列表 + 最多 MAX_PREVIEW_PROJECTS 次工作项请求
const PREVIEW_TIMEOUT_SECS: u64 = 20;
/// 项目列表单页条数（只取第一页，避免多页翻页失控）
const PROJECT_PAGE_SIZE: u32 = 100;
/// 预览聚合的项目数上限（取项目列表前 N 个）
const MAX_PREVIEW_PROJECTS: usize = 100;
/// 每个项目最多拉取的工作项数（按 updated_at 倒序取前 N；预览口径）
const PER_PROJECT_WORK_ITEM_LIMIT: u32 = 10;
/// 同步时每个项目拉取的工作项数（按 updated_at 倒序取前 N；100 条足以覆盖
/// 单项目单日的更新量，更旧的条目落在目标日期窗口外，本就会被过滤）
const PER_PROJECT_SYNC_LIMIT: u32 = 100;
/// 同步拉取总预算（秒）：1 次项目列表 + 逐项目工作项请求（同一期预览口径）
const SYNC_TIMEOUT_SECS: u64 = 20;
/// 归因候选拉取总预算（秒，与同步口径一致）
const ATTRIBUTION_FETCH_TIMEOUT_SECS: u64 = 20;
/// 归因候选内存缓存 TTL：时间线徽标与日报小节同日各算一次时避免重复全量拉取
const CANDIDATE_CACHE_TTL_SECS: u64 = 600;
/// 归因时每日活动读取上限（get_activities_in_range 内部钳到 10_000）
const ATTRIBUTION_ACTIVITY_LIMIT: usize = 10_000;
/// 预览条数默认值
pub const DEFAULT_PREVIEW_LIMIT: u32 = 20;

/// Plane 连接测试结果（设置页展示）
#[derive(Debug, Clone, Serialize)]
pub struct PlaneConnectionTestResult {
    pub ok: bool,
    pub workspace_name: String,
    pub message: Option<String>,
}

/// 工作项预览条目（id / 名称 / 项目名 / 状态 / 更新时间）
#[derive(Debug, Clone, Serialize)]
pub struct PlanePreviewItem {
    pub id: String,
    pub name: String,
    pub project: String,
    pub state: String,
    pub updated_at: String,
    /// 工作项在项目内的序号（「ABC-123」中的 123；缺失/非法时为 0）
    pub sequence_id: i64,
}

/// 工作项预览拉取结果
#[derive(Debug, Clone, Serialize)]
pub struct PlanePreviewResult {
    pub ok: bool,
    pub items: Vec<PlanePreviewItem>,
    pub message: Option<String>,
}

/// Plane 调用错误（已本地化/可展示）
#[derive(Debug)]
enum PlaneError {
    /// 网络/传输错误
    Http(reqwest::Error),
    /// 非 2xx：HTTP 状态码 + 响应体片段
    Api(u16, String),
    /// JSON 解析失败
    Json(serde_json::Error),
    /// 2xx 但响应体不是 JSON（典型：命中了 Web 前端的 SPA 兜底页——
    /// 实例地址填成了 Web 界面子路径，而 API 在根路径或另一前缀下）
    NonJson(u16, String),
}

/// 错误串剔除 api_key：密钥只应出现在请求头，正常不会进入错误消息，
/// 这里兜底保证任何路径（含服务端回显）都不会把密钥展示到 UI。
fn redact_api_key(message: &str, api_key: &str) -> String {
    let key = api_key.trim();
    if key.is_empty() {
        return message.to_string();
    }
    message.replace(key, "***")
}

impl PlaneError {
    /// 转为可展示消息（必须携带 api_key 以剔除潜在回显）
    fn message(&self, api_key: &str) -> String {
        match self {
            PlaneError::Http(e) => {
                format!("网络请求失败: {}", redact_api_key(&e.to_string(), api_key))
            }
            PlaneError::Api(status, body) => {
                let snippet = redact_api_key(body, api_key)
                    .chars()
                    .take(120)
                    .collect::<String>();
                match *status {
                    401 => format!("API Key 无效或已过期（Plane API 返回 {status}）"),
                    404 => format!("实例地址或 workspace slug 不正确（Plane API 返回 {status}）"),
                    _ => format!("Plane API 返回 {status}: {snippet}"),
                }
            }
            PlaneError::Json(e) => {
                format!(
                    "解析 Plane 响应失败: {}",
                    redact_api_key(&e.to_string(), api_key)
                )
            }
            PlaneError::NonJson(status, snippet) => {
                format!(
                    "实例返回了非 JSON 内容（HTTP {status}）——通常是把 Web 界面地址填进了实例地址（Web 子路径不是 API 根路径），请去掉界面路径前缀后重试: {snippet}",
                    snippet = redact_api_key(snippet, api_key)
                )
            }
        }
    }
}

/// Plane REST 客户端（api/v1）。
///
/// 薄封装：只负责 URL 拼接、鉴权头与 JSON 反序列化，聚合/排序判断全部在
/// 独立函数中（可单测）。Clone 供逐项目拉取时分发。
#[derive(Clone)]
struct PlaneClient {
    base_url: String,
    workspace_slug: String,
    api_key: String,
    http: reqwest::Client,
}

impl PlaneClient {
    fn new(base_url: &str, workspace_slug: &str, api_key: &str) -> Self {
        // 必须显式设超时：reqwest 默认无超时，实例挂起时命令会先走外层 tokio
        // timeout 之外还持续占用连接。禁重定向：自部署地址静默跟随重定向会把
        // 带密钥的请求发往别处，遇到重定向型配置错误宁可显式失败。
        // 构建失败（TLS 后端初始化失败）直接 panic 暴露环境问题（同 GitlabClient）。
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("reqwest Client 初始化失败（TLS 后端不可用）");
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            workspace_slug: workspace_slug.trim().to_string(),
            api_key: api_key.trim().to_string(),
            http,
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!(
            "{}/api/v1/workspaces/{}{}",
            self.base_url, self.workspace_slug, path
        )
    }

    fn projects_url(&self, per_page: u32) -> String {
        self.api_url(&format!("/projects/?per_page={per_page}"))
    }

    async fn get_json(&self, url: &str) -> Result<serde_json::Value, PlaneError> {
        let resp = self
            .http
            .get(url)
            .header("X-API-Key", &self.api_key)
            .send()
            .await
            .map_err(PlaneError::Http)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(PlaneError::Http)?;
        if !status.is_success() {
            return Err(PlaneError::Api(
                status.as_u16(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(value) => Ok(value),
            Err(error) => {
                let head: String = String::from_utf8_lossy(&bytes).chars().take(120).collect();
                let looks_like_html = head.trim_start().starts_with('<');
                if looks_like_html {
                    Err(PlaneError::NonJson(status.as_u16(), head))
                } else {
                    Err(PlaneError::Json(error))
                }
            }
        }
    }

    /// 连接测试：`GET /api/v1/workspaces/{slug}/projects/?per_page=1`
    /// （文档明确列出的端点；实测 2xx JSON 即连通，Key 对该工作区有效）。
    /// 不再优先打旧版工作区详情端点——现行 API 已移除，且部分实例该路由
    /// 仅接受会话认证（X-API-Key 会得到 401），易被误判为 Key 无效。
    /// 工作区名回退为 slug（可后续从项目列表的 workspace 字段富化）。
    async fn test_connection(&self) -> Result<String, PlaneError> {
        self.get_json(&self.projects_url(1)).await?;
        Ok(self.workspace_slug.clone())
    }

    /// 列项目（第一页）：返回 (project_id, 显示名, identifier)；名称为空回退
    /// identifier。identifier 是工作项展示 key（`{identifier}-{sequence_id}`，
    /// 如 ABC-123）的组成项，归因链路必须保留。
    async fn list_projects(&self) -> Result<Vec<(String, String, String)>, PlaneError> {
        let projects_value = self.get_json(&self.projects_url(PROJECT_PAGE_SIZE)).await?;
        Ok(parse_results(&projects_value)
            .iter()
            .map(|project| {
                let identifier = json_str(project, "identifier");
                let name = {
                    let name = json_str(project, "name");
                    if name.is_empty() {
                        identifier.clone()
                    } else {
                        name
                    }
                };
                (json_str(project, "id"), name, identifier)
            })
            .filter(|(id, _, _)| !id.is_empty())
            .collect())
    }

    /// 逐项目拉取近期工作项（`order_by=-updated_at` + `expand=state`）并聚合。
    /// 单个项目失败仅跳过（log::warn，与 GitLab 的部分失败策略一致）。
    async fn fetch_work_items(
        &self,
        selected: &[(String, String, String)],
        per_project_limit: u32,
    ) -> Result<Vec<PlanePreviewItem>, PlaneError> {
        let mut items: Vec<PlanePreviewItem> = Vec::new();
        for (project_id, project_name, _identifier) in selected {
            let url = self.api_url(&format!(
                "/projects/{project_id}/work-items/?per_page={per_project_limit}&order_by=-updated_at&expand=state"
            ));
            match self.get_json(&url).await {
                Ok(value) => {
                    for work_item in parse_results(&value) {
                        items.push(parse_work_item(work_item, project_name));
                    }
                }
                Err(error) => {
                    // 部分失败不能静默：否则该项目工作项无声丢失、无从排查
                    log::warn!(
                        "拉取 Plane 项目 {project_name} 工作项失败，已跳过: {}",
                        error.message(&self.api_key)
                    );
                }
            }
        }
        Ok(items)
    }

    /// 预览工作项：取项目列表前 `MAX_PREVIEW_PROJECTS` 个项目，逐个拉近期
    /// 工作项后全局按 updated_at 倒序截 limit。不读配置、不做项目过滤
    /// （设置页可在未保存前先预览，过滤口径属于保存后的同步/日报链路）。
    async fn fetch_preview(&self, limit: usize) -> Result<Vec<PlanePreviewItem>, PlaneError> {
        let projects = self.list_projects().await?;
        let selected: Vec<(String, String, String)> =
            projects.into_iter().take(MAX_PREVIEW_PROJECTS).collect();
        let mut items = self
            .fetch_work_items(&selected, PER_PROJECT_WORK_ITEM_LIMIT)
            .await?;
        items.sort_by_key(|item| std::cmp::Reverse(updated_at_sort_key(item)));
        items.truncate(limit);
        Ok(items)
    }

    /// 同步拉取：项目按允许列表过滤（空 = 全部，匹配语义见
    /// `project_matches_allowlist`）后聚合近期工作项。不排序不截断：
    /// 日期窗口过滤由命令层按本地时区完成。
    async fn fetch_work_items_for_sync(
        &self,
        project_filter: &[String],
    ) -> Result<Vec<PlanePreviewItem>, PlaneError> {
        let projects = self.list_projects().await?;
        let selected: Vec<(String, String, String)> = projects
            .into_iter()
            .filter(|(_, name, _)| project_matches_allowlist(name, project_filter))
            .collect();
        self.fetch_work_items(&selected, PER_PROJECT_SYNC_LIMIT)
            .await
    }

    /// 归因候选拉取（时间线徽标 + 日报小节共用输入）：项目按允许列表过滤后，
    /// 逐项目取第一页 100 条近期工作项（`order_by=-updated_at&expand=state`，
    /// 候选只需覆盖「当天活跃的工作项」）。单个项目失败仅跳过（与同步一致）；
    /// 总预算 20 秒由调用方 `fetch_candidates_cached` 施加。
    async fn fetch_candidates(
        &self,
        project_filter: &[String],
    ) -> Result<Vec<PlaneCandidate>, PlaneError> {
        let projects = self.list_projects().await?;
        let selected: Vec<(String, String, String)> = projects
            .into_iter()
            .filter(|(_, name, _)| project_matches_allowlist(name, project_filter))
            .collect();
        let mut candidates: Vec<PlaneCandidate> = Vec::new();
        for (project_id, project_name, identifier) in &selected {
            let url = self.api_url(&format!(
                "/projects/{project_id}/work-items/?per_page={PER_PROJECT_SYNC_LIMIT}&order_by=-updated_at&expand=state"
            ));
            match self.get_json(&url).await {
                Ok(value) => {
                    for work_item in parse_results(&value) {
                        candidates.push(parse_candidate(work_item, project_name, identifier));
                    }
                }
                Err(error) => {
                    // 部分失败不能静默：该项目候选无声丢失会直接劣化归因命中率
                    log::warn!(
                        "拉取 Plane 项目 {project_name} 工作项候选失败，已跳过: {}",
                        error.message(&self.api_key)
                    );
                }
            }
        }
        Ok(candidates)
    }
}

/// Plane 分页信封取 `results` 数组；旧版个别端点可能直接返回数组，两种形态都兼容。
fn parse_results(value: &serde_json::Value) -> Vec<&serde_json::Value> {
    match value.get("results") {
        Some(results) if results.is_array() => results.as_array().unwrap().iter().collect(),
        _ if value.is_array() => value.as_array().unwrap().iter().collect(),
        _ => Vec::new(),
    }
}

fn json_str(value: &serde_json::Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 解析单个工作项：id / 名称 / 项目名 / 状态名 / updated_at（缺 updated_at 时
/// 回退 created_at，文档示例两字段均为 ISO8601 字符串；`expand=state` 时
/// state 为含 `name` 的对象，未展开时是 UUID——后者状态名留空）。
/// `sequence_id` 是「ABC-123」中的数字序号，归因链路依赖它拼展示 key。
fn parse_work_item(value: &serde_json::Value, project_name: &str) -> PlanePreviewItem {
    let updated_at = {
        let updated = json_str(value, "updated_at");
        if updated.is_empty() {
            json_str(value, "created_at")
        } else {
            updated
        }
    };
    PlanePreviewItem {
        id: json_str(value, "id"),
        name: json_str(value, "name"),
        project: project_name.to_string(),
        state: value
            .get("state")
            .and_then(|state| state.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at,
        sequence_id: value
            .get("sequence_id")
            .and_then(|v| v.as_i64())
            .unwrap_or(0),
    }
}

/// 工作项 → 归因候选（附带项目 identifier 以拼 `{identifier}-{sequence_id}` key；
/// updated_at 毫秒转 UTC unix 秒，与 PlaneWorkItemRow 口径一致；文本字段剥离
/// 控制字符防止破坏日报 markdown 的单行结构）。
fn parse_candidate(
    value: &serde_json::Value,
    project_name: &str,
    project_identifier: &str,
) -> PlaneCandidate {
    let preview = parse_work_item(value, project_name);
    PlaneCandidate {
        plane_item_id: sanitize_single_line(&preview.id),
        sequence_id: preview.sequence_id,
        project_identifier: sanitize_single_line(project_identifier),
        project_name: sanitize_single_line(project_name),
        name: sanitize_single_line(&preview.name),
        state_name: sanitize_single_line(&preview.state),
        updated_at: updated_at_sort_key(&preview) / 1000,
    }
}

/// 从实例地址提取 host（小写），供归因引擎识别「浏览器正开着 Plane 工作项页」。
/// 解析失败返回 None（此时跳过 URL 锁定信号，不影响其余信号）。仅取 `//` 后、
/// 路径分隔符前的 authority 并去掉端口；userinfo(`user@host`) 与 IPv6 字面量
/// 属边缘部署形态，直接按普通 host 串尽力解析。
fn endpoint_host(base_url: &str) -> Option<String> {
    let trimmed = base_url.trim().to_lowercase();
    let without_scheme = trimmed
        .strip_prefix("https://")
        .or_else(|| trimmed.strip_prefix("http://"))
        .unwrap_or(&trimmed);
    let authority = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_end_matches('@');
    let host = authority.rsplit('@').next().unwrap_or(authority);
    let host = host.split(':').next().unwrap_or("");
    let host = host.trim().to_lowercase();
    if host.is_empty() {
        None
    } else {
        Some(host)
    }
}

/// 排序键：updated_at（ISO8601）解析为毫秒时间戳；解析失败的条目排最后。
fn updated_at_sort_key(item: &PlanePreviewItem) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&item.updated_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(i64::MIN)
}

/// 项目允许列表匹配（PlaneConfig.project_filter）：空列表 = 全部项目；
/// 忽略大小写的**精确相等**匹配（用户裁决 2026-09-24：不做前缀命中，
/// 避免「API」误纳「API-docs」）。空白条目直接忽略；项目名为空不命中。
fn project_matches_allowlist(project_name: &str, allowlist: &[String]) -> bool {
    if allowlist.is_empty() {
        return true;
    }
    let name = project_name.trim().to_lowercase();
    if name.is_empty() {
        return false;
    }
    allowlist
        .iter()
        .map(|entry| entry.trim().to_lowercase())
        .filter(|entry| !entry.is_empty())
        .any(|entry| name == entry)
}

/// 实例地址协议门禁（用户裁决 2026-09-24）：默认仅允许 HTTPS；用户在设置页
/// 显式勾选「允许不安全协议」后才放行 http——API Key 随请求头明文出网，
/// 仅限可信内网环境。地址缺协议（未写 https:// 或 http:// 前缀）一并拒绝。
fn validate_endpoint_url(url: &str, allow_insecure_http: bool) -> Result<(), String> {
    let trimmed = url.trim().to_lowercase();
    let has_scheme = trimmed.starts_with("https://") || trimmed.starts_with("http://");
    let scheme_ok = has_scheme && (allow_insecure_http || trimmed.starts_with("https://"));
    if scheme_ok {
        Ok(())
    } else if !has_scheme {
        Err(String::from(
            "实例地址需以 https:// 开头（勾选「允许不安全协议」后可用 http://）",
        ))
    } else {
        Err(String::from(
            "当前仅允许 HTTPS 地址；如需使用 http（API Key 明文出网，仅限可信内网），请先勾选「允许不安全协议」并保存",
        ))
    }
}

/// 目标本地日期的 [当日 00:00, 次日 00:00) UTC 毫秒窗口（与仓库其他日期边界
/// 口径一致：本地时区当日 00:00 起算，经本机固定偏移换算为 UTC）。
fn local_date_window_millis(date: chrono::NaiveDate, offset: chrono::FixedOffset) -> (i64, i64) {
    let day_start_local = date.and_hms_opt(0, 0, 0).expect("有效的 00:00");
    let start: chrono::DateTime<chrono::Utc> = offset
        .from_local_datetime(&day_start_local)
        .single()
        .expect("偏移换算必须唯一")
        .with_timezone(&chrono::Utc);
    let end = start + chrono::Duration::days(1);
    (start.timestamp_millis(), end.timestamp_millis())
}

/// 剥离 API 返回文本中的 C0 控制字符（同 GitLab 提交标题的注入防护：名称/
/// 项目/状态进入时间线与日报 markdown 时必须保持单行，否则被劫持的服务端
/// 可用 `\n## …` 注入伪造小节；正常输入是恒等变换）。
fn sanitize_single_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch.is_control() {
            if !out.ends_with(' ') {
                out.push(' ');
            }
        } else {
            out.push(ch);
        }
    }
    out
}

/// 工作项 → 入库行：文本字段剥离控制字符，updated_at 毫秒转 UTC unix 秒
/// （调用方已按日期窗口过滤，解析失败的条目不会走到这里）。
fn work_item_to_row(item: &PlanePreviewItem) -> PlaneWorkItemRow {
    PlaneWorkItemRow {
        plane_item_id: sanitize_single_line(&item.id),
        project_name: sanitize_single_line(&item.project),
        name: sanitize_single_line(&item.name),
        state_name: sanitize_single_line(&item.state),
        updated_at: updated_at_sort_key(item) / 1000,
        url: None,
    }
}

/// 判断 Plane 配置是否齐全：开关开启且 url/api_key/workspace_slug 均非空白。
/// 时间线同步与日报小节共用同一口径。
pub(crate) fn plane_configured(plane: &work_review_core::config::PlaneConfig) -> bool {
    plane.enabled
        && [
            plane.url.as_deref(),
            plane.api_key.as_deref(),
            plane.workspace_slug.as_deref(),
        ]
        .into_iter()
        .all(|value| !value.unwrap_or("").trim().is_empty())
}

/// 验证 Plane 实例地址 + API Key + workspace slug 是否有效。
/// 不读取已保存配置：设置页可在未保存前先测试（同 gitlab_test_connection）。
#[tauri::command]
pub async fn plane_test_connection(
    url: String,
    api_key: String,
    workspace_slug: String,
    allow_insecure_http: bool,
) -> Result<PlaneConnectionTestResult, String> {
    if let Err(message) = validate_endpoint_url(&url, allow_insecure_http) {
        return Ok(PlaneConnectionTestResult {
            ok: false,
            workspace_name: String::new(),
            message: Some(message),
        });
    }
    let client = PlaneClient::new(&url, &workspace_slug, &api_key);
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(TEST_CONNECTION_TIMEOUT_SECS),
        client.test_connection(),
    )
    .await;

    match outcome {
        Ok(Ok(workspace_name)) => Ok(PlaneConnectionTestResult {
            ok: true,
            workspace_name,
            message: None,
        }),
        Ok(Err(error)) => Ok(PlaneConnectionTestResult {
            ok: false,
            workspace_name: String::new(),
            message: Some(error.message(&client.api_key)),
        }),
        Err(_) => Ok(PlaneConnectionTestResult {
            ok: false,
            workspace_name: String::new(),
            message: Some(String::from("连接超时（8 秒）")),
        }),
    }
}

/// 拉取工作项预览：列项目后聚合前几个项目的近期工作项，按 updated_at 倒序截
/// limit（默认 20，钳 1..=50）。不读取已保存配置：设置页可在未保存前先预览。
#[tauri::command]
pub async fn plane_fetch_preview(
    url: String,
    api_key: String,
    workspace_slug: String,
    limit: Option<u32>,
    allow_insecure_http: bool,
) -> Result<PlanePreviewResult, String> {
    if let Err(message) = validate_endpoint_url(&url, allow_insecure_http) {
        return Ok(PlanePreviewResult {
            ok: false,
            items: Vec::new(),
            message: Some(message),
        });
    }
    let limit = limit.unwrap_or(DEFAULT_PREVIEW_LIMIT).clamp(1, 50) as usize;
    let client = PlaneClient::new(&url, &workspace_slug, &api_key);
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(PREVIEW_TIMEOUT_SECS),
        client.fetch_preview(limit),
    )
    .await;

    match outcome {
        Ok(Ok(items)) => Ok(PlanePreviewResult {
            ok: true,
            items,
            message: None,
        }),
        Ok(Err(error)) => Ok(PlanePreviewResult {
            ok: false,
            items: Vec::new(),
            message: Some(error.message(&client.api_key)),
        }),
        Err(_) => Ok(PlanePreviewResult {
            ok: false,
            items: Vec::new(),
            message: Some(String::from("获取工作项预览超时（20 秒）")),
        }),
    }
}

/// 时间线页 Plane 工作项补采结果
#[derive(Debug, Clone, Serialize)]
pub struct PlaneSyncResult {
    pub ok: bool,
    /// 本次落在目标日期窗口内并写入数据库的条数（0 也可能是"当天没有工作项更新"）
    pub synced: usize,
    pub message: Option<String>,
}

/// 拉取指定日期（本机时区）的 Plane 工作项并入库，供时间线页打开/切日期时补采
/// （与日报小节同一数据源）。读取已保存配置：enabled + url/api_key/slug 齐全才
/// 执行，否则返回空成功结果（不当失败，前端静默）。项目按 `project_filter`
/// 允许列表过滤（空 = 全部），工作项按 updated_at 落在目标日期
/// [当日 00:00, 次日 00:00)（本地时区）过滤，总预算 20 秒（同一期预览口径）。
#[tauri::command]
pub async fn plane_sync_work_items_for_date(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<PlaneSyncResult, String> {
    let Ok(date_local) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
        return Ok(PlaneSyncResult {
            ok: false,
            synced: 0,
            message: Some(String::from("日期格式无效")),
        });
    };
    // 读已保存配置并在持锁块内完成客户端构造，避免 State 跨 await
    let (client, project_filter) = {
        let state_guard = state.lock().map_err(|e| e.to_string())?;
        let plane = &state_guard.config.plane;
        if !plane_configured(plane) {
            return Ok(PlaneSyncResult {
                ok: true,
                synced: 0,
                message: None,
            });
        }
        if let Err(message) = validate_endpoint_url(
            plane.url.as_deref().unwrap_or(""),
            plane.allow_insecure_http,
        ) {
            return Ok(PlaneSyncResult {
                ok: false,
                synced: 0,
                message: Some(message),
            });
        }
        (
            PlaneClient::new(
                plane.url.as_deref().unwrap_or(""),
                plane.workspace_slug.as_deref().unwrap_or(""),
                plane.api_key.as_deref().unwrap_or(""),
            ),
            plane.project_filter.clone(),
        )
    };

    let offset = work_review_core::gitlab::local_offset_now();
    let (window_start_ms, window_end_ms) = local_date_window_millis(date_local, offset);
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(SYNC_TIMEOUT_SECS),
        client.fetch_work_items_for_sync(&project_filter),
    )
    .await;

    let items = match outcome {
        Ok(Ok(items)) => items,
        Ok(Err(error)) => {
            return Ok(PlaneSyncResult {
                ok: false,
                synced: 0,
                message: Some(error.message(&client.api_key)),
            });
        }
        Err(_) => {
            return Ok(PlaneSyncResult {
                ok: false,
                synced: 0,
                message: Some(String::from("获取 Plane 工作项超时（20 秒）")),
            });
        }
    };

    let rows: Vec<PlaneWorkItemRow> = items
        .iter()
        .map(|item| (updated_at_sort_key(item), item))
        // 按本地日期窗口过滤：updated_at 解析失败的条目（i64::MIN）自然落选
        .filter(|(updated_ms, _)| *updated_ms >= window_start_ms && *updated_ms < window_end_ms)
        .map(|(_, item)| work_item_to_row(item))
        .collect();
    let synced = rows.len();
    let db = state.lock().map_err(|e| e.to_string())?;
    match db.database.upsert_plane_work_items(&date, &rows) {
        Ok(_) => Ok(PlaneSyncResult {
            ok: true,
            synced,
            message: None,
        }),
        Err(error) => Ok(PlaneSyncResult {
            ok: false,
            synced: 0,
            message: Some(error.to_string()),
        }),
    }
}

/// 读取已入库的指定日期 Plane 工作项（按更新时间倒序），时间线页合并展示用。
#[tauri::command]
pub fn get_plane_work_items_for_date(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<PlaneWorkItemRow>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.database
        .get_plane_work_items_for_date(&date)
        .map_err(|e| e.to_string())
}

/// 指定日期的 Plane 工作项归因结果（时间线徽标与日报小节共用数据源）。
/// 按日随用随算、不建表不入库；未配置 Plane 返回空成功（前端静默）。
#[derive(Debug, Clone, Serialize)]
pub struct PlaneAttributionResult {
    pub ok: bool,
    pub attributions: Vec<PlaneAttribution>,
    /// 本次是否用到了 AI 兜底（确定性信号已覆盖全部会话时为 false）
    pub ai_used: bool,
    pub message: Option<String>,
}

/// 归因候选的进程内缓存条目（抓取时刻 + 候选列表）
type CandidateCacheEntry = (std::time::Instant, Vec<PlaneCandidate>);

/// 归因候选的进程内缓存（10 分钟 TTL）：时间线与日报同日各调一次归因时，
/// 避免重复「列项目 + 逐项目工作项」全量拉取。只缓存成功结果，超时/失败不缓存。
fn candidate_cache() -> &'static Mutex<Option<CandidateCacheEntry>> {
    static CACHE: OnceLock<Mutex<Option<CandidateCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

/// 归因计算完整产出（report 路径与命令路径共用）
pub(crate) struct PlaneAttributionOutcome {
    pub ok: bool,
    pub attributions: Vec<PlaneAttribution>,
    pub ai_used: bool,
    pub message: Option<String>,
}

impl PlaneAttributionOutcome {
    fn empty(ok: bool, message: Option<String>) -> Self {
        Self {
            ok,
            attributions: Vec::new(),
            ai_used: false,
            message,
        }
    }
}

/// 带缓存地拉取归因候选（网络等待在锁外进行；错误转可展示消息）。
async fn fetch_candidates_cached(
    client: &PlaneClient,
    project_filter: &[String],
) -> Result<Vec<PlaneCandidate>, String> {
    {
        let guard = candidate_cache().lock().map_err(|e| e.to_string())?;
        if let Some((fetched_at, items)) = guard.as_ref() {
            if fetched_at.elapsed() < std::time::Duration::from_secs(CANDIDATE_CACHE_TTL_SECS) {
                return Ok(items.clone());
            }
        }
    }
    let result = match tokio::time::timeout(
        std::time::Duration::from_secs(ATTRIBUTION_FETCH_TIMEOUT_SECS),
        client.fetch_candidates(project_filter),
    )
    .await
    {
        Ok(Ok(items)) => Ok(items),
        Ok(Err(error)) => Err(error.message(&client.api_key)),
        Err(_) => Err(format!(
            "获取 Plane 工作项候选超时（{ATTRIBUTION_FETCH_TIMEOUT_SECS} 秒）"
        )),
    };
    if let Ok(items) = &result {
        if let Ok(mut guard) = candidate_cache().lock() {
            *guard = Some((std::time::Instant::now(), items.clone()));
        }
    }
    result
}

/// 计算 `date`（本机时区）的 Plane 工作项归因：确定性信号优先，未达阈值
/// （`plane_match::ATTRIBUTION_THRESHOLD`）的会话打包为一次 AI 调用兜底。
///
/// 隐私边界：归因读库统一走 `load_filtered_activities_in_range`（忽略应用/
/// 排除域名的活动根本不参与归因）；AI prompt 只含窗口标题与候选工作项
/// key/名称——`plane_match::extract_session_title_texts` 在类型层面就不携带
/// ocr_text/browser_url，OCR 文本只在本地确定性匹配中使用、永不出网。
pub(crate) async fn compute_plane_attributions(
    date: &str,
    state: &Arc<Mutex<AppState>>,
) -> PlaneAttributionOutcome {
    if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
        return PlaneAttributionOutcome::empty(false, Some(String::from("日期格式无效")));
    }
    // 当日活动窗口换算交给 get_activities_in_range（同款本地时区 [00:00, 次日 00:00)）

    // 持锁块：读配置 + 取当日隐私过滤后的活动与 commits + 文本模型；锁不跨 await
    let (client, project_filter, model_config, activities, commits) = {
        let state_guard = match state.lock() {
            Ok(guard) => guard,
            Err(error) => return PlaneAttributionOutcome::empty(false, Some(error.to_string())),
        };
        let plane = &state_guard.config.plane;
        if !plane_configured(plane) {
            return PlaneAttributionOutcome::empty(true, None);
        }
        if let Err(message) = validate_endpoint_url(
            plane.url.as_deref().unwrap_or(""),
            plane.allow_insecure_http,
        ) {
            return PlaneAttributionOutcome::empty(false, Some(message));
        }
        let client = PlaneClient::new(
            plane.url.as_deref().unwrap_or(""),
            plane.workspace_slug.as_deref().unwrap_or(""),
            plane.api_key.as_deref().unwrap_or(""),
        );
        let activities = match super::shared::load_filtered_activities_in_range(
            &state_guard,
            Some(date),
            Some(date),
            ATTRIBUTION_ACTIVITY_LIMIT,
        ) {
            Ok(activities) => activities,
            Err(error) => return PlaneAttributionOutcome::empty(false, Some(error.to_string())),
        };
        let commits = match state_guard.database.get_gitlab_commits_by_date(date) {
            Ok(commits) => commits,
            Err(error) => return PlaneAttributionOutcome::empty(false, Some(error.to_string())),
        };
        let model_config =
            work_review_core::config::effective_text_model(&state_guard.config.text_model);
        (
            client,
            plane.project_filter.clone(),
            model_config,
            activities,
            commits,
        )
    };

    let candidates = match fetch_candidates_cached(&client, &project_filter).await {
        Ok(candidates) => candidates,
        Err(message) => return PlaneAttributionOutcome::empty(false, Some(message)),
    };
    if candidates.is_empty() {
        return PlaneAttributionOutcome::empty(true, None);
    }

    let instance_host = endpoint_host(&client.base_url);
    let sessions = work_review_core::plane_match::build_activity_sessions(&activities);
    let mut attributions = work_review_core::plane_match::attribute_sessions(
        &sessions,
        &activities,
        &candidates,
        &commits,
        instance_host.as_deref(),
    );

    // AI 兜底：确定性未达阈值的会话打包为一次单轮文本模型调用；
    // 未配置模型 / 调用失败 / 回复解析失败一律静默跳过（归因本就不承诺全覆盖）。
    let mut ai_used = false;
    let all_title_texts =
        work_review_core::plane_match::extract_session_title_texts(&sessions, &activities);
    let unmatched: Vec<(usize, (i64, i64), Vec<String>)> = sessions
        .iter()
        .enumerate()
        .filter(|(_, (start, end))| {
            !attributions
                .iter()
                .any(|a| a.session_start == *start && a.session_end == *end)
        })
        .filter_map(|(index, (start, end))| {
            let titles = all_title_texts.get(index)?.clone();
            Some((index, (*start, *end), titles))
        })
        .collect();
    if !unmatched.is_empty() {
        let session_titles: Vec<Vec<String>> = unmatched
            .iter()
            .map(|(_, _, titles)| titles.clone())
            .collect();
        let prompt = work_review_core::plane_match::build_ai_attribution_prompt(
            &session_titles,
            &candidates,
        );
        let system_prompt = "你是工作项归因助手。只输出 JSON 数组，不要任何解释文字。";
        match super::ask::generate_text_answer_with_model(&model_config, system_prompt, &prompt)
            .await
        {
            Ok(answer) => {
                for (ordinal, key) in
                    work_review_core::plane_match::parse_ai_attribution_reply(&answer)
                {
                    let Some((_, (start, end), _)) = unmatched.get(ordinal) else {
                        continue;
                    };
                    let key_lc = key.to_lowercase();
                    let Some(candidate) = candidates.iter().find(|c| {
                        !c.display_key().is_empty() && c.display_key().to_lowercase() == key_lc
                    }) else {
                        continue;
                    };
                    attributions.push(PlaneAttribution {
                        session_start: *start,
                        session_end: *end,
                        plane_item_id: candidate.plane_item_id.clone(),
                        display_key: {
                            let key = candidate.display_key();
                            if key.is_empty() {
                                candidate.plane_item_id.chars().take(8).collect()
                            } else {
                                key
                            }
                        },
                        project_name: candidate.project_name.clone(),
                        name: candidate.name.clone(),
                        score: AI_FALLBACK_SCORE,
                        evidence: String::from("ai"),
                        source: PlaneAttributionSource::Ai,
                    });
                    ai_used = true;
                }
            }
            Err(error) => log::info!("Plane 归因 AI 兜底跳过: {error}"),
        }
    }

    attributions.sort_by_key(|a| a.session_start);
    PlaneAttributionOutcome {
        ok: true,
        attributions,
        ai_used,
        message: None,
    }
}

/// 时间线页归因命令：时间线加载完成后按日调用（随用随算不入库）。
/// 未配置 Plane 返回空成功；拉取/计算失败返回 ok=false + message（前端静默）。
#[tauri::command]
pub async fn plane_attributions_for_date(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<PlaneAttributionResult, String> {
    let outcome = compute_plane_attributions(&date, state.inner()).await;
    Ok(PlaneAttributionResult {
        ok: outcome.ok,
        attributions: outcome.attributions,
        ai_used: outcome.ai_used,
        message: outcome.message,
    })
}

/// 时间线「今日项目」卡：取某日的人工调整项（removed=当日屏蔽 / 未删=手动添加）
#[tauri::command]
pub async fn get_daily_project_adjustments(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<work_review_core::database::DailyProjectAdjustment>, String> {
    let state_guard = state.lock().map_err(|e| e.to_string())?;
    state_guard
        .database
        .get_daily_project_adjustments(&date)
        .map_err(|e| e.to_string())
}

/// 时间线「今日项目」卡：upsert 一条人工调整（添加 removed=false / 删除屏蔽
/// removed=true；同 (date, project) 覆盖，增删状态可反复翻转）
#[tauri::command]
pub async fn set_daily_project_adjustment(
    date: String,
    project_name: String,
    removed: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let name = project_name.trim().to_string();
    if name.is_empty() {
        return Err("项目名不能为空".to_string());
    }
    let state_guard = state.lock().map_err(|e| e.to_string())?;
    state_guard
        .database
        .upsert_daily_project_adjustment(&date, &name, removed)
        .map_err(|e| e.to_string())
}

/// 时间线「今日项目」卡：Plane 项目下拉数据源（候选取过去重排序）。
/// 未配置 Plane / 拉取失败返回结构化空成功（卡隐藏，不报错打扰）。
#[tauri::command]
pub async fn list_plane_projects(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, String> {
    let (client, project_filter) = {
        let state_guard = state.lock().map_err(|e| e.to_string())?;
        let plane = &state_guard.config.plane;
        if !plane_configured(plane) {
            return Ok(Vec::new());
        }
        validate_endpoint_url(
            plane.url.as_deref().unwrap_or(""),
            plane.allow_insecure_http,
        )?;
        let client = PlaneClient::new(
            plane.url.as_deref().unwrap_or(""),
            plane.workspace_slug.as_deref().unwrap_or(""),
            plane.api_key.as_deref().unwrap_or(""),
        );
        (client, plane.project_filter.clone())
    };
    let candidates = match fetch_candidates_cached(&client, &project_filter).await {
        Ok(items) => items,
        Err(message) => {
            log::warn!("Plane 项目列表拉取失败（今日项目卡隐藏）: {message}");
            return Ok(Vec::new());
        }
    };
    Ok(work_review_core::plane_match::distinct_project_names(
        &candidates,
    ))
}

/// 日报「工作项归因」小节标题（四语言，同 gitlab_section_title 机制）
fn plane_section_title(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "工作项归因",
        AppLocale::ZhTw => "工作項歸因",
        AppLocale::En => "Work Item Attribution",
        AppLocale::Ar => "إسناد بنود العمل",
    }
}

/// 归因来源的展示标签（四语言）
fn attribution_source_label(source: PlaneAttributionSource, locale: AppLocale) -> &'static str {
    match (source, locale) {
        (PlaneAttributionSource::Ai, _) => "AI",
        (PlaneAttributionSource::Deterministic, AppLocale::ZhCn) => "确定性",
        (PlaneAttributionSource::Deterministic, AppLocale::ZhTw) => "確定性",
        (PlaneAttributionSource::Deterministic, AppLocale::En) => "deterministic",
        (PlaneAttributionSource::Deterministic, AppLocale::Ar) => "حتمي",
    }
}

/// 把归因结果渲染成日报「工作项归因」小节 markdown（无归因 = 空串不产出小节）。
/// 行形如：`- 10:00-10:40 [ABC-123] 修复登录页崩溃（确定性）`。
pub(crate) fn plane_attribution_report_section(
    attributions: &[PlaneAttribution],
    offset: chrono::FixedOffset,
    locale: AppLocale,
) -> String {
    if attributions.is_empty() {
        return String::new();
    }
    // 括号随语言：中繁用全角，英/阿用半角
    let (open, close) = match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => ("（", "）"),
        AppLocale::En | AppLocale::Ar => (" (", ")"),
    };
    let lines: Vec<String> = attributions
        .iter()
        .map(|a| {
            let format_time = |ts: i64| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.with_timezone(&offset).format("%H:%M").to_string())
                    .unwrap_or_else(|| "??".to_string())
            };
            format!(
                "- {start}-{end} [{key}] {name} · {project}{open}{source}{close}",
                start = format_time(a.session_start),
                end = format_time(a.session_end),
                key = a.display_key,
                name = sanitize_single_line(&a.name),
                project = sanitize_single_line(&a.project_name),
                source = attribution_source_label(a.source, locale),
            )
        })
        .collect();
    format!("## {}\n\n{}", plane_section_title(locale), lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api地址拼接应规范化并携带工作区slug() {
        let client = PlaneClient::new("https://plane.example.com/", "my-team", "k");
        assert_eq!(
            client.api_url("/projects/"),
            "https://plane.example.com/api/v1/workspaces/my-team/projects/"
        );
        assert_eq!(
            client.projects_url(1),
            "https://plane.example.com/api/v1/workspaces/my-team/projects/?per_page=1"
        );
        let plain = PlaneClient::new("https://plane.example.com", "team", "k");
        assert_eq!(
            plain.api_url("/"),
            "https://plane.example.com/api/v1/workspaces/team/"
        );
    }

    #[test]
    fn 分页信封与裸数组两种响应形态都应能取results() {
        let envelope: serde_json::Value =
            serde_json::from_str(r#"{"total_count": 2, "results": [{"id": "a"}, {"id": "b"}]}"#)
                .unwrap();
        assert_eq!(parse_results(&envelope).len(), 2);
        let bare: serde_json::Value =
            serde_json::from_str(r#"[{"id": "a"}, {"id": "b"}, {"id": "c"}]"#).unwrap();
        assert_eq!(parse_results(&bare).len(), 3);
        let garbage: serde_json::Value = serde_json::from_str(r#"{"error": "x"}"#).unwrap();
        assert!(parse_results(&garbage).is_empty());
    }

    #[test]
    fn 工作项解析应取id名称项目状态序号与更新时间() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "550e8400-...",
                "name": "修复登录页崩溃",
                "priority": "high",
                "sequence_id": 123,
                "state": {"id": "s1", "name": "进行中", "group": "started"},
                "updated_at": "2026-09-24T02:30:00.000Z"
            }"#,
        )
        .unwrap();
        let item = parse_work_item(&value, "示例项目");
        assert_eq!(item.id, "550e8400-...");
        assert_eq!(item.name, "修复登录页崩溃");
        assert_eq!(item.project, "示例项目");
        assert_eq!(item.state, "进行中");
        assert_eq!(item.updated_at, "2026-09-24T02:30:00.000Z");
        assert_eq!(item.sequence_id, 123);
    }

    #[test]
    fn 工作项解析缺更新时间应回退创建时间且未展开状态留空() {
        // 未请求 expand 时 state 是 UUID 字符串而非对象，状态名取不到应留空
        let value: serde_json::Value = serde_json::from_str(
            r#"{
                "name": "t",
                "state": "550e8400-0000-0000-0000-000000000000",
                "created_at": "2026-09-24T01:00:00.000Z"
            }"#,
        )
        .unwrap();
        let item = parse_work_item(&value, "p");
        assert_eq!(item.state, "");
        assert_eq!(item.updated_at, "2026-09-24T01:00:00.000Z");
        assert_eq!(item.sequence_id, 0, "缺 sequence_id 时为 0");
    }

    #[test]
    fn 归因候选解析应携带identifier序号并把时间转为秒() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "550e8400-...",
                "name": "修复登录页崩溃",
                "sequence_id": 456,
                "state": {"id": "s1", "name": "进行中", "group": "started"},
                "updated_at": "2026-09-24T02:30:00.000Z"
            }"#,
        )
        .unwrap();
        let candidate = parse_candidate(&value, "示例项目", "ABC");
        assert_eq!(candidate.plane_item_id, "550e8400-...");
        assert_eq!(candidate.sequence_id, 456);
        assert_eq!(candidate.project_identifier, "ABC");
        assert_eq!(candidate.project_name, "示例项目");
        assert_eq!(candidate.name, "修复登录页崩溃");
        assert_eq!(candidate.state_name, "进行中");
        assert_eq!(
            candidate.updated_at, 1_790_217_000,
            "ISO 时间应转为 UTC unix 秒"
        );
        assert_eq!(candidate.display_key(), "ABC-456");
    }

    #[test]
    fn 实例地址应能提取host供归因url信号使用() {
        assert_eq!(
            endpoint_host("https://plane.example.com"),
            Some("plane.example.com".to_string())
        );
        assert_eq!(
            endpoint_host("https://plane.example.com/"),
            Some("plane.example.com".to_string())
        );
        assert_eq!(
            endpoint_host("http://192.168.1.10:8080"),
            Some("192.168.1.10".to_string())
        );
        assert_eq!(
            endpoint_host("HTTPS://Plane.Example.COM/path"),
            Some("plane.example.com".to_string())
        );
        assert_eq!(endpoint_host("  "), None);
    }

    #[test]
    fn 预览排序应按更新时间倒序且非法时间排最后() {
        let items = vec![
            PlanePreviewItem {
                id: "1".into(),
                name: "old".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "2026-09-01T00:00:00Z".into(),
                sequence_id: 1,
            },
            PlanePreviewItem {
                id: "2".into(),
                name: "bad".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "not-a-date".into(),
                sequence_id: 2,
            },
            PlanePreviewItem {
                id: "3".into(),
                name: "new".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "2026-09-24T00:00:00Z".into(),
                sequence_id: 3,
            },
        ];
        let mut items = items;
        items.sort_by_key(|item| std::cmp::Reverse(updated_at_sort_key(item)));
        assert_eq!(items[0].name, "new");
        assert_eq!(items[1].name, "old");
        assert_eq!(items[2].name, "bad");
    }

    #[test]
    fn 错误消息应剔除api_key回显() {
        let error = PlaneError::Api(500, r#"{"error": "plane_api_secret leaked"}"#.to_string());
        let message = error.message("plane_api_secret");
        assert!(!message.contains("plane_api_secret"), "message: {message}");
        assert!(message.contains("***"));

        // 空密钥时原样返回（不 panic、不误替换）
        let plain = PlaneError::Api(500, "boom".to_string());
        assert_eq!(plain.message(""), "Plane API 返回 500: boom");
    }

    #[test]
    fn 错误消息应翻译401与404提示() {
        let unauthorized = PlaneError::Api(401, String::new());
        assert!(unauthorized.message("k").contains("API Key 无效或已过期"));
        let not_found = PlaneError::Api(404, String::new());
        assert!(not_found
            .message("k")
            .contains("实例地址或 workspace slug 不正确"));
    }

    #[test]
    fn 实例地址协议门禁应按开关放行https与勾选后的http() {
        // 默认仅 HTTPS
        assert!(validate_endpoint_url("https://plane.example.com", false).is_ok());
        assert!(validate_endpoint_url("http://plane.example.com", false).is_err());
        // 勾选「允许不安全协议」后 http 放行，https 依旧放行
        assert!(validate_endpoint_url("http://192.168.1.10:8080", true).is_ok());
        assert!(validate_endpoint_url("https://plane.example.com", true).is_ok());
        // 缺协议一并拒绝（两种开关状态都拒绝）
        assert!(validate_endpoint_url("plane.example.com", false).is_err());
        assert!(validate_endpoint_url("plane.example.com", true).is_err());
    }

    #[test]
    fn 项目允许列表应忽略大小写并仅支持精确匹配() {
        // 空列表 = 全部项目
        assert!(project_matches_allowlist("任意项目", &[]));
        // 精确匹配（忽略大小写与首尾空白）
        let exact = vec!["Alpha".to_string()];
        assert!(project_matches_allowlist("alpha", &exact));
        assert!(project_matches_allowlist("  ALPHA  ", &exact));
        // 仅精确匹配（用户裁决：不做前缀命中，避免「API」误纳「API-docs」）
        let prefix = vec!["alpha".to_string()];
        assert!(!project_matches_allowlist("Alpha Web", &prefix));
        assert!(!project_matches_allowlist("ALPHA 管理后台", &prefix));
        assert!(!project_matches_allowlist("Web Alpha", &prefix));
        assert!(!project_matches_allowlist("Beta", &prefix));
        // 空白条目被忽略，不会造成 starts_with("") 恒真命中
        let blank = vec!["   ".to_string()];
        assert!(!project_matches_allowlist("anything", &blank));
        // 项目名为空不命中任何非空列表
        assert!(!project_matches_allowlist("  ", &prefix));
    }

    #[test]
    fn 本地日期窗口应换算为utc毫秒且右开区间() {
        let offset = chrono::FixedOffset::east_opt(8 * 3600).expect("有效偏移");
        let date = chrono::NaiveDate::from_ymd_opt(2026, 9, 24).expect("有效日期");
        let (start, end) = local_date_window_millis(date, offset);
        // 本地 2026-09-24 00:00 (+08:00) == UTC 2026-09-23 16:00
        assert_eq!(start, 1_790_179_200_000);
        assert_eq!(end - start, 24 * 60 * 60 * 1000, "窗口恰为一整天");
    }

    #[test]
    fn 文本清洗应剥离控制字符保持单行() {
        assert_eq!(sanitize_single_line("修复登录页崩溃"), "修复登录页崩溃");
        assert_eq!(
            sanitize_single_line("标题\n## 伪造小节"),
            "标题 ## 伪造小节"
        );
        assert_eq!(sanitize_single_line("a\tb"), "a b");
    }

    #[test]
    fn 工作项转入库行应清洗文本并把毫秒转为秒() {
        let item = PlanePreviewItem {
            id: "wi-1".into(),
            name: "搭建设计稿\n评审流程".into(),
            project: "Alpha".into(),
            state: "进行中".into(),
            updated_at: "2026-09-24T02:30:00Z".into(),
            sequence_id: 7,
        };
        let row = work_item_to_row(&item);
        assert_eq!(row.plane_item_id, "wi-1");
        assert_eq!(row.name, "搭建设计稿 评审流程", "换行应替换为空格保持单行");
        assert_eq!(row.project_name, "Alpha");
        assert_eq!(row.state_name, "进行中");
        assert_eq!(row.updated_at, 1_790_217_000, "ISO 时间应转为 UTC unix 秒");
        assert_eq!(row.url, None);
    }

    #[test]
    fn plane配置开关与任一凭据缺失都应视为未配置() {
        let mut plane = work_review_core::config::PlaneConfig::default();
        assert!(!plane_configured(&plane), "默认配置未启用");

        plane.enabled = true;
        assert!(!plane_configured(&plane), "缺凭据");

        plane.url = Some("https://plane.example.com".to_string());
        plane.api_key = Some("key".to_string());
        assert!(!plane_configured(&plane), "缺 workspace slug");

        plane.workspace_slug = Some("my-team".to_string());
        assert!(plane_configured(&plane));

        plane.api_key = Some("   ".to_string());
        assert!(!plane_configured(&plane), "空白凭据视为缺失");
    }

    fn attribution_fixture(source: PlaneAttributionSource) -> PlaneAttribution {
        PlaneAttribution {
            session_start: 1_790_217_000, // 2026-09-24 02:30 UTC
            session_end: 1_790_220_600,   // 2026-09-24 03:30 UTC
            plane_item_id: "wi-1".to_string(),
            display_key: "ABC-123".to_string(),
            project_name: "示例项目".to_string(),
            name: "修复登录页\n崩溃".to_string(),
            score: 90,
            evidence: "identifier".to_string(),
            source,
        }
    }

    #[test]
    fn 日报工作项归因小节应四语言成节并清洗换行() {
        let offset = chrono::FixedOffset::east_opt(8 * 3600).expect("有效偏移");
        let attributions = vec![
            attribution_fixture(PlaneAttributionSource::Deterministic),
            attribution_fixture(PlaneAttributionSource::Ai),
        ];
        let zh = plane_attribution_report_section(&attributions, offset, AppLocale::ZhCn);
        assert!(zh.starts_with("## 工作项归因\n\n"));
        assert!(zh.contains("- 10:30-11:30 [ABC-123] 修复登录页 崩溃 · 示例项目（确定性）"));
        assert!(zh.contains("（AI）"));
        assert_eq!(
            plane_attribution_report_section(&attributions, offset, AppLocale::ZhTw),
            "## 工作項歸因\n\n- 10:30-11:30 [ABC-123] 修复登录页 崩溃 · 示例项目（確定性）\n- 10:30-11:30 [ABC-123] 修复登录页 崩溃 · 示例项目（AI）"
        );
        let en = plane_attribution_report_section(&attributions, offset, AppLocale::En);
        assert!(en.starts_with("## Work Item Attribution\n\n"));
        assert!(en.contains("(deterministic)"));
        let ar = plane_attribution_report_section(&attributions, offset, AppLocale::Ar);
        assert!(ar.starts_with("## إسناد بنود العمل\n\n"));
        assert!(ar.contains("(AI)"));
    }

    #[test]
    fn 无归因时日报不应产出工作项归因小节() {
        let offset = chrono::FixedOffset::east_opt(0).expect("有效偏移");
        assert!(plane_attribution_report_section(&[], offset, AppLocale::ZhCn).is_empty());
    }

    #[tokio::test]
    async fn 连接测试未勾选允许不安全协议时应拒绝http地址且不发起请求() {
        let result = plane_test_connection(
            "http://127.0.0.1:1".to_string(),
            "stub-key".to_string(),
            "stub-workspace".to_string(),
            false,
        )
        .await
        .expect("命令不应直接失败");
        assert!(!result.ok, "http 地址在未勾选时应判定为失败");
        assert!(
            result.message.unwrap_or_default().contains("HTTPS"),
            "失败原因应指向协议门禁"
        );
    }

    #[tokio::test]
    async fn 连接测试指向不可达地址应返回结构化失败而不是panic() {
        // 127.0.0.1:1 上无服务，连接立即失败；命令必须返回 ok=false + message
        let result = plane_test_connection(
            "http://127.0.0.1:1".to_string(),
            "stub-key".to_string(),
            "stub-workspace".to_string(),
            true,
        )
        .await
        .expect("命令不应直接失败");
        assert!(!result.ok, "不可达地址应判定为失败");
        assert!(result.workspace_name.is_empty(), "失败时不应携带工作区名");
        let message = result.message.unwrap_or_default();
        assert!(!message.is_empty(), "失败时应给出可展示的原因");
        assert!(!message.contains("stub-key"), "错误串不得包含 api_key");
    }

    #[tokio::test]
    async fn 预览指向不可达地址应返回结构化失败而不是panic() {
        let result = plane_fetch_preview(
            "http://127.0.0.1:1".to_string(),
            "stub-key".to_string(),
            "stub-workspace".to_string(),
            Some(20),
            true,
        )
        .await
        .expect("命令不应直接失败");
        assert!(!result.ok, "不可达地址应判定为失败");
        assert!(result.items.is_empty(), "失败时不应携带预览数据");
        let message = result.message.unwrap_or_default();
        assert!(!message.is_empty(), "失败时应给出可展示的原因");
        assert!(!message.contains("stub-key"), "错误串不得包含 api_key");
    }
}
