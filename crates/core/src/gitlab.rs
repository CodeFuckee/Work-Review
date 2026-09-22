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
/// 并发补拉提交行数统计的批量大小（详情接口逐条取，批间串行）
pub const COMMIT_STATS_CONCURRENCY: usize = 8;
/// 补拉提交行数统计的总预算（秒）：超预算的提交保持无统计（概览统计按 0 计）
pub const COMMIT_STATS_BUDGET_SECS: u64 = 10;
/// 作者选择器最多扫描的仓库数（未配允许列表时防止全量遍历失控）
pub const MAX_CONTRIBUTOR_PROJECTS: usize = 200;
/// 每仓库最多拉取的贡献者条数（按提交数倒序取前 N）
pub const PER_PROJECT_CONTRIBUTOR_LIMIT: usize = 100;

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
    /// 供 LLM 的上下文（含提交列表与"可引用、不单列小节"指令）；无提交时为 None
    pub ai_context: Option<String>,
    /// 「GitLab 今日提交」独立小节的 markdown（含无编号标题）。
    /// 已配置即非空：有提交 = 提交列表；无提交 = 占位说明；拉取失败 = 失败说明。
    /// 空串 = 未配置（不生成小节）。位置由分析器的块系统锚定在按小时活跃度之前。
    pub local_section: String,
    /// 原始提交条目（时间线交错渲染与入库用）；无提交/失败时为空
    pub entries: Vec<GitlabCommitEntry>,
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

/// 提交是否命中纳入作者名单：每条名单同时与 author_email 和 author_name
/// 忽略大小写精确比对（名单条目容忍首尾空白；精确相等，杜绝子串与
/// contains("") 式恒真命中）。名单为空返回 false。
pub fn commit_matches_work_author(commit: &GitlabCommit, work_authors: &[String]) -> bool {
    let commit_email = commit.author_email.trim().to_lowercase();
    let commit_name = commit.author_name.trim().to_lowercase();
    if commit_email.is_empty() && commit_name.is_empty() {
        return false;
    }
    work_authors
        .iter()
        .map(|entry| entry.trim().to_lowercase())
        .filter(|entry| !entry.is_empty())
        .any(|entry| commit_email == entry || commit_name == entry)
}

/// 提交是否在日报统计范围内：token 账号本人，或纳入作者名单（并集）。
pub fn commit_in_report_scope(
    commit: &GitlabCommit,
    user: &GitlabUser,
    work_authors: &[String],
) -> bool {
    commit_belongs_to_user(commit, user) || commit_matches_work_author(commit, work_authors)
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

/// contributors API 返回的单仓库贡献者条目
/// （字段名与 GitLab v4 REST 保持一致；大仓库可能缺 additions/deletions，不取）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitlabContributor {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub commits: i64,
}

/// 作者选择器候选（每个邮箱一行）
#[derive(Debug, Clone, PartialEq)]
pub struct GitlabAuthorCandidate {
    pub name: String,
    pub email: String,
    pub commits: i64,
    pub is_current_user: bool,
}

/// 贡献者是否 token 账号本人：邮箱精确（忽略大小写）匹配优先，
/// token 账号不暴露邮箱时退回姓名/用户名匹配（与提交归属判定同思路）。
pub fn contributor_is_current_user(contributor: &GitlabContributor, user: &GitlabUser) -> bool {
    let contributor_email = contributor.email.trim().to_lowercase();
    let user_email = user.email.trim().to_lowercase();
    if !contributor_email.is_empty() && !user_email.is_empty() {
        return contributor_email == user_email;
    }
    let contributor_name = contributor.name.trim().to_lowercase();
    if contributor_name.is_empty() {
        return false;
    }
    contributor_name == user.name.trim().to_lowercase()
        || contributor_name == user.username.trim().to_lowercase()
}

/// 聚合多仓库贡献者为作者候选：每个邮箱一行（同名多邮箱保留多行，见
/// 作者选择器交互约定），提交数累加、姓名取提交数最多的写法；
/// 空/空白邮箱丢弃（名单只存邮箱，无邮箱条目无法命中提交）。
/// 排序：提交数倒序、其次姓名/邮箱升序，保证展示稳定。
pub fn aggregate_author_candidates(
    per_project: &[Vec<GitlabContributor>],
    current_user: &GitlabUser,
) -> Vec<GitlabAuthorCandidate> {
    struct Accumulator {
        best_name: String,
        best_name_commits: i64,
        total_commits: i64,
    }
    let mut by_email: std::collections::HashMap<String, Accumulator> =
        std::collections::HashMap::new();
    for contributors in per_project {
        for contributor in contributors {
            let email = contributor.email.trim().to_lowercase();
            if email.is_empty() {
                continue;
            }
            let commits = contributor.commits.max(0);
            let entry = by_email.entry(email).or_insert(Accumulator {
                best_name: contributor.name.trim().to_string(),
                best_name_commits: commits,
                total_commits: 0,
            });
            entry.total_commits += commits;
            if commits > entry.best_name_commits {
                entry.best_name = contributor.name.trim().to_string();
                entry.best_name_commits = commits;
            }
        }
    }
    let mut candidates: Vec<GitlabAuthorCandidate> = by_email
        .into_iter()
        .map(|(email, acc)| {
            let contributor = GitlabContributor {
                name: acc.best_name.clone(),
                email: email.clone(),
                commits: acc.total_commits,
            };
            GitlabAuthorCandidate {
                name: acc.best_name,
                email,
                commits: acc.total_commits,
                is_current_user: contributor_is_current_user(&contributor, current_user),
            }
        })
        .collect();
    candidates.sort_by(|a, b| {
        b.commits
            .cmp(&a.commits)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
            .then_with(|| a.email.cmp(&b.email))
    });
    candidates
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

/// 构建「GitLab 今日提交」独立小节 markdown（含无编号标题，块系统会统一编号）。
/// 已配置即始终产出小节：有提交 = 提交列表（超限附截断提示）；
/// 无提交 = 占位说明，不再静默省略（与"配置了就始终显示小节"的产品约定一致）。
pub fn build_gitlab_local_section(
    entries: &[GitlabCommitEntry],
    truncated: bool,
    total: usize,
    offset: FixedOffset,
    locale: AppLocale,
) -> String {
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
    if lines.is_empty() {
        lines.push(gitlab_empty_hint(locale));
    }
    format!(
        "## {}\n\n{}",
        gitlab_section_title(locale),
        lines.join("\n")
    )
}

/// 拉取失败时的「GitLab 今日提交」小节（失败说明内嵌小节，不再文末追加备注行）。
pub fn gitlab_unavailable_section(locale: AppLocale) -> String {
    format!(
        "## {}\n\n{}",
        gitlab_section_title(locale),
        gitlab_failure_note(locale)
    )
}

/// 拉取失败时的说明行（按 locale；内嵌进小节，不再文末追加）
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

/// 剥离 API 返回文本中的 C0 控制字符（换行/制表等，替换为空格）。
/// 提交标题/项目路径/分支进入日报 markdown 与 AI 上下文时必须保持单行，
/// 否则被劫持的服务端可用 `\n## …` 注入伪造小节；真实 git 提交标题不含
/// 控制字符，本清洗对正常输入是恒等变换。
fn strip_control_chars(text: &str) -> String {
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

/// 无提交时的占位说明（按 locale）
fn gitlab_empty_hint(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => "今日无 GitLab 提交记录。".to_string(),
        AppLocale::ZhTw => "今日無 GitLab 提交記錄。".to_string(),
        AppLocale::En => "No GitLab commits today.".to_string(),
        AppLocale::Ar => "لا توجد تسليمات GitLab اليوم.".to_string(),
    }
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
    // 小节标题从 gitlab_section_title 注入（单一来源），避免双份维护漂移
    let title = gitlab_section_title(locale);
    match locale {
        AppLocale::ZhCn => {
            format!("上述 GitLab 提交清单已在日报的「{title}」小节中完整展示（该小节由模板固定生成）。你撰写日报分析时可以自然引用这些代码工作（实现了什么功能、修复了什么问题、做了哪些重构或配置调整），但不要在分析中单列「{title}」小节，也不要逐条复述提交清单；同样不得编造清单之外的提交。")
        }
        AppLocale::ZhTw => {
            format!("上述 GitLab 提交清單已在日報的「{title}」小節中完整展示（該小節由模板固定生成）。你撰寫日報分析時可以自然引用這些程式碼工作（實現了什麼功能、修復了什麼問題、做了哪些重構或配置調整），但不要在分析中單列「{title}」小節，也不要逐條複述提交清單；同樣不得虛構清單之外的提交。")
        }
        AppLocale::En => {
            format!("The GitLab commits above are already shown in full in the report's \"{title}\" section (generated by the template). When writing the report analysis, feel free to reference this coding work naturally (features implemented, bugs fixed, refactors or config changes), but do not add a separate \"{title}\" section of your own and do not restate the commit list item by item; likewise, do not invent commits not listed.")
        }
        AppLocale::Ar => {
            format!("قائمة تسليمات GitLab أعلاه معروضة بالكامل في قسم «{title}» من التقرير (ينشأ هذا القسم من القالب تلقائيًا). عند كتابة تحليل التقرير يمكنك الإشارة إلى هذا العمل البرمجي بشكل طبيعي (ما نُفِّذ من ميزات، ما أُصلح من أخطاء، أعمال إعادة الهيكلة أو تعديلات الإعداد)، لكن لا تنشئ قسم «{title}» بنفسك ولا تعِد سرد القائمة بندًا بندًا؛ ولا تخترع تسليمات غير مذكورة كذلك.")
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
            entries: self.entries.clone(),
        }
    }
}

/// 把提交条目转成时间线入库行。`date` 为本机时区日期（YYYY-MM-DD）；
/// committed_date 解析失败的条目跳过（时间线需要有效时间戳排序）。
pub fn entries_to_rows(
    entries: &[GitlabCommitEntry],
    date: &str,
) -> Vec<crate::database::GitlabCommitRow> {
    entries
        .iter()
        .filter_map(|e| {
            let dt = DateTime::parse_from_rfc3339(&e.commit.committed_date).ok()?;
            Some(crate::database::GitlabCommitRow {
                sha: e.commit.id.clone(),
                date: date.to_string(),
                committed_at: dt.timestamp(),
                project: e.project.clone(),
                branch: e.branch.clone(),
                short_id: e.commit.short_id.clone(),
                title: e.commit.title.clone(),
                additions: e.commit.stats.as_ref().map(|s| s.additions),
                deletions: e.commit.stats.as_ref().map(|s| s.deletions),
                author_name: e.commit.author_name.clone(),
                // fetch 侧 strip 未覆盖该字段（入库行单行化不变量补齐，对合法邮箱恒等）
                author_email: strip_control_chars(&e.commit.author_email),
            })
        })
        .collect()
}

/// GitLab REST 客户端。
///
/// 设计为可注入 HTTP client（默认 `reqwest::Client`），保持薄封装：
/// 只负责 URL 拼接、分页与 JSON 反序列化，业务判断全部在纯函数中。
/// Clone 供并发拉取多仓库贡献者时分发到 tokio 任务。
#[derive(Clone)]
pub struct GitlabClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl GitlabClient {
    pub fn new(base_url: &str, token: &str) -> Self {
        // 必须显式设超时：reqwest 默认无超时，GitLab 挂起时日报生成会无限阻塞。
        // 构建失败（TLS 后端初始化失败）不能静默回退为无超时 Client——那会把
        // 快速失败变成永久挂起，这里直接 panic 暴露环境问题。
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("reqwest Client 初始化失败（TLS 后端不可用）");
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

    /// GET /projects/:id/repository/commits/:sha — 单条提交详情。
    /// GitLab 的列表接口不返回 stats（stats=true 被忽略），行数统计必须逐条取详情；
    /// merge/空提交等可能没有 stats，解析为 None。
    pub async fn fetch_commit_stats(
        &self,
        project_id: i64,
        sha: &str,
    ) -> Result<Option<GitlabCommitStats>, GitlabError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/projects/{project_id}/repository/commits/{sha}")))
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
        let detail: GitlabCommit = serde_json::from_slice(&bytes).map_err(GitlabError::Json)?;
        Ok(detail.stats)
    }

    /// GET /projects/:id/repository/contributors — 默认分支贡献者
    /// （与日报统计口径一致：日报 commits 接口同样只看默认分支）。
    /// 按提交数倒序取前 `PER_PROJECT_CONTRIBUTOR_LIMIT` 条。
    pub async fn fetch_project_contributors(
        &self,
        project_id: i64,
    ) -> Result<Vec<GitlabContributor>, GitlabError> {
        let resp = self
            .http
            .get(self.api_url(&format!("/projects/{project_id}/repository/contributors")))
            .query(&[
                ("order_by", "commits"),
                ("sort", "desc"),
                ("per_page", &PER_PROJECT_CONTRIBUTOR_LIMIT.to_string()),
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

    /// 拉取（允许列表过滤后的）仓库贡献者并聚合为作者候选，供设置页作者选择器。
    /// - 允许列表为空 = 全部有权限项目；项目数超过 `MAX_CONTRIBUTOR_PROJECTS`
    ///   时按路径排序取前 N 个并返回 truncated 标记。
    /// - 逐仓库失败仅跳过（log::warn），与 fetch_today_commits 的部分失败策略一致。
    /// - 并发拉取（信号量限 8），避免多仓库时串行过慢。
    pub async fn fetch_author_candidates(
        &self,
        allowlist: &[String],
    ) -> Result<(Vec<GitlabAuthorCandidate>, bool), GitlabError> {
        let user = self.fetch_current_user().await?;
        let projects = self.fetch_all_projects().await?;
        let mut selected: Vec<GitlabProject> = filter_projects_by_allowlist(&projects, allowlist)
            .into_iter()
            .cloned()
            .collect();
        // 截断前按路径排序，保证“前 N 个”结果稳定可复现
        selected.sort_by_cached_key(|p| p.path_with_namespace.to_lowercase());
        let truncated = selected.len() > MAX_CONTRIBUTOR_PROJECTS;
        if truncated {
            selected.truncate(MAX_CONTRIBUTOR_PROJECTS);
        }

        let client = std::sync::Arc::new(self.clone());
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));
        let mut handles = Vec::with_capacity(selected.len());
        for project in selected {
            let permit = semaphore
                .clone()
                .acquire_owned()
                .await
                .map_err(|e| GitlabError::Api(0, format!("信号量获取失败: {e}")))?;
            let client = client.clone();
            handles.push(tokio::spawn(async move {
                // permit 活到请求结束：限制同时在飞的贡献者请求数
                let _permit = permit;
                let contributors = client.fetch_project_contributors(project.id).await;
                (project.path_with_namespace, contributors)
            }));
        }
        let mut per_project: Vec<Vec<GitlabContributor>> = Vec::with_capacity(handles.len());
        for handle in handles {
            match handle.await {
                Ok((_path, Ok(contributors))) => per_project.push(contributors),
                Ok((path, Err(e))) => {
                    // 部分失败不能静默：否则该仓库作者无声丢失、无从排查
                    log::warn!("拉取仓库 {path} 贡献者失败，已跳过: {e}");
                }
                Err(e) => {
                    log::warn!("拉取仓库贡献者任务异常，已跳过: {e}");
                }
            }
        }
        Ok((aggregate_author_candidates(&per_project, &user), truncated))
    }

    /// 拉取当前用户与纳入作者名单在本地 offset 时区 `date` 当天的提交。
    ///
    /// 流程：GET /user → 有权限项目（可选允许列表）→ 逐项目拉 commits →
    /// 按作者（本人 ∪ 名单）与日期过滤 → 分项目截断、全局截断。
    pub async fn fetch_today_commits(
        &self,
        offset: FixedOffset,
        date: NaiveDate,
        allowlist: &[String],
        work_authors: &[String],
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
        let project_id_by_path: std::collections::HashMap<String, i64> = selected
            .iter()
            .map(|project| (project.path_with_namespace.clone(), project.id))
            .collect();
        for project in &selected {
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
                if !commit_in_report_scope(&commit, &user, work_authors) {
                    continue;
                }
                if !is_commit_on_local_date(&commit.committed_date, offset, date) {
                    continue;
                }
                entries.push(GitlabCommitEntry {
                    branch: strip_control_chars(
                        &project
                            .default_branch
                            .clone()
                            .unwrap_or_else(|| "HEAD".to_string()),
                    ),
                    project: strip_control_chars(&project.path_with_namespace),
                    // 标题/作者字段会拼进日报 markdown 与入库行，进入前剥掉
                    // 控制字符保证单行（防 markdown 结构注入，见 strip_control_chars）
                    commit: GitlabCommit {
                        title: strip_control_chars(&commit.title),
                        author_name: strip_control_chars(&commit.author_name),
                        ..commit
                    },
                });
            }
        }
        let total = entries.len();
        // GitLab 列表接口不返回 stats，逐条补拉详情（有总预算，超时保持无统计）
        self.enrich_commit_stats(&project_id_by_path, &mut entries)
            .await;
        let (entries, truncated) = cap_total_entries(entries);
        Ok(GitlabFetchResult {
            entries,
            truncated,
            total,
        })
    }

    /// 并发补拉提交详情的行数统计（列表接口不返回 stats，只有详情接口有）。
    /// 每批 `COMMIT_STATS_CONCURRENCY` 条；总预算 `COMMIT_STATS_BUDGET_SECS` 秒，
    /// 超预算或单条失败的提交保持 None（统计按 0 计），不影响其余数据。
    async fn enrich_commit_stats(
        &self,
        project_id_by_path: &std::collections::HashMap<String, i64>,
        entries: &mut [GitlabCommitEntry],
    ) {
        let pending: Vec<(usize, i64, String)> = entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.commit.stats.is_none())
            .filter_map(|(index, entry)| {
                project_id_by_path
                    .get(&entry.project)
                    .map(|project_id| (index, *project_id, entry.commit.id.clone()))
            })
            .collect();
        if pending.is_empty() {
            return;
        }

        let deadline =
            tokio::time::Instant::now() + std::time::Duration::from_secs(COMMIT_STATS_BUDGET_SECS);
        for batch in pending.chunks(COMMIT_STATS_CONCURRENCY) {
            if tokio::time::Instant::now() >= deadline {
                log::warn!(
                    "提交行数统计补拉超预算（{}s），剩余 {} 条按无统计处理",
                    COMMIT_STATS_BUDGET_SECS,
                    batch.len()
                );
                break;
            }
            let mut handles = Vec::with_capacity(batch.len());
            for (_, project_id, sha) in batch {
                handles.push(tokio::spawn({
                    let client = self.clone();
                    let project_id = *project_id;
                    let sha = sha.clone();
                    async move { client.fetch_commit_stats(project_id, &sha).await }
                }));
            }
            for ((index, _, sha), handle) in batch.iter().zip(handles) {
                match handle.await {
                    Ok(Ok(Some(stats))) => {
                        entries[*index].commit.stats = Some(stats);
                    }
                    Ok(Ok(None)) => {}
                    Ok(Err(e)) => log::warn!("拉取提交 {sha} 行数统计失败: {e}"),
                    Err(e) => log::warn!("提交行数统计任务异常: {e}"),
                }
            }
        }
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
    fn 入库行应对author_email剥控制字符保持单行不变量() {
        // fetch 侧 strip 只覆盖 title/author_name/project/branch；
        // author_email 在入库行补齐同一清洗（对合法邮箱恒等）
        let mut malicious = commit(
            "abc123",
            "fix: 标题",
            "evil@example.com\n## 伪造小节",
            "张三",
            "2026-09-02T01:00:00.000Z",
            None,
        );
        // fetch 侧清洗后 title/author_name 已单行，email 原样进入 entries
        malicious.title = strip_control_chars(&malicious.title);
        malicious.author_name = strip_control_chars(&malicious.author_name);

        let rows = entries_to_rows(&[entry(malicious)], "2026-09-02");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].author_email, "evil@example.com ## 伪造小节",
            "控制字符应被替换为空格，入库行不得携带换行"
        );
        assert!(!rows[0].author_email.contains('\n'));
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
    fn 名单作者应命中邮箱或姓名() {
        let c = commit(
            "a",
            "t",
            "lisi@CORP.com",
            "李四",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(
            commit_matches_work_author(&c, &["lisi@corp.com".into()]),
            "条目按邮箱命中（忽略大小写）"
        );
        assert!(
            commit_matches_work_author(&c, &["李四".into()]),
            "条目按姓名命中"
        );
        assert!(
            commit_matches_work_author(&c, &["wangwu@x.com".into(), "  LISI@Corp.Com  ".into()]),
            "名单中任一条目命中即可，且容忍首尾空白"
        );
        assert!(
            !commit_matches_work_author(&c, &["wangwu@x.com".into()]),
            "不在名单不应命中"
        );
    }

    #[test]
    fn 名单作者空名单不应命中() {
        let c = commit(
            "a",
            "t",
            "lisi@corp.com",
            "李四",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(!commit_matches_work_author(&c, &[]), "空名单 = 不扩展范围");
    }

    #[test]
    fn 名单作者不应部分匹配且空条目不应命中() {
        let c = commit(
            "a",
            "t",
            "lisi@corp.com",
            "李四",
            "2026-09-01T00:00:00Z",
            None,
        );
        assert!(
            !commit_matches_work_author(&c, &["li".into()]),
            "子串不算命中（精确相等语义）"
        );
        assert!(!commit_matches_work_author(&c, &["李".into()]));
        assert!(
            !commit_matches_work_author(&c, &["".into(), "   ".into()]),
            "空条目不得出现 contains 式恒真命中"
        );
        let anon = commit("b", "t", "", "", "2026-09-01T00:00:00Z", None);
        assert!(
            !commit_matches_work_author(&anon, &["lisi@corp.com".into()]),
            "提交缺作者信息不应命中"
        );
    }

    #[test]
    fn 统计范围应为本人与名单作者并集() {
        let user = GitlabUser {
            id: 1,
            username: "zhangsan".into(),
            name: "Zhang San".into(),
            email: "zhangsan@example.com".into(),
        };
        let own = commit(
            "a",
            "t",
            "zhangsan@example.com",
            "Zhang San",
            "2026-09-01T00:00:00Z",
            None,
        );
        let listed = commit(
            "b",
            "t",
            "lisi@corp.com",
            "李四",
            "2026-09-01T00:00:00Z",
            None,
        );
        let other = commit(
            "c",
            "t",
            "wangwu@x.com",
            "王五",
            "2026-09-01T00:00:00Z",
            None,
        );
        let empty: Vec<String> = Vec::new();
        assert!(
            commit_in_report_scope(&own, &user, &empty),
            "空名单 = 仅本人（现状）"
        );
        assert!(!commit_in_report_scope(&listed, &user, &empty));
        assert!(!commit_in_report_scope(&other, &user, &empty));
        let authors = vec!["李四".to_string()];
        assert!(
            commit_in_report_scope(&own, &user, &authors),
            "本人始终在范围内"
        );
        assert!(
            commit_in_report_scope(&listed, &user, &authors),
            "名单作者命中"
        );
        assert!(!commit_in_report_scope(&other, &user, &authors));
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
        assert!(
            ctx.contains("不要在分析中单列"),
            "应含「可引用但不单列小节」的整合指令"
        );
    }

    #[test]
    fn 整合指令应要求不单列小节且保留防编造约束() {
        let guards = [
            (AppLocale::ZhCn, "不得编造", "不要在分析中单列"),
            (AppLocale::ZhTw, "不得虛構", "不要在分析中單列"),
            (AppLocale::En, "not invent", "do not add a separate"),
            (AppLocale::Ar, "ولا تخترع", "لا تنشئ قسم"),
        ];
        for (locale, guard, no_section) in guards {
            let instruction = integration_instruction(locale);
            assert!(
                instruction.contains(gitlab_section_title(locale)),
                "指令应固定小节标题（{locale:?}）"
            );
            assert!(
                instruction.contains(guard),
                "指令应保留防编造约束（{locale:?}）"
            );
            assert!(
                instruction.contains(no_section),
                "指令应明确禁止 AI 单列小节（{locale:?}）"
            );
        }
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
    fn 本地小节空提交应输出占位说明() {
        let section = build_gitlab_local_section(&[], false, 0, UTC_PLUS_8, AppLocale::ZhCn);
        assert!(
            section.contains("## GitLab 今日提交"),
            "空提交也应带小节标题: {section}"
        );
        assert!(
            section.contains("今日无 GitLab 提交记录"),
            "空提交应显示占位说明: {section}"
        );
        let en = build_gitlab_local_section(&[], false, 0, UTC_PLUS_8, AppLocale::En);
        assert!(en.contains("No GitLab commits today."), "en: {en}");
    }

    #[test]
    fn 控制字符剥离应保证提交文本单行且对正常输入恒等() {
        // 正常输入是恒等变换
        assert_eq!(
            strip_control_chars("fix: 修复登录页崩溃"),
            "fix: 修复登录页崩溃"
        );
        assert_eq!(
            strip_control_chars("[core] handle x (closes #12)"),
            "[core] handle x (closes #12)"
        );
        // 换行/回车/制表注入 markdown 结构 → 折叠为单行
        assert_eq!(
            strip_control_chars("正常标题\n## 伪造小节"),
            "正常标题 ## 伪造小节"
        );
        assert_eq!(strip_control_chars("a\r\nb\tc"), "a b c");
        assert_eq!(strip_control_chars("\u{0}\u{1f}x"), " x");
        // 连续控制字符不产生连续空格
        assert_eq!(strip_control_chars("a\n\n\nb"), "a b");
        // 提交行拼装后不再含任何换行（防小节/列表注入）：fetch 侧（fetch_today_commits）
        // 构造条目前已按 strip_control_chars 清洗标题/项目/分支，此处复现同款清洗再拼行
        let raw = commit(
            "abc123",
            "恶意标题\n## 伪造小节\n- 项",
            "evil@example.com",
            " attacker ",
            "2026-09-17T02:30:00Z",
            None,
        );
        let cleaned = GitlabCommit {
            title: strip_control_chars(&raw.title),
            author_name: strip_control_chars(&raw.author_name),
            ..raw
        };
        let line = format_commit_line(&entry(cleaned), UTC_PLUS_8, AppLocale::ZhCn, false);
        assert!(!line.contains('\n'), "提交行必须单行: {line:?}");
    }

    #[test]
    fn 失败小节应内嵌失败说明() {
        let section = gitlab_unavailable_section(AppLocale::ZhCn);
        assert!(section.contains("## GitLab 今日提交"), "section: {section}");
        assert!(
            section.contains("GitLab 提交获取失败"),
            "section: {section}"
        );
        let tw = gitlab_unavailable_section(AppLocale::ZhTw);
        assert!(tw.contains("GitLab 提交獲取失敗"), "tw: {tw}");
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
        assert!(!section.contains("（1）"), "标题不再带计数后缀: {section}");
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
        assert!(supplement.local_section.contains("## GitLab 今日提交"));
    }

    #[test]
    fn fetch空结果转补充内容应带占位小节且无ai上下文() {
        let result = GitlabFetchResult::default();
        let supplement = result.to_supplement(UTC_PLUS_8, AppLocale::ZhCn);
        assert!(supplement.ai_context.is_none(), "无提交不应有 AI 上下文");
        assert!(
            supplement.local_section.contains("今日无 GitLab 提交记录"),
            "空结果应产出占位小节: {}",
            supplement.local_section
        );
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

    #[test]
    fn 贡献者json应按gitlab字段名解析() {
        let payload = r#"[
            {"name":"张三","email":"zhangsan@corp.com","commits":12,"additions":100,"deletions":50},
            {"name":"lisi","email":"lisi@corp.com","commits":3}
        ]"#;
        let contributors: Vec<GitlabContributor> = serde_json::from_str(payload).unwrap();
        assert_eq!(contributors.len(), 2);
        assert_eq!(contributors[0].name, "张三");
        assert_eq!(contributors[0].commits, 12);
        assert_eq!(contributors[1].email, "lisi@corp.com");
    }

    #[test]
    fn 作者聚合应按邮箱去重累加并按提交数倒序() {
        let user = GitlabUser {
            id: 1,
            username: "chenkaidi".to_string(),
            name: "chenkaidi".to_string(),
            email: "me@corp.com".to_string(),
        };
        let per_project = vec![
            vec![
                GitlabContributor {
                    name: "李四".into(),
                    email: "Lisi@Corp.com".into(),
                    commits: 5,
                },
                GitlabContributor {
                    name: "李四".into(),
                    email: "lisi@corp.com".into(),
                    commits: 2,
                },
            ],
            vec![
                // 同一邮箱换项目再出现：提交数累加，姓名保持提交数最多的写法
                GitlabContributor {
                    name: "lisi".into(),
                    email: "lisi@corp.com".into(),
                    commits: 9,
                },
                // 同名不同邮箱：保留为两行
                GitlabContributor {
                    name: "李四".into(),
                    email: "lisi-other@corp.com".into(),
                    commits: 4,
                },
                // 空/空白邮箱：丢弃
                GitlabContributor {
                    name: "无邮箱".into(),
                    email: "  ".into(),
                    commits: 7,
                },
            ],
        ];
        let candidates = aggregate_author_candidates(&per_project, &user);
        assert_eq!(candidates.len(), 2, "大小写去重后剩 2 个邮箱");
        assert_eq!(candidates[0].email, "lisi@corp.com");
        assert_eq!(candidates[0].commits, 16, "同邮箱跨仓库提交数累加");
        assert_eq!(candidates[0].name, "lisi", "姓名取提交数最多的写法");
        assert!(!candidates[0].is_current_user);
        assert_eq!(candidates[1].email, "lisi-other@corp.com");
        assert_eq!(candidates[1].commits, 4);
    }

    #[test]
    fn 作者聚合应标记当前账号且提交数倒序并列时按姓名升序() {
        let user = GitlabUser {
            id: 1,
            username: "chenkaidi".to_string(),
            name: "chenkaidi".to_string(),
            email: "Me@Corp.com".to_string(),
        };
        let per_project = vec![vec![
            GitlabContributor {
                name: "cjy66668888".into(),
                email: "ME@corp.com".into(),
                commits: 30,
            },
            GitlabContributor {
                name: "王五".into(),
                email: "wangwu@corp.com".into(),
                commits: 30,
            },
            GitlabContributor {
                name: "赵六".into(),
                email: "zhaoliu@corp.com".into(),
                commits: 1,
            },
        ]];
        let candidates = aggregate_author_candidates(&per_project, &user);
        assert_eq!(candidates.len(), 3);
        assert!(candidates[0].is_current_user, "邮箱忽略大小写命中当前账号");
        assert_eq!(candidates[0].name, "cjy66668888");
        // 并列提交数时按姓名小写升序：wangwu < 赵六(cjy 的 is_current_user 不参与排序权重)
        assert_eq!(candidates[1].email, "wangwu@corp.com");
        assert!(!candidates[1].is_current_user);
        assert_eq!(candidates[2].email, "zhaoliu@corp.com");
    }

    #[test]
    fn 当前账号判定在无账号邮箱时退回姓名匹配() {
        let user = GitlabUser {
            id: 1,
            username: "chenkaidi".to_string(),
            name: "chenkaidi".to_string(),
            email: String::new(),
        };
        let by_name = GitlabContributor {
            name: "ChenKaidi".into(),
            email: "x@corp.com".into(),
            commits: 1,
        };
        let by_username = GitlabContributor {
            name: "someone".into(),
            email: "y@corp.com".into(),
            commits: 1,
        };
        assert!(
            contributor_is_current_user(&by_name, &user),
            "姓名忽略大小写命中"
        );
        assert!(!contributor_is_current_user(&by_username, &user));
    }
}
