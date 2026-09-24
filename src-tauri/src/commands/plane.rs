//! Plane 集成命令（设置页「测试连接」与工作项预览）。
//!
//! 一期范围仅为设置 + 连接测试 + 预览；时间线/日报联动为下一期。
//! 端点与响应字段依据官方文档 <https://developers.plane.so>（api-reference）：
//! - `GET /api/v1/workspaces/{workspace_slug}/projects/`（projects:read，
//!   分页信封 `results`，项目对象含 `id` / `name`）；
//! - `GET /api/v1/workspaces/{workspace_slug}/projects/{project_id}/work-items/`
//!   （`order_by` 支持 `-` 前缀倒序、`per_page` 每页条数、`expand=state` 把状态
//!   UUID 展开为 `{id, name, group}` 对象——文档示例响应即此形态）；
//! - 鉴权：请求头 `X-API-Key: <Personal Access Token>`。
//!
//! 工作区级「近期 issues」列表端点在 api/v1 无文档（仅有必须携带搜索词的
//! search 端点），预览按「列项目 → 聚合前几个项目的近期工作项」实现。

use serde::Serialize;

/// 连接测试超时（秒）
const TEST_CONNECTION_TIMEOUT_SECS: u64 = 8;
/// 预览拉取总超时（秒）：1 次项目列表 + 最多 MAX_PREVIEW_PROJECTS 次工作项请求
const PREVIEW_TIMEOUT_SECS: u64 = 20;
/// 项目列表单页条数（预览只取第一页，避免多页翻页失控）
const PREVIEW_PROJECT_PAGE_SIZE: u32 = 100;
/// 预览聚合的项目数上限（取项目列表前 N 个）
const MAX_PREVIEW_PROJECTS: usize = 8;
/// 每个项目最多拉取的工作项数（按 updated_at 倒序取前 N）
const PER_PROJECT_WORK_ITEM_LIMIT: u32 = 10;
/// 预览条数默认值
pub const DEFAULT_PREVIEW_LIMIT: u32 = 20;

/// Plane 连接测试结果（设置页展示）
#[derive(Debug, Clone, Serialize)]
pub struct PlaneConnectionTestResult {
    pub ok: bool,
    pub workspace_name: String,
    pub message: Option<String>,
}

/// 工作项预览条目（名称 / 项目名 / 状态 / 更新时间）
#[derive(Debug, Clone, Serialize)]
pub struct PlanePreviewItem {
    pub name: String,
    pub project: String,
    pub state: String,
    pub updated_at: String,
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
        serde_json::from_slice(&bytes).map_err(PlaneError::Json)
    }

    /// 连接测试：优先 `GET /api/v1/workspaces/{slug}/`（旧版实例的端点，可取回
    /// 工作区名）；现行版本已移除该端点，404 时回退用文档列出的项目列表端点
    /// `GET /api/v1/workspaces/{slug}/projects/?per_page=1` 验证（工作区名回退
    /// 为 slug）。两种路径下 401 → Key 无效、最终 404 → 地址或 slug 不正确。
    async fn test_connection(&self) -> Result<String, PlaneError> {
        let workspace_url = self.api_url("/");
        match self.get_json(&workspace_url).await {
            Ok(value) => {
                let name = value
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                Ok(if name.is_empty() {
                    self.workspace_slug.clone()
                } else {
                    name
                })
            }
            Err(PlaneError::Api(404, _)) => {
                self.get_json(&self.projects_url(1)).await?;
                Ok(self.workspace_slug.clone())
            }
            Err(error) => Err(error),
        }
    }

    /// 预览工作项：列项目（第一页）后取前 `MAX_PREVIEW_PROJECTS` 个项目，逐个
    /// 拉近期工作项（`order_by=-updated_at` + `expand=state`）；单个项目失败仅
    /// 跳过（log::warn，与 GitLab 的部分失败策略一致），全局按 updated_at 倒序
    /// 截 limit。
    async fn fetch_preview(&self, limit: usize) -> Result<Vec<PlanePreviewItem>, PlaneError> {
        let projects_value = self
            .get_json(&self.projects_url(PREVIEW_PROJECT_PAGE_SIZE))
            .await?;
        let projects = parse_results(&projects_value);
        let selected: Vec<(String, String)> = projects
            .iter()
            .take(MAX_PREVIEW_PROJECTS)
            .map(|project| {
                (json_str(project, "id"), {
                    let name = json_str(project, "name");
                    if name.is_empty() {
                        json_str(project, "identifier")
                    } else {
                        name
                    }
                })
            })
            .filter(|(id, _)| !id.is_empty())
            .collect();

        let mut items: Vec<PlanePreviewItem> = Vec::new();
        for (project_id, project_name) in &selected {
            let url = self.api_url(&format!(
                "/projects/{project_id}/work-items/?per_page={PER_PROJECT_WORK_ITEM_LIMIT}&order_by=-updated_at&expand=state"
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
        items.sort_by_key(|item| std::cmp::Reverse(updated_at_sort_key(item)));
        items.truncate(limit);
        Ok(items)
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

/// 解析单个工作项：名称 / 项目名 / 状态名 / updated_at（缺 updated_at 时
/// 回退 created_at，文档示例两字段均为 ISO8601 字符串；`expand=state` 时
/// state 为含 `name` 的对象，未展开时是 UUID——后者状态名留空）。
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
        name: json_str(value, "name"),
        project: project_name.to_string(),
        state: value
            .get("state")
            .and_then(|state| state.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        updated_at,
    }
}

/// 排序键：updated_at（ISO8601）解析为毫秒时间戳；解析失败的条目排最后。
fn updated_at_sort_key(item: &PlanePreviewItem) -> i64 {
    chrono::DateTime::parse_from_rfc3339(&item.updated_at)
        .map(|dt| dt.timestamp_millis())
        .unwrap_or(i64::MIN)
}

/// 验证 Plane 实例地址 + API Key + workspace slug 是否有效。
/// 不读取已保存配置：设置页可在未保存前先测试（同 gitlab_test_connection）。
#[tauri::command]
pub async fn plane_test_connection(
    url: String,
    api_key: String,
    workspace_slug: String,
) -> Result<PlaneConnectionTestResult, String> {
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
) -> Result<PlanePreviewResult, String> {
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
    fn 工作项解析应取名称项目状态与更新时间() {
        let value: serde_json::Value = serde_json::from_str(
            r#"{
                "id": "550e8400-...",
                "name": "修复登录页崩溃",
                "priority": "high",
                "state": {"id": "s1", "name": "进行中", "group": "started"},
                "updated_at": "2026-09-24T02:30:00.000Z"
            }"#,
        )
        .unwrap();
        let item = parse_work_item(&value, "示例项目");
        assert_eq!(item.name, "修复登录页崩溃");
        assert_eq!(item.project, "示例项目");
        assert_eq!(item.state, "进行中");
        assert_eq!(item.updated_at, "2026-09-24T02:30:00.000Z");
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
    }

    #[test]
    fn 预览排序应按更新时间倒序且非法时间排最后() {
        let items = vec![
            PlanePreviewItem {
                name: "old".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "2026-09-01T00:00:00Z".into(),
            },
            PlanePreviewItem {
                name: "bad".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "not-a-date".into(),
            },
            PlanePreviewItem {
                name: "new".into(),
                project: "p".into(),
                state: String::new(),
                updated_at: "2026-09-24T00:00:00Z".into(),
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

    #[tokio::test]
    async fn 连接测试指向不可达地址应返回结构化失败而不是panic() {
        // 127.0.0.1:1 上无服务，连接立即失败；命令必须返回 ok=false + message
        let result = plane_test_connection(
            "http://127.0.0.1:1".to_string(),
            "stub-key".to_string(),
            "stub-workspace".to_string(),
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
