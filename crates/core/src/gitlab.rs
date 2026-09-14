//! 私有化 GitLab 集成：提交拉取（REST API v4）、按用户/今日过滤、
//! 以及日报上下文文本构建。
//!
//! 分层约定：本模块同时承载网络（`GitlabClient`，reqwest）与纯函数（过滤/
//! 格式化）；纯函数全部可单测，网络部分保持薄封装。日报生成流程
//! （`src-tauri` 的 `commands/report.rs`）在生成前调用 `GitlabClient` 拉取
//! 当日提交，把结果转成 `GitlabReportSupplement` 传给分析器。

use crate::analysis::AppLocale;
use chrono::{DateTime, FixedOffset, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};

/// 每个项目最多拉取的提交数
pub const PER_PROJECT_COMMIT_LIMIT: usize = 20;
/// 全局（全部项目合计）最多展示的提交数
pub const TOTAL_COMMIT_LIMIT: usize = 200;

/// GitLab 当前用户（GET /user）
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GitlabUser {
    pub id: i64,
    pub username: String,
    pub name: String,
    pub email: String,
}

/// 一次提交的变更统计
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitlabCommitStats {
    pub additions: i64,
    pub deletions: i64,
}

/// GitLab repository/commits API 返回的单条提交
/// （字段名与 GitLab v4 REST 保持 snake_case 一致）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitlabCommit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub message: String,
    pub author_name: String,
    pub author_email: String,
    /// ISO8601（GitLab 返回 UTC）
    pub committed_date: String,
    #[serde(default)]
    pub stats: Option<GitlabCommitStats>,
}

/// GitLab projects API 返回的项目（simple=true / 常规字段）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GitlabProject {
    pub id: i64,
    #[serde(default)]
    pub path_with_namespace: String,
    #[serde(default)]
    pub default_branch: Option<String>,
}

/// 一条聚合后的提交条目（带项目与分支上下文）
#[derive(Debug, Clone, PartialEq)]
pub struct GitlabCommitEntry {
    pub commit: GitlabCommit,
    /// path_with_namespace
    pub project: String,
    /// 分支（当前实现取项目默认分支）
    pub branch: String,
}

/// 传给日报分析器的 GitLab 补充内容
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GitlabReportSupplement {
    /// 供 LLM 的上下文（含提交列表与整合指令）；无提交时为 None
    pub ai_context: Option<String>,
    /// 本地模板用的小节（无提交时为空字符串）
    pub local_section: String,
    /// 拉取失败时的备注行（None = 不提示）
    pub failure_note: Option<String>,
}

/// 判定 committed_date（UTC ISO8601）是否落在本地 offset 时区的指定日期。
/// 解析失败返回 false。
pub fn is_commit_on_local_date(
    committed_date_utc: &str,
    offset: FixedOffset,
    date: NaiveDate,
) -> bool {
    let Ok(parsed) = DateTime::parse_from_rfc3339(committed_date_utc) else {
        return false;
    };
    parsed.with_timezone(&offset).date_naive() == date
}

/// 提交是否属于该用户：author_email 精确（忽略大小写）匹配优先，
/// 无 email 时再按 author_name 与 user.name / user.username 忽略大小写匹配。
pub fn commit_belongs_to_user(commit: &GitlabCommit, user: &GitlabUser) -> bool {
    let commit_email = commit.author_email.trim().to_lowercase();
    let user_email = user.email.trim().to_lowercase();
    if !commit_email.is_empty() && !user_email.is_empty() && commit_email == user_email {
        return true;
    }
    // token 对应用户不暴露邮箱时，邮箱不作为负判据（继续按名字匹配）
    let commit_name = commit.author_name.trim().to_lowercase();
    if commit_name.is_empty() {
        return false;
    }
    let user_name = user.name.trim().to_lowercase();
    let user_username = user.username.trim().to_lowercase();
    commit_name == user_name || (!user_username.is_empty() && commit_name == user_username)
}

/// 项目允许列表过滤：空列表 = 全部；路径大小写不敏感精确匹配。
pub fn filter_projects_by_allowlist<'a>(
    projects: &'a [GitlabProject],
    allowlist: &[String],
) -> Vec<&'a GitlabProject> {
    let lower: Vec<String> = allowlist
        .iter()
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    if lower.is_empty() {
        return projects.iter().collect();
    }
    projects
        .iter()
        .filter(|p| lower.contains(&p.path_with_namespace.trim().to_lowercase()))
        .collect()
}

/// 全局截断：超过 TOTAL_COMMIT_LIMIT 时保留前 N 条并标记 truncated。
pub fn cap_total_entries(entries: Vec<GitlabCommitEntry>) -> (Vec<GitlabCommitEntry>, bool) {
    if entries.len() <= TOTAL_COMMIT_LIMIT {
        return (entries, false);
    }
    let kept: Vec<GitlabCommitEntry> = entries.into_iter().take(TOTAL_COMMIT_LIMIT).collect();
    (kept, true)
}

/// 构建供 LLM 整合日报的 GitLab 上下文；无提交时返回 None。
/// `total` 为截断前的总条数（供「还有 N 条未展示」提示）。
pub fn build_gitlab_ai_context(
    entries: &[GitlabCommitEntry],
    truncated: bool,
    total: usize,
    offset: FixedOffset,
    locale: AppLocale,
) -> Option<String> {
    if entries.is_empty() {
        return None;
    }
    let count = entries.len();
    let lines: Vec<String> = entries
        .iter()
        .map(|e| format_commit_line(e, offset, locale, false))
        .collect();
    let mut body = format!(
        "## {}\n\n{}",
        gitlab_section_title(locale),
        lines.join("\n")
    );
    if truncated {
        let skipped = total.saturating_sub(count);
        if skipped > 0 {
            body.push_str(&format!("\n\n{}", truncated_hint(skipped, locale)));
        }
    }
    let integration = integration_instruction(locale);
    body.push_str(&format!("\n\n{integration}"));
    Some(body)
}

/// 构建本地模板（非 AI）追加的「GitLab 今日提交」Markdown 小节；无提交返回空串。
pub fn build_gitlab_local_section(
    entries: &[GitlabCommitEntry],
    truncated: bool,
    total: usize,
    offset: FixedOffset,
    locale: AppLocale,
) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let count = entries.len();
    let mut lines: Vec<String> = entries
        .iter()
        .map(|e| format!("- {}", format_commit_line(e, offset, locale, false)))
        .collect();
    if truncated {
        let skipped = total.saturating_sub(count);
        if skipped > 0 {
            lines.push(format!("- {}", truncated_hint(skipped, locale)));
        }
    }
    format!(
        "## {}（{}）\n\n{}",
        gitlab_section_title(locale),
        count,
        lines.join("\n")
    )
}

/// 拉取失败时的日报备注行（按 locale）。
pub fn gitlab_failure_note(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => "（GitLab 提交获取失败，已跳过）".to_string(),
        AppLocale::ZhTw => "（GitLab 提交獲取失敗，已略過）".to_string(),
        AppLocale::En => "(GitLab commits could not be fetched and were skipped)".to_string(),
        AppLocale::Ar => "(تعذّر جلب تسليمات GitLab وتم تخطّيها)".to_string(),
    }
}

/// 当前本机时区偏移（供调用方取用；测试用显式偏移绕开它）
pub fn local_offset_now() -> FixedOffset {
    *chrono::Local::now().offset()
}

fn gitlab_section_title(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => "GitLab 今日提交",
        AppLocale::En => "GitLab Commits Today",
        AppLocale::Ar => "تسليمات GitLab اليوم",
    }
}

fn truncated_hint(skipped: usize, locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => format!("还有 {skipped} 条未展示"),
        AppLocale::ZhTw => format!("還有 {skipped} 條未展示"),
        AppLocale::En => format!("{skipped} more commits are not shown"),
        AppLocale::Ar => format!("لا تُعرض {skipped} تسليمات أخرى"),
    }
}

fn integration_instruction(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => {
            "请将上述 GitLab 提交作为当日代码工作，整合进日报的进展/代码提交等小节；不得编造列表之外的提交。".to_string()
        }
        AppLocale::ZhTw => {
            "請將上述 GitLab 提交視為當日程式碼工作，整合進日報的進展/程式碼提交等小節；不得虛構清單之外的提交。".to_string()
        }
        AppLocale::En => {
            "Incorporate the GitLab commits above as today's coding work in the relevant report sections (e.g. progress / code commits); do not invent commits not listed.".to_string()
        }
        AppLocale::Ar => {
            "ادمج تسليمات GitLab أعلاه كعمل برمجة اليوم في الأقسام ذات الصلة بالتقرير؛ ولا تخترع تسليمات غير مذكورة.".to_string()
        }
    }
}

/// 单条提交的展示行。
/// 形如：`[group/proj] [main] abc1234 fix: xxx (+12/-3) 10:30`
fn format_commit_line(
    entry: &GitlabCommitEntry,
    offset: FixedOffset,
    _locale: AppLocale,
    _for_ai: bool,
) -> String {
    let committed_at = DateTime::parse_from_rfc3339(&entry.commit.committed_date)
        .map(|dt| dt.with_timezone(&offset).format("%H:%M").to_string())
        .unwrap_or_else(|_| "??".to_string());
    let stats = entry
        .commit
        .stats
        .map(|s| format!(" (+{}/-{})", s.additions, s.deletions))
        .unwrap_or_default();
    format!(
        "[{project}] [{branch}] {short_id} {title}{stats} {committed_at}",
        project = entry.project,
        branch = entry.branch,
        short_id = entry.commit.short_id,
        title = entry.commit.title,
    )
}

/// 拉取/过滤 GitLab 提交的聚合结果
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GitlabFetchResult {
    pub entries: Vec<GitlabCommitEntry>,
    /// 截断后是否丢弃了部分提交
    pub truncated: bool,
    /// 截断前总数
    pub total: usize,
}

impl GitlabFetchResult {
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// 转成日报分析器补充内容
    pub fn to_supplement(&self, offset: FixedOffset, locale: AppLocale) -> GitlabReportSupplement {
        GitlabReportSupplement {
            ai_context: build_gitlab_ai_context(
                &self.entries,
                self.truncated,
                self.total,
                offset,
                locale,
            ),
            local_section: build_gitlab_local_section(
                &self.entries,
                self.truncated,
                self.total,
                offset,
                locale,
            ),
            failure_note: None,
        }
    }
}

/// GitLab REST 客户端。
///
/// 设计为可注入 HTTP client（默认 `reqwest::Client`），保持薄封装：
/// 只负责 URL 拼接、分页与 JSON 反序列化，业务判断全部在纯函数中。
pub struct GitlabClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl GitlabClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        // 必须显式设超时：reqwest 默认无超时，GitLab 挂起时日报生成会无限阻塞
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .unwrap_or_default();
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.trim().to_string(),
            http,
        }
    }

    fn api_url(&self, path: &str) -> String {
        format!("{}/api/v4{}", self.base_url, path)
    }

    fn auth_header(&self) -> &'static str {
        "PRIVATE-TOKEN"
    }

    /// GET /user — 当前 token 的用户
    pub async fn fetch_current_user(&self) -> Result<GitlabUser, GitlabError> {
        let resp = self
            .http
            .get(self.api_url("/user"))
            .header(self.auth_header(), &self.token)
            .send()
            .await
            .map_err(GitlabError::Http)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(GitlabError::Http)?;
        if !status.is_success() {
            return Err(GitlabError::Api(
                status.as_u16(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(GitlabError::Json)?;
        Ok(GitlabUser {
            id: value.get("id").and_then(|v| v.as_i64()).unwrap_or(0),
            username: value
                .get("username")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            name: value
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            email: value
                .get("email")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
        })
    }

    /// GET /projects?membership=true&simple=true&per_page=100 — 有权限的全部项目
    pub async fn fetch_all_projects(&self) -> Result<Vec<GitlabProject>, GitlabError> {
        let mut projects = Vec::new();
        let mut page = 1u32;
        let max_pages = 20; // 最多 2000 个项目，避免失控
        loop {
            let resp = self
                .http
                .get(self.api_url("/projects"))
                .query(&[
                    ("membership", "true"),
                    ("simple", "true"),
                    ("per_page", "100"),
                    ("page", &page.to_string()),
                ])
                .header(self.auth_header(), &self.token)
                .send()
                .await
                .map_err(GitlabError::Http)?;
            let status = resp.status();
            let bytes = resp.bytes().await.map_err(GitlabError::Http)?;
            if !status.is_success() {
                return Err(GitlabError::Api(
                    status.as_u16(),
                    String::from_utf8_lossy(&bytes).into_owned(),
                ));
            }
            let batch: Vec<GitlabProject> =
                serde_json::from_slice(&bytes).map_err(GitlabError::Json)?;
            let fetched = batch.len();
            projects.extend(batch);
            if fetched < 100 || page >= max_pages {
                break;
            }
            page += 1;
        }
        Ok(projects)
    }

    /// GET /projects/:id/repository/commits — 时间窗口内的提交（最多 limit 条）
    pub async fn fetch_project_commits(
        &self,
        project_id: i64,
        since_utc: &str,
        until_utc: &str,
        limit: usize,
    ) -> Result<Vec<GitlabCommit>, GitlabError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/projects/{project_id}/repository/commits")))
            .query(&[
                ("since", since_utc),
                ("until", until_utc),
                ("per_page", &limit.to_string()),
                ("stats", "true"),
            ])
            .header(self.auth_header(), &self.token)
            .send()
            .await
            .map_err(GitlabError::Http)?;
        let status = resp.status();
        let bytes = resp.bytes().await.map_err(GitlabError::Http)?;
        if !status.is_success() {
            return Err(GitlabError::Api(
                status.as_u16(),
                String::from_utf8_lossy(&bytes).into_owned(),
            ));
        }
        serde_json::from_slice(&bytes).map_err(GitlabError::Json)
    }

    /// 拉取当前用户于本地 offset 时区 `date` 当天的提交。
    ///
    /// 流程：GET /user → 有权限项目（可选允许列表）→ 逐项目拉 commits →
    /// 按作者与日期过滤 → 分项目截断、全局截断。
    pub async fn fetch_today_commits(
        &self,
        offset: FixedOffset,
        date: NaiveDate,
        allowlist: &[String],
    ) -> Result<GitlabFetchResult, GitlabError> {
        let user = self.fetch_current_user().await?;
        let projects = self.fetch_all_projects().await?;
        let selected = filter_projects_by_allowlist(&projects, allowlist);

        // 本日 00:00（本地）→ UTC
        let day_start_local = date.and_hms_opt(0, 0, 0).expect("有效的 00:00");
        let since_dt: DateTime<Utc> = offset
            .from_local_datetime(&day_start_local)
            .single()
            .expect("偏移换算必须唯一")
            .with_timezone(&Utc);
        let until_dt: DateTime<Utc> = Utc::now();

        let mut entries: Vec<GitlabCommitEntry> = Vec::new();
        for project in selected {
            let commits = match self
                .fetch_project_commits(
                    project.id,
                    &since_dt.to_rfc3339(),
                    &until_dt.to_rfc3339(),
                    PER_PROJECT_COMMIT_LIMIT,
                )
                .await
            {
                Ok(commits) => commits,
                Err(e) => {
                    // 部分失败不能静默：否则该项目提交无声丢失、无从排查
                    log::warn!("拉取项目 {} 提交失败，已跳过: {e}", project.id);
                    continue;
                }
            };
            for commit in commits {
                if !commit_belongs_to_user(&commit, &user) {
                    continue;
                }
                if !is_commit_on_local_date(&commit.committed_date, offset, date) {
                    continue;
                }
                entries.push(GitlabCommitEntry {
                    branch: project
                        .default_branch
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string()),
                    project: project.path_with_namespace.clone(),
                    commit,
                });
            }
        }
        let total = entries.len();
        let (entries, truncated) = cap_total_entries(entries);
        Ok(GitlabFetchResult {
            entries,
            truncated,
            total,
        })
    }
}

/// GitLab 调用错误（已本地化/可展示）
#[derive(Debug)]
pub enum GitlabError {
    /// 网络/传输错误
    Http(reqwest::Error),
    /// 非 2xx：HTTP 状态码 + 响应体片段
    Api(u16, String),
    /// JSON 解析失败
    Json(serde_json::Error),
}

impl std::fmt::Display for GitlabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitlabError::Http(e) => write!(f, "网络请求失败: {e}"),
            GitlabError::Api(status, body) => {
                write!(
                    f,
                    "GitLab API 返回 {status}: {}",
                    body.chars().take(120).collect::<String>()
                )
            }
            GitlabError::Json(e) => write!(f, "解析 GitLab 响应失败: {e}"),
        }
    }
}

impl std::error::Error for GitlabError {}

#[cfg(test)]
mod tests {
    use super::*;

    const UTC_PLUS_8: FixedOffset = match FixedOffset::east_opt(8 * 3600) {
        Some(o) => o,
        None => panic!("+08:00 offset"),
    };

    fn commit(
        id: &str,
        title: &str,
        author_email: &str,
        author_name: &str,
        committed_date: &str,
        stats: Option<(i64, i64)>,
    ) -> GitlabCommit {
        GitlabCommit {
            id: id.to_string(),
            short_id: id.chars().take(8).collect(),
            title: title.to_string(),
            message: title.to_string(),
            author_name: author_name.to_string(),
            author_email: author_email.to_string(),
            committed_date: committed_date.to_string(),
            stats: stats.map(|(a, d)| GitlabCommitStats {
                additions: a,
                deletions: d,
            }),
        }
    }

    fn entry(commit: GitlabCommit) -> GitlabCommitEntry {
        GitlabCommitEntry {
            commit,
            project: "work/test-proj".to_string(),
            branch: "main".to_string(),
        }
    }

    #[test]
    fn 解析提交json应兼容gitlab字段() {
        let json = r#"{
            "id": "abcdef1234567890",
            "short_id": "abcdef12",
            "title": "fix: 修复日报导出",
            "message": "fix: 修复日报导出\n\n详情",
            "author_name": "张三",
            "author_email": "zhangsan@example.com",
            "authored_date": "2026-09-02T01:00:00.000Z",
            "committed_date": "2026-09-02T01:00:00.000Z",
            "stats": {"additions": 12, "deletions": 3}
        }"#;
        let parsed: GitlabCommit = serde_json::from_str(json).expect("应能解析");
        assert_eq!(parsed.short_id, "abcdef12");
        assert_eq!(parsed.author_email, "zhangsan@example.com");
        let stats = parsed.stats.expect("stats 应存在");
        assert_eq!(stats.additions, 12);
        assert_eq!(stats.deletions, 3);
    }

    #[test]
    fn 解析提交json缺少stats应默认为空() {
        let json = r#"{
            "id": "abc",
            "short_id": "abc",
            "title": "t",
            "message": "t",
            "author_name": "n",
            "author_email": "e@x.com",
            "committed_date": "2026-09-02T01:00:00.000Z"
        }"#;
        let parsed: GitlabCommit = serde_json::from_str(json).expect("应能解析");
        assert_eq!(parsed.stats, None);
    }

    #[test]
    fn 解析项目json应兼容path字段() {
        let json = r#"{"id": 17, "path_with_namespace": "group/web", "default_branch": "main"}"#;
        let proj: GitlabProject = serde_json::from_str(json).unwrap();
        assert_eq!(proj.path_with_namespace, "group/web");
        assert_eq!(proj.default_branch.as_deref(), Some("main"));
    }

    #[test]
    fn 本地日期判断应处理跨时区边界() {
        // 09-02T16:30Z = 09-03 00:30 (+08)，属本地 09-03
        assert!(is_commit_on_local_date(
            "2026-09-02T16:30:00.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 3).unwrap()
        ));
        assert!(!is_commit_on_local_date(
            "2026-09-02T16:30:00.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 2).unwrap()
        ));
        // 09-02T00:01:00Z = 09-02 08:01 (+08)，属本地 09-02
        assert!(is_commit_on_local_date(
            "2026-09-02T00:01:00.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 2).unwrap()
        ));
        // 08-31T15:59:59Z = 08-31 23:59:59 (+08)，属本地 08-31
        assert!(is_commit_on_local_date(
            "2026-08-31T15:59:59.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        ));
        assert!(!is_commit_on_local_date(
            "2026-08-31T15:59:59.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        ));
        // 08-31T16:00:00Z = 09-01 00:00 (+08)，属本地 09-01
        assert!(is_commit_on_local_date(
            "2026-08-31T16:00:00.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        ));
        assert!(!is_commit_on_local_date(
            "2026-08-31T16:00:00.000Z",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 8, 31).unwrap()
        ));
    }

    #[test]
    fn 非法时间字符串应返回false() {
        assert!(!is_commit_on_local_date(
            "not-a-date",
            UTC_PLUS_8,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap()
        ));
    }

    #[test]
    fn 作者匹配应邮箱优先且忽略大小写() {
        let user = GitlabUser {
            id: 1,
            username: "zhangsan".into(),
            name: "Zhang San".into(),
            email: "Zhangsan@EXAMPLE.com".into(),
        };
        let c1 = commit(
            "a",
            "t",
            "zhangsan@example.com",
            "Other",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(commit_belongs_to_user(&c1, &user), "email 忽略大小写匹配");
        let c2 = commit(
            "b",
            "t",
            "other@example.com",
            "zhangsan",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(commit_belongs_to_user(&c2, &user), "name 匹配 username");
        let c3 = commit(
            "c",
            "t",
            "other@x.com",
            "Nobody",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(!commit_belongs_to_user(&c3, &user));
    }

    #[test]
    fn 作者匹配用户邮箱为空时应回退名字() {
        let user = GitlabUser {
            id: 1,
            username: "zhangsan".into(),
            name: "Zhang San".into(),
            email: String::new(),
        };
        let c = commit(
            "a",
            "t",
            "zhangsan@example.com",
            "Other",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(
            !commit_belongs_to_user(&c, &user),
            "邮箱不同且名字不同不应命中"
        );
        let c2 = commit(
            "b",
            "t",
            "whatever@x.com",
            "zhangsan",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(commit_belongs_to_user(&c2, &user), "按 username 命中");
    }

    #[test]
    fn 允许列表过滤应空列表全量且大小写不敏感() {
        let projects = vec![
            GitlabProject {
                id: 1,
                path_with_namespace: "Group/Web".into(),
                default_branch: Some("main".into()),
            },
            GitlabProject {
                id: 2,
                path_with_namespace: "group/mobile".into(),
                default_branch: None,
            },
            GitlabProject {
                id: 3,
                path_with_namespace: "other/service".into(),
                default_branch: None,
            },
        ];
        let all = filter_projects_by_allowlist(&projects, &[]);
        assert_eq!(all.len(), 3);
        let web = filter_projects_by_allowlist(&projects, &["group/web".to_string()]);
        assert_eq!(web.len(), 1);
        assert_eq!(web[0].id, 1, "大小写不敏感的精确匹配");
        let none = filter_projects_by_allowlist(&projects, &["group/we".to_string()]);
        assert_eq!(none.len(), 0, "部分路径不应命中");
        let two = filter_projects_by_allowlist(
            &projects,
            &["group/web".to_string(), "GROUP/MOBILE".to_string()],
        );
        assert_eq!(two.len(), 2);
    }

    #[test]
    fn 全局截断应保留前200条并标记() {
        let entries: Vec<GitlabCommitEntry> = (0..250)
            .map(|i| {
                entry(commit(
                    &format!("id{i:016}"),
                    &format!("title {i}"),
                    "a@b.c",
                    "A",
                    "2026-09-01T00:00:00Z",
                    None,
                ))
            })
            .collect();
        let (kept, truncated) = cap_total_entries(entries);
        assert_eq!(kept.len(), TOTAL_COMMIT_LIMIT);
        assert!(truncated);
        assert_eq!(kept[0].commit.title, "title 0");
        assert_eq!(
            kept.last().unwrap().commit.title,
            format!("title {}", TOTAL_COMMIT_LIMIT - 1)
        );
    }

    #[test]
    fn 少量提交不应截断() {
        let entries: Vec<GitlabCommitEntry> = (0..10)
            .map(|i| {
                entry(commit(
                    &format!("id{i:016}"),
                    "t",
                    "a@b.c",
                    "A",
                    "2026-09-01T00:00:00Z",
                    None,
                ))
            })
            .collect();
        let (kept, truncated) = cap_total_entries(entries);
        assert_eq!(kept.len(), 10);
        assert!(!truncated);
    }

    #[test]
    fn ai上下文空提交应为none() {
        assert_eq!(
            build_gitlab_ai_context(&[], false, 0, UTC_PLUS_8, AppLocale::ZhCn),
            None
        );
    }

    #[test]
    fn ai上下文应包含提交信息_统计与整合指令() {
        let c1 = commit(
            "abcdef123456",
            "fix: 修复日报导出",
            "a@b.c",
            "A",
            "2026-09-02T02:30:00.000Z",
            Some((12, 3)),
        );
        let c2 = commit(
            "11223344",
            "feat: 新增设置项",
            "a@b.c",
            "A",
            "2026-09-02T03:45:00.000Z",
            Some((30, 0)),
        );
        let entries = vec![entry(c1), entry(c2)];
        let ctx = build_gitlab_ai_context(&entries, false, 2, UTC_PLUS_8, AppLocale::ZhCn)
            .expect("非空提交应有上下文");
        assert!(ctx.contains("GitLab 今日提交"));
        assert!(ctx.contains("work/test-proj"));
        assert!(ctx.contains("main"));
        assert!(ctx.contains("fix: 修复日报导出"));
        assert!(ctx.contains("feat: 新增设置项"));
        assert!(ctx.contains("(+12/-3)"));
        assert!(ctx.contains("(+30/-0)"));
        assert!(ctx.contains("10:30"), "应展示本地时间 HH:mm（+8）");
        assert!(ctx.contains("整合"), "应含 AI 整合指令");
    }

    #[test]
    fn ai上下文截断时应提示剩余数量() {
        let all: Vec<GitlabCommitEntry> = (0..230)
            .map(|i| {
                entry(commit(
                    &format!("id{i:016}"),
                    &format!("t{i}"),
                    "a@b.c",
                    "A",
                    "2026-09-02T02:00:00.000Z",
                    None,
                ))
            })
            .collect();
        let (kept, truncated) = cap_total_entries(all);
        assert!(truncated);
        let ctx = build_gitlab_ai_context(&kept, truncated, 230, UTC_PLUS_8, AppLocale::ZhCn)
            .expect("有提交");
        assert!(ctx.contains("还有 30 条未展示"), "ctx: {ctx}");
    }

    #[test]
    fn ai上下文应按语言出标题() {
        let c = entry(commit(
            "abc",
            "t",
            "a@b.c",
            "A",
            "2026-09-02T02:00:00.000Z",
            None,
        ));
        let zh = build_gitlab_ai_context(
            std::slice::from_ref(&c),
            false,
            1,
            UTC_PLUS_8,
            AppLocale::ZhCn,
        )
        .unwrap();
        let tw = build_gitlab_ai_context(
            std::slice::from_ref(&c),
            false,
            1,
            UTC_PLUS_8,
            AppLocale::ZhTw,
        )
        .unwrap();
        let en = build_gitlab_ai_context(
            std::slice::from_ref(&c),
            false,
            1,
            UTC_PLUS_8,
            AppLocale::En,
        )
        .unwrap();
        let ar = build_gitlab_ai_context(
            std::slice::from_ref(&c),
            false,
            1,
            UTC_PLUS_8,
            AppLocale::Ar,
        )
        .unwrap();
        assert!(zh.contains("GitLab 今日提交"));
        assert!(tw.contains("GitLab 今日提交"));
        assert!(en.contains("GitLab Commits Today"));
        assert!(ar.contains("GitLab"));
    }

    #[test]
    fn 本地小节空提交应为空串() {
        assert_eq!(
            build_gitlab_local_section(&[], false, 0, UTC_PLUS_8, AppLocale::ZhCn),
            ""
        );
    }

    #[test]
    fn 本地小节应输出markdown列表() {
        let c = commit(
            "abcdef12",
            "docs: 更新说明",
            "a@b.c",
            "A",
            "2026-09-02T04:05:00.000Z",
            Some((2, 1)),
        );
        let section = build_gitlab_local_section(
            std::slice::from_ref(&entry(c)),
            false,
            1,
            UTC_PLUS_8,
            AppLocale::ZhCn,
        );
        assert!(section.contains("## GitLab 今日提交"));
        assert!(section.contains("- "));
        assert!(section.contains("docs: 更新说明"));
        assert!(section.contains("(+2/-1)"));
    }

    #[test]
    fn 本地小节截断应提示() {
        let all: Vec<GitlabCommitEntry> = (0..205)
            .map(|i| {
                entry(commit(
                    &format!("id{i:016}"),
                    "t",
                    "a@b.c",
                    "A",
                    "2026-09-02T02:00:00.000Z",
                    None,
                ))
            })
            .collect();
        let (kept, truncated) = cap_total_entries(all);
        assert!(truncated);
        let section =
            build_gitlab_local_section(&kept, truncated, 205, UTC_PLUS_8, AppLocale::ZhCn);
        assert!(section.contains("还有 5 条未展示"), "section: {section}");
    }

    #[test]
    fn 失败备注应按语言生成() {
        assert!(gitlab_failure_note(AppLocale::ZhCn).contains("GitLab"));
        assert!(gitlab_failure_note(AppLocale::En).contains("GitLab"));
        assert!(gitlab_failure_note(AppLocale::ZhTw).contains("GitLab"));
        assert!(gitlab_failure_note(AppLocale::Ar).contains("GitLab"));
        assert!(!gitlab_failure_note(AppLocale::ZhCn).is_empty());
    }

    #[test]
    fn 本地偏移应可用() {
        let offset = local_offset_now();
        assert!(offset.local_minus_utc().abs() < 24 * 3600);
    }

    #[test]
    fn fetch结果转补充内容应联动() {
        let c = commit(
            "abc",
            "t",
            "a@b.c",
            "A",
            "2026-09-02T02:00:00.000Z",
            Some((1, 2)),
        );
        let result = GitlabFetchResult {
            entries: vec![entry(c)],
            truncated: false,
            total: 1,
        };
        let supplement = result.to_supplement(UTC_PLUS_8, AppLocale::ZhCn);
        assert!(supplement.ai_context.is_some());
        assert!(!supplement.local_section.is_empty());
        assert_eq!(supplement.failure_note, None);
    }

    #[test]
    fn utc解析应保持与chrono一致() {
        let dt: DateTime<Utc> = chrono::DateTime::parse_from_rfc3339("2026-09-02T02:30:00.000Z")
            .unwrap()
            .with_timezone(&Utc);
        assert_eq!(dt.to_rfc3339(), "2026-09-02T02:30:00+00:00");
        let _ = UTC_PLUS_8;
        let _ = Utc::now();
    }

    #[test]
    fn api地址拼接应规范化base_url() {
        let client = GitlabClient::new("https://git.example.com/", "t");
        assert_eq!(
            client.api_url("/user"),
            "https://git.example.com/api/v4/user"
        );
        let plain = GitlabClient::new("https://git.example.com", "t");
        assert_eq!(
            plain.api_url("/projects"),
            "https://git.example.com/api/v4/projects"
        );
    }
}
