//! GitLab 集成命令（设置页「测试连接」）。

use serde::Serialize;
use work_review_core::gitlab::GitlabClient;

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
}
