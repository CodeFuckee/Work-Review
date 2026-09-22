//! GitLab 集成命令（设置页「测试连接」/「仓库选择器」/「作者选择器」与时间线页提交同步）。

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::State;
use work_review_core::gitlab::GitlabClient;

use crate::AppState;

/// GitLab 连接测试结果（设置页展示）
#[derive(Debug, Clone, Serialize)]
pub struct GitlabConnectionTestResult {
    pub ok: bool,
    pub user_name: String,
    pub username: String,
    pub project_count: usize,
    pub allowed_count: usize,
    pub message: Option<String>,
}

/// 仓库选择器条目（设置页「从 GitLab 选择仓库」）
#[derive(Debug, Clone, Serialize)]
pub struct GitlabProjectOption {
    pub path_with_namespace: String,
}

/// 仓库列表拉取结果
#[derive(Debug, Clone, Serialize)]
pub struct GitlabProjectListResult {
    pub ok: bool,
    pub projects: Vec<GitlabProjectOption>,
    pub message: Option<String>,
}

/// 拉取 token 有权限的全部项目（按路径排序），供仓库选择器勾选。
/// 不读取已保存配置：设置页可在未保存前先选。
#[tauri::command]
pub async fn gitlab_list_projects(
    url: String,
    token: String,
) -> Result<GitlabProjectListResult, String> {
    let client = GitlabClient::new(&url, &token);
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(8),
        client.fetch_all_projects(),
    )
    .await;

    match outcome {
        Ok(Ok(mut projects)) => {
            // 每次比较都 to_lowercase 会产生 O(n log n) 次字符串分配，
            // cached_key 只对每个项目算一次键
            projects.sort_by_cached_key(|p| p.path_with_namespace.to_lowercase());
            Ok(GitlabProjectListResult {
                ok: true,
                projects: projects
                    .into_iter()
                    .map(|p| GitlabProjectOption {
                        path_with_namespace: p.path_with_namespace,
                    })
                    .collect(),
                message: None,
            })
        }
        Ok(Err(error)) => Ok(GitlabProjectListResult {
            ok: false,
            projects: Vec::new(),
            message: Some(error.to_string()),
        }),
        Err(_) => Ok(GitlabProjectListResult {
            ok: false,
            projects: Vec::new(),
            message: Some(String::from("获取仓库列表超时（8 秒）")),
        }),
    }
}

/// 验证 GitLab URL + Token 是否有效，返回当前用户与有权限项目数。
/// 不读取已保存配置：设置页可在未保存前先测试。
#[tauri::command]
pub async fn gitlab_test_connection(
    url: String,
    token: String,
    project_filter: Option<Vec<String>>,
) -> Result<GitlabConnectionTestResult, String> {
    let client = GitlabClient::new(&url, &token);
    let outcome = tokio::time::timeout(std::time::Duration::from_secs(8), async {
        let user = client.fetch_current_user().await?;
        let projects = client.fetch_all_projects().await?;
        let filter = project_filter.unwrap_or_default();
        let allowed_count =
            work_review_core::gitlab::filter_projects_by_allowlist(&projects, &filter).len();
        Ok::<_, work_review_core::gitlab::GitlabError>((user, projects.len(), allowed_count))
    })
    .await;

    match outcome {
        Ok(Ok((user, project_count, allowed_count))) => Ok(GitlabConnectionTestResult {
            ok: true,
            user_name: user.name,
            username: user.username,
            project_count,
            allowed_count,
            message: None,
        }),
        Ok(Err(error)) => Ok(GitlabConnectionTestResult {
            ok: false,
            user_name: String::new(),
            username: String::new(),
            project_count: 0,
            allowed_count: 0,
            message: Some(error.to_string()),
        }),
        Err(_) => Ok(GitlabConnectionTestResult {
            ok: false,
            user_name: String::new(),
            username: String::new(),
            project_count: 0,
            allowed_count: 0,
            message: Some(String::from("连接超时（8 秒）")),
        }),
    }
}

/// 作者选择器条目（设置页「从仓库选择作者」；每个邮箱一行）
#[derive(Debug, Clone, Serialize)]
pub struct GitlabAuthorOption {
    pub name: String,
    pub email: String,
    pub commits: i64,
    pub is_current_user: bool,
}

/// 作者列表拉取结果
#[derive(Debug, Clone, Serialize)]
pub struct GitlabAuthorListResult {
    pub ok: bool,
    pub authors: Vec<GitlabAuthorOption>,
    /// 允许列表为空且仓库超过扫描上限时提示（只统计了前 N 个仓库）
    pub truncated: bool,
    pub message: Option<String>,
}

/// 拉取允许列表（为空 = 全部有权限项目）内仓库的贡献者，按邮箱聚合为
/// 作者候选（提交数倒序），供作者选择器勾选。
/// 不读取已保存配置：设置页可在未保存前先选。
#[tauri::command]
pub async fn gitlab_list_authors(
    url: String,
    token: String,
    project_filter: Option<Vec<String>>,
) -> Result<GitlabAuthorListResult, String> {
    let client = GitlabClient::new(&url, &token);
    let filter = project_filter.unwrap_or_default();
    // 贡献者按仓库逐个拉取，比仓库列表慢；并发 8 + 仓库数上限下 25 秒足够宽裕
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(25),
        client.fetch_author_candidates(&filter),
    )
    .await;

    match outcome {
        Ok(Ok((candidates, truncated))) => Ok(GitlabAuthorListResult {
            ok: true,
            authors: candidates
                .into_iter()
                .map(|c| GitlabAuthorOption {
                    name: c.name,
                    email: c.email,
                    commits: c.commits,
                    is_current_user: c.is_current_user,
                })
                .collect(),
            truncated,
            message: None,
        }),
        Ok(Err(error)) => Ok(GitlabAuthorListResult {
            ok: false,
            authors: Vec::new(),
            truncated: false,
            message: Some(error.to_string()),
        }),
        Err(_) => Ok(GitlabAuthorListResult {
            ok: false,
            authors: Vec::new(),
            truncated: false,
            message: Some(String::from("获取作者列表超时（25 秒）")),
        }),
    }
}

/// 时间线页 GitLab 提交补采结果
#[derive(Debug, Clone, Serialize)]
pub struct GitlabCommitSyncResult {
    pub ok: bool,
    /// 本次新写入/更新的提交条数（0 也可能是"远端本来就没有提交"）
    pub synced: usize,
    /// 涉及项目的仓库头像（data URL；无自定义头像/拉取失败为 None，走默认图标）
    pub avatars: Vec<GitlabProjectAvatarEntry>,
    pub message: Option<String>,
}

/// 概览卡片用的仓库头像条目
#[derive(Debug, Clone, Serialize)]
pub struct GitlabProjectAvatarEntry {
    /// path_with_namespace
    pub path: String,
    /// data:image/...;base64 形式；None = 走默认图标
    pub data_url: Option<String>,
}

/// 拉取指定日期（本机时区）的 GitLab 提交并入库，供时间线页打开/切日期时补采。
/// 作者与项目过滤口径由前端透传（与设置页保存的日报口径一致）；30 秒超时
/// （含提交行数统计与仓库头像的补拉预算）。
#[tauri::command]
pub async fn gitlab_sync_commits_for_date(
    url: String,
    token: String,
    date: String,
    project_filter: Option<Vec<String>>,
    work_authors: Option<Vec<String>>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<GitlabCommitSyncResult, String> {
    let Ok(date_local) = chrono::NaiveDate::parse_from_str(&date, "%Y-%m-%d") else {
        return Ok(GitlabCommitSyncResult {
            ok: false,
            synced: 0,
            avatars: Vec::new(),
            message: Some(String::from("日期格式无效")),
        });
    };
    let client = GitlabClient::new(&url, &token);
    let offset = work_review_core::gitlab::local_offset_now();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        client.fetch_today_commits(
            offset,
            date_local,
            &project_filter.unwrap_or_default(),
            &work_authors.unwrap_or_default(),
        ),
    )
    .await;

    match outcome {
        Ok(Ok(result)) => {
            let rows = work_review_core::gitlab::entries_to_rows(&result.entries, &date);
            let synced = rows.len();
            // 有自定义头像的项目并发补拉字节并转 data URL（webview CSP 不放行远程图片）
            let avatar_pairs: Vec<(String, i64)> = result
                .project_avatars
                .iter()
                .filter(|p| p.avatar_url.is_some())
                .map(|p| (p.path.clone(), p.project_id))
                .collect();
            let avatars = client.fetch_avatar_data_urls(&avatar_pairs).await;

            let db = state.lock().map_err(|e| e.to_string())?;
            match db.database.upsert_gitlab_commits(&rows) {
                Ok(_) => Ok(GitlabCommitSyncResult {
                    ok: true,
                    synced,
                    avatars: avatars
                        .into_iter()
                        .map(|(path, data_url)| GitlabProjectAvatarEntry { path, data_url })
                        .collect(),
                    message: None,
                }),
                Err(error) => Ok(GitlabCommitSyncResult {
                    ok: false,
                    synced: 0,
                    avatars: Vec::new(),
                    message: Some(error.to_string()),
                }),
            }
        }
        Ok(Err(error)) => Ok(GitlabCommitSyncResult {
            ok: false,
            synced: 0,
            avatars: Vec::new(),
            message: Some(error.to_string()),
        }),
        Err(_) => Ok(GitlabCommitSyncResult {
            ok: false,
            synced: 0,
            avatars: Vec::new(),
            message: Some(String::from("获取 GitLab 提交超时（30 秒）")),
        }),
    }
}

/// 读取已入库的指定日期 GitLab 提交（按提交时间升序），时间线页合并展示用。
#[tauri::command]
pub fn get_gitlab_commits_for_date(
    date: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<work_review_core::database::GitlabCommitRow>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.database
        .get_gitlab_commits_by_date(&date)
        .map_err(|e| e.to_string())
}

/// 按本地日期区间（含两端）聚合已入库的 GitLab 提交：每项目一行
/// （提交次数、新增/删除行数合计，提交次数倒序），概览页统计卡片用。
#[tauri::command]
pub fn get_gitlab_project_stats(
    date_from: String,
    date_to: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<work_review_core::database::GitlabProjectStatRow>, String> {
    let db = state.lock().map_err(|e| e.to_string())?;
    db.database
        .get_gitlab_project_stats(&date_from, &date_to)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn 连接测试指向不可达地址应返回结构化失败而不是panic() {
        // 127.0.0.1:1 上无服务，连接立即失败；命令必须返回 ok=false + message
        let result = gitlab_test_connection(
            "http://127.0.0.1:1".to_string(),
            "stub-token".to_string(),
            None,
        )
        .await
        .expect("命令不应直接失败");
        assert!(!result.ok, "不可达地址应判定为失败");
        let message = result.message.unwrap_or_default();
        assert!(!message.is_empty(), "失败时应给出可展示的原因");
    }

    #[tokio::test]
    async fn 仓库列表指向不可达地址应返回结构化失败而不是panic() {
        let result =
            gitlab_list_projects("http://127.0.0.1:1".to_string(), "stub-token".to_string())
                .await
                .expect("命令不应直接失败");
        assert!(!result.ok, "不可达地址应判定为失败");
        assert!(result.projects.is_empty(), "失败时不应携带项目数据");
        let message = result.message.unwrap_or_default();
        assert!(!message.is_empty(), "失败时应给出可展示的原因");
    }

    #[tokio::test]
    async fn 作者列表指向不可达地址应返回结构化失败而不是panic() {
        let result = gitlab_list_authors(
            "http://127.0.0.1:1".to_string(),
            "stub-token".to_string(),
            None,
        )
        .await
        .expect("命令不应直接失败");
        assert!(!result.ok, "不可达地址应判定为失败");
        assert!(result.authors.is_empty(), "失败时不应携带作者数据");
        assert!(!result.truncated, "失败时不应标记截断");
        let message = result.message.unwrap_or_default();
        assert!(!message.is_empty(), "失败时应给出可展示的原因");
    }
}
