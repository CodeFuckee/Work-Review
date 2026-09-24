//! 日报"动态统计区块"占位符与读时替换 + 段落积木化
//!
//! 解决 issue #80 / #79.cmt3：保存的日报 markdown 里固化的时长数字会变陈旧（自动生成
//! 触发后用户继续工作的活动不会被回填）。读取/展示时使用最新的 DailyStats 重新渲染
//! 这些区块，保证每次看到的数字都是当前的。
//!
//! 段落积木化（方向 A）：每个统计区块是一个 `StatsBlock` 枚举变体，`assemble()`
//! 按调用方提供的顺序依次渲染 + wrap_block，输出带标记的 markdown 字符串。
//! 调用方（summary.rs / local.rs）不再逐个 push_str，而是构造 block 数组。

use crate::analysis::{
    format_duration_for_locale, generate_hourly_activity_summary_for_locale,
    translate_category_name, translate_semantic_category_name, AppLocale,
};
use crate::database::{DailyStats, DomainUsage, PlaneWorkItemRow};
use std::collections::HashMap;

pub const BLOCK_CATEGORY_TABLE: &str = "CATEGORY_TABLE";
pub const BLOCK_APP_USAGE_TABLE: &str = "APP_USAGE_TABLE";
pub const BLOCK_HOURLY_SUMMARY: &str = "HOURLY_SUMMARY";
pub const BLOCK_DOMAIN_USAGE_TABLE: &str = "DOMAIN_USAGE_TABLE";
pub const BLOCK_AI_ANALYSIS: &str = "AI_ANALYSIS";
pub const BLOCK_LOCAL_OVERVIEW: &str = "LOCAL_OVERVIEW";
pub const BLOCK_LOCAL_CATEGORY: &str = "LOCAL_CATEGORY";
pub const BLOCK_LOCAL_APP_USAGE: &str = "LOCAL_APP_USAGE";
pub const BLOCK_LOCAL_DOMAIN_USAGE: &str = "LOCAL_DOMAIN_USAGE";
pub const BLOCK_GITLAB_TODAY: &str = "GITLAB_TODAY";
pub const BLOCK_PLANE_TODAY: &str = "PLANE_TODAY";

/// 段落积木枚举——每个变体对应一个可独立渲染的统计区块。
/// 调用方（summary.rs / local.rs）构造 `Vec<StatsBlock>` 传入 `assemble()`，
/// 由 assemble 按顺序渲染 + wrap_block，输出带标记的 markdown。
///
/// AI 分析段和活动时间线不纳入此枚举——它们不是 stats 驱动的纯函数，
/// 由调用方在 assemble 前后直接插入。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatsBlock {
    /// summary 模式：时间分配表格
    CategoryTable,
    /// summary 模式：应用使用明细表格
    AppUsageTable,
    /// summary + local：按小时活跃度
    HourlySummary,
    /// summary 模式：网站访问明细表格
    DomainUsageTable,
    /// local 模式：今日概览
    LocalOverview,
    /// local 模式：时间分配列表
    LocalCategory,
    /// local 模式：应用使用列表
    LocalAppUsage,
    /// local 模式：网站访问列表
    LocalDomainUsage,
    /// GitLab 今日提交（内容来自日报补充的 pre-rendered markdown，经
    /// assemble 的 `gitlab_markdown` 参数注入，非 stats 驱动；未配置时为
    /// 空串、wrap_block 自动跳过该块）
    GitlabToday,
    /// Plane 工作项（同 GitlabToday：pre-rendered markdown 经 assemble 的
    /// `plane_markdown` 参数注入，非 stats 驱动；未配置时为空串、自动跳过）
    PlaneToday,
}

impl StatsBlock {
    /// 对应的 BLOCK_* 常量名（用于 wrap_block 标记）
    pub fn block_name(&self) -> &'static str {
        match self {
            Self::CategoryTable => BLOCK_CATEGORY_TABLE,
            Self::AppUsageTable => BLOCK_APP_USAGE_TABLE,
            Self::HourlySummary => BLOCK_HOURLY_SUMMARY,
            Self::DomainUsageTable => BLOCK_DOMAIN_USAGE_TABLE,
            Self::LocalOverview => BLOCK_LOCAL_OVERVIEW,
            Self::LocalCategory => BLOCK_LOCAL_CATEGORY,
            Self::LocalAppUsage => BLOCK_LOCAL_APP_USAGE,
            Self::LocalDomainUsage => BLOCK_LOCAL_DOMAIN_USAGE,
            Self::GitlabToday => BLOCK_GITLAB_TODAY,
            Self::PlaneToday => BLOCK_PLANE_TODAY,
        }
    }

    /// 用 stats 渲染该区块的 markdown 内容（不含 wrap_block 标记）
    pub fn render(
        &self,
        stats: &DailyStats,
        locale: AppLocale,
        category_overrides: &HashMap<String, String>,
        semantic_overrides: &HashMap<String, String>,
    ) -> String {
        match self {
            Self::CategoryTable => render_category_table(stats, locale, category_overrides),
            Self::AppUsageTable => render_app_usage_table(stats, locale),
            Self::HourlySummary => render_hourly_summary(stats, locale),
            Self::DomainUsageTable => render_domain_usage_table(stats, locale, semantic_overrides),
            Self::LocalOverview => render_local_overview(stats, locale),
            Self::LocalCategory => render_local_category_list(stats, locale, category_overrides),
            Self::LocalAppUsage => render_local_app_usage_list(stats, locale),
            Self::LocalDomainUsage => {
                render_local_domain_usage_list(stats, locale, semantic_overrides)
            }
            // GitLab 小节不由 stats 渲染：assemble 会用传入的 gitlab_markdown 替代
            Self::GitlabToday => String::new(),
            // Plane 小节同上：assemble 会用传入的 plane_markdown 替代
            Self::PlaneToday => String::new(),
        }
    }
}

fn section_number_prefix(locale: AppLocale, index: usize) -> String {
    let cjk = ["一", "二", "三", "四", "五", "六", "七", "八", "九", "十"];

    match locale {
        AppLocale::En => format!("{index}. "),
        AppLocale::Ar => format!("{index}. "),
        _ => {
            let number = cjk
                .get(index.saturating_sub(1))
                .map(|value| (*value).to_string())
                .unwrap_or_else(|| index.to_string());
            format!("{number}、")
        }
    }
}

/// 生成带动态序号的二级标题。
pub fn format_section_heading(locale: AppLocale, index: usize, title: &str) -> String {
    format!("## {}{title}\n\n", section_number_prefix(locale, index))
}

/// 给 markdown 字符串中的 `## ` 标题加上序号（根据 locale 用中文或阿拉伯数字）。
fn number_sections_from(markdown: &str, locale: AppLocale, first_index: usize) -> (String, usize) {
    let mut counter = 0;
    let mut out = String::with_capacity(markdown.len());

    for line in markdown.split_inclusive('\n') {
        if let Some(rest) = line.strip_prefix("## ") {
            counter += 1;
            out.push_str("## ");
            out.push_str(&section_number_prefix(locale, first_index + counter - 1));
            out.push_str(rest);
        } else {
            out.push_str(line);
        }
    }

    (out, counter)
}

/// 按给定顺序组装统计区块，返回带 WR_BLOCK 标记的 markdown 字符串。
/// 标题序号由 assemble 动态分配（render 函数里的标题不含序号）。
/// 空内容的块自动跳过（wrap_block 返回空串）。
/// `gitlab_markdown` / `plane_markdown` 分别为「GitLab 今日提交」「Plane 工作
/// 项」小节的 pre-rendered 内容（含无编号标题；未配置时传空串，该块自动跳过）。
pub fn assemble(
    blocks: &[StatsBlock],
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
    semantic_overrides: &HashMap<String, String>,
    gitlab_markdown: &str,
    plane_markdown: &str,
) -> String {
    assemble_with_section_count(
        blocks,
        stats,
        locale,
        category_overrides,
        semantic_overrides,
        gitlab_markdown,
        plane_markdown,
    )
    .0
}

/// 按给定顺序组装统计区块，并返回实际渲染出的二级段落数量。
#[allow(clippy::too_many_arguments)]
pub fn assemble_with_section_count(
    blocks: &[StatsBlock],
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
    semantic_overrides: &HashMap<String, String>,
    gitlab_markdown: &str,
    plane_markdown: &str,
) -> (String, usize) {
    let mut out = String::new();
    for block in blocks {
        let rendered = match block {
            StatsBlock::GitlabToday => gitlab_markdown.to_string(),
            StatsBlock::PlaneToday => plane_markdown.to_string(),
            _ => block.render(stats, locale, category_overrides, semantic_overrides),
        };
        let wrapped = wrap_block(block.block_name(), &rendered);
        if !wrapped.is_empty() {
            out.push_str(&wrapped);
        }
    }
    // 动态编号：把 render 函数输出的无编号标题（## 时间分配）按实际顺序编号
    number_sections_from(&out, locale, 1)
}

/// 从字符串名解析回 StatsBlock（AI 编排用）
pub fn parse_block_name(name: &str) -> Option<StatsBlock> {
    match name.trim().to_uppercase().as_str() {
        "CATEGORY_TABLE" => Some(StatsBlock::CategoryTable),
        "APP_USAGE_TABLE" => Some(StatsBlock::AppUsageTable),
        "HOURLY_SUMMARY" => Some(StatsBlock::HourlySummary),
        "DOMAIN_USAGE_TABLE" => Some(StatsBlock::DomainUsageTable),
        "LOCAL_OVERVIEW" => Some(StatsBlock::LocalOverview),
        "LOCAL_CATEGORY" => Some(StatsBlock::LocalCategory),
        "LOCAL_APP_USAGE" => Some(StatsBlock::LocalAppUsage),
        "LOCAL_DOMAIN_USAGE" => Some(StatsBlock::LocalDomainUsage),
        "GITLAB_TODAY" => Some(StatsBlock::GitlabToday),
        "PLANE_TODAY" => Some(StatsBlock::PlaneToday),
        _ => None,
    }
}

/// 根据用户偏好（pinned）调整段落顺序：pinned 中的块排到最前（按 pinned 列表顺序）。
///
/// 注意：hidden 的过滤已移到前端显示层（`getVisibleReportSections`），生成时保留
/// 全部区块。这样用户在「管理段落」恢复隐藏区块后无需重新生成即可见——之前的
/// 实现在此过滤 hidden，导致 hidden 区块被永久写出生成内容、restore 后无法显示。
pub fn apply_preferences(blocks: Vec<StatsBlock>, pinned: &[String]) -> Vec<StatsBlock> {
    let is_pinned = |b: &StatsBlock| pinned.iter().any(|p| p == b.block_name());

    // 分成 pinned（按 pinned 列表顺序）+ 其余（保持原顺序）
    let mut pinned_blocks = Vec::new();
    for pin_name in pinned {
        if let Some(b) = blocks.iter().find(|b| b.block_name() == pin_name) {
            pinned_blocks.push(*b);
        }
    }
    let rest: Vec<StatsBlock> = blocks.into_iter().filter(|b| !is_pinned(b)).collect();

    pinned_blocks.into_iter().chain(rest).collect()
}

/// 将 AI 返回的排序结果（block name 数组）映射为 StatsBlock 数组。
/// 只保留在 available 中的 block（避免 AI 返回不存在的块），未覆盖的块按原顺序追加。
pub fn merge_ai_order(ai_order: &[String], available: &[StatsBlock]) -> Vec<StatsBlock> {
    let mut result = Vec::new();
    let mut used = std::collections::HashSet::new();

    // 按 AI 顺序，把能匹配上的 block 放进来
    for name in ai_order {
        if let Some(block) = parse_block_name(name) {
            if available.contains(&block) && !used.contains(&block) {
                result.push(block);
                used.insert(block);
            }
        }
    }

    // AI 没覆盖到的 block，保持原顺序追加
    for block in available {
        if !used.contains(block) {
            result.push(*block);
        }
    }

    result
}
pub fn default_summary_order() -> Vec<StatsBlock> {
    vec![
        StatsBlock::CategoryTable,
        StatsBlock::AppUsageTable,
        StatsBlock::HourlySummary,
        StatsBlock::DomainUsageTable,
        StatsBlock::GitlabToday,
        StatsBlock::PlaneToday,
    ]
}

/// local 模式的默认段落顺序
pub fn default_local_order() -> Vec<StatsBlock> {
    vec![
        StatsBlock::LocalOverview,
        StatsBlock::LocalCategory,
        StatsBlock::LocalAppUsage,
        StatsBlock::HourlySummary,
        StatsBlock::LocalDomainUsage,
        StatsBlock::GitlabToday,
        StatsBlock::PlaneToday,
    ]
}

/// 把 GitLab 今日提交块锚定到「按小时活跃度」之前（产品约定的固定位置）。
///
/// AI 编排序与用户钉选都可能让块落在别的位置：merge_ai_order 对未覆盖的
/// GITLAB_TODAY 会追加到末尾，因此组装后必须重锚。用户显式钉选
/// GITLAB_TODAY 时（apply_preferences 已把它排到最前）视为主动布局，跳过重锚。
/// 「按小时活跃度」块不在列表中时锚到末尾。
pub fn place_gitlab_before_hourly(
    mut blocks: Vec<StatsBlock>,
    pinned: &[String],
) -> Vec<StatsBlock> {
    if pinned.iter().any(|p| p == BLOCK_GITLAB_TODAY) {
        return blocks;
    }
    let Some(from) = blocks
        .iter()
        .position(|b| matches!(b, StatsBlock::GitlabToday))
    else {
        return blocks;
    };
    let gitlab = blocks.remove(from);
    let anchor = blocks
        .iter()
        .position(|b| matches!(b, StatsBlock::HourlySummary))
        .unwrap_or(blocks.len());
    blocks.insert(anchor, gitlab);
    blocks
}

/// 把 Plane 工作项块锚定到「按小时活跃度」之前（与 GitLab 小节同款锚定先例；
/// 在 place_gitlab_before_hourly 之后调用时，落在 GitLab 小节之后、
/// 按小时活跃度之前）。用户显式钉选 PLANE_TODAY 时视为主动布局，跳过重锚。
pub fn place_plane_before_hourly(
    mut blocks: Vec<StatsBlock>,
    pinned: &[String],
) -> Vec<StatsBlock> {
    if pinned.iter().any(|p| p == BLOCK_PLANE_TODAY) {
        return blocks;
    }
    let Some(from) = blocks
        .iter()
        .position(|b| matches!(b, StatsBlock::PlaneToday))
    else {
        return blocks;
    };
    let plane = blocks.remove(from);
    let anchor = blocks
        .iter()
        .position(|b| matches!(b, StatsBlock::HourlySummary))
        .unwrap_or(blocks.len());
    blocks.insert(anchor, plane);
    blocks
}

/// 构建「Plane 工作项」独立小节 markdown（含无编号标题，块系统会统一编号）。
/// 与 GitLab 小节的占位先例一致：调用方在配置齐全时始终产出小节——
/// 有工作项 = 每条一行的列表（状态 + 名称 + 项目 + 更新时间）；
/// 无工作项 = 占位说明，不静默省略。数据来自当日入库行（时间线页补采写入）。
pub fn build_plane_local_section(items: &[PlaneWorkItemRow], locale: AppLocale) -> String {
    let mut lines: Vec<String> = items
        .iter()
        .map(|item| format!("- {}", format_plane_item_line(item)))
        .collect();
    if lines.is_empty() {
        lines.push(plane_empty_hint(locale));
    }
    format!("## {}\n\n{}", plane_section_title(locale), lines.join("\n"))
}

/// 单条工作项的展示行，形如：`[进行中] 搭建设计稿评审流程 · Alpha · 14:30`
fn format_plane_item_line(item: &PlaneWorkItemRow) -> String {
    let updated_at = chrono::DateTime::from_timestamp(item.updated_at, 0)
        .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
        .unwrap_or_else(|| "??".to_string());
    format!(
        "[{state}] {name} · {project} · {updated_at}",
        state = item.state_name,
        name = item.name,
        project = item.project_name,
    )
}

/// 「Plane 工作项」小节标题（按 locale；与 gitlab_section_title 同款机制）
fn plane_section_title(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => "Plane 工作项",
        AppLocale::En => "Plane Work Items",
        AppLocale::Ar => "عناصر عمل Plane",
    }
}

/// 无工作项时的占位说明（按 locale；zh-TW 沿用简体「工作项」一词，
/// 与 plane_section_title 保持同一小节标题口径）
fn plane_empty_hint(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => "今日无 Plane 工作项记录。".to_string(),
        AppLocale::ZhTw => "今日無 Plane 工作項記錄。".to_string(),
        AppLocale::En => "No Plane work items today.".to_string(),
        AppLocale::Ar => "لا توجد عناصر عمل Plane اليوم.".to_string(),
    }
}

const BLOCK_PREFIX_START: &str = "<!-- WR_BLOCK_START:";
const BLOCK_PREFIX_END: &str = "<!-- WR_BLOCK_END:";
const MARKER_SUFFIX: &str = " -->";

/// 把内容包入占位符标记中。如果传入 content 是空串则返回空串（不插入空块）。
pub fn wrap_block(name: &str, content: &str) -> String {
    if content.is_empty() {
        return String::new();
    }
    format!(
        "{prefix_start}{name}{suffix}\n{content}{newline}{prefix_end}{name}{suffix}\n",
        prefix_start = BLOCK_PREFIX_START,
        prefix_end = BLOCK_PREFIX_END,
        suffix = MARKER_SUFFIX,
        newline = if content.ends_with('\n') { "" } else { "\n" }
    )
}

/// 读时用最新 stats 重新渲染所有已知统计区块。未识别 / 未出现的块原样保留。
pub fn render_report_with_live_stats(
    content: &str,
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
    semantic_overrides: &HashMap<String, String>,
) -> String {
    let mut output = content.to_string();
    output = replace_block(
        &output,
        BLOCK_CATEGORY_TABLE,
        &render_category_table(stats, locale, category_overrides),
    );
    output = replace_block(
        &output,
        BLOCK_APP_USAGE_TABLE,
        &render_app_usage_table(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_HOURLY_SUMMARY,
        &render_hourly_summary(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_DOMAIN_USAGE_TABLE,
        &render_domain_usage_table(stats, locale, semantic_overrides),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_OVERVIEW,
        &render_local_overview(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_CATEGORY,
        &render_local_category_list(stats, locale, category_overrides),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_APP_USAGE,
        &render_local_app_usage_list(stats, locale),
    );
    output = replace_block(
        &output,
        BLOCK_LOCAL_DOMAIN_USAGE,
        &render_local_domain_usage_list(stats, locale, semantic_overrides),
    );
    output
}

fn replace_block(content: &str, name: &str, fresh: &str) -> String {
    let start = format!("{BLOCK_PREFIX_START}{name}{MARKER_SUFFIX}");
    let end = format!("{BLOCK_PREFIX_END}{name}{MARKER_SUFFIX}");

    let mut result = String::with_capacity(content.len());
    let mut cursor = 0usize;

    loop {
        let Some(rel_start) = content[cursor..].find(&start) else {
            result.push_str(&content[cursor..]);
            break;
        };
        let abs_start = cursor + rel_start;
        result.push_str(&content[cursor..abs_start]);

        let Some(rel_end) = content[abs_start..].find(&end) else {
            // 未配对，原样保留
            result.push_str(&content[abs_start..]);
            break;
        };
        let abs_end = abs_start + rel_end + end.len();

        result.push_str(&start);
        result.push('\n');
        let trimmed = fresh.trim_matches('\n');
        if !trimmed.is_empty() {
            result.push_str(trimmed);
            result.push('\n');
        }
        result.push_str(&end);
        cursor = abs_end;
    }

    result
}

// ─────────── summary mode 的块渲染器 ───────────

fn usage_percentage(duration: i64, total_duration: i64) -> i32 {
    if total_duration > 0 {
        (duration as f64 / total_duration as f64 * 100.0).round() as i32
    } else {
        0
    }
}

fn format_hour_range_for_note(hour: i32) -> String {
    let start = hour.clamp(0, 23);
    let end = (start + 1) % 24;
    format!("{start:02}:00-{end:02}:00")
}

fn category_table_takeaway(
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
) -> String {
    let Some(top_category) = stats.category_usage.first() else {
        return String::new();
    };
    let name = translate_category_name(&top_category.category, locale, category_overrides);
    let duration = format_duration_for_locale(top_category.duration, locale);
    let percentage = usage_percentage(top_category.duration, stats.total_duration);

    match locale {
        AppLocale::ZhCn => format!(
            "> **短结论：** 主要时间集中在{name}，累计{duration}，占总记录时长 {percentage}%。\n\n"
        ),
        AppLocale::ZhTw => format!(
            "> **短結論：** 主要時間集中在{name}，累計{duration}，佔總記錄時長 {percentage}%。\n\n"
        ),
        AppLocale::En => format!(
            "> **Takeaway:** Most tracked time went to {name}, totaling {duration} ({percentage}% of the recorded day).\n\n"
        ),
        AppLocale::Ar => format!(
            "> **الخلاصة:** ذهب معظم الوقت المتتبع إلى {name}، بإجمالي {duration} ({percentage}% من اليوم المسجل).\n\n"
        ),
    }
}

fn app_usage_table_takeaway(stats: &DailyStats, locale: AppLocale) -> String {
    let Some(top_app) = stats.app_usage.first() else {
        return String::new();
    };
    let duration = format_duration_for_locale(top_app.duration, locale);
    let app_count = stats.app_usage.len();

    match locale {
        AppLocale::ZhCn => format!(
            "> **短结论：** 使用最久的应用是 {}，累计{}；今天共记录 {} 个应用。\n\n",
            top_app.app_name, duration, app_count
        ),
        AppLocale::ZhTw => format!(
            "> **短結論：** 使用最久的應用是 {}，累計{}；今天共記錄 {} 個應用。\n\n",
            top_app.app_name, duration, app_count
        ),
        AppLocale::En => format!(
            "> **Takeaway:** The most-used app was {}, totaling {}; {} apps were recorded today.\n\n",
            top_app.app_name, duration, app_count
        ),
        AppLocale::Ar => format!(
            "> **الخلاصة:** التطبيق الأكثر استخداماً كان {}، بإجمالي {}؛ تم تسجيل {} تطبيقات اليوم.\n\n",
            top_app.app_name, duration, app_count
        ),
    }
}

fn hourly_summary_takeaway(stats: &DailyStats, locale: AppLocale) -> String {
    let active_buckets = stats
        .hourly_activity_distribution
        .iter()
        .filter(|bucket| bucket.duration > 0)
        .collect::<Vec<_>>();
    let Some(peak_bucket) = active_buckets
        .iter()
        .max_by(|left, right| {
            left.duration
                .cmp(&right.duration)
                .then_with(|| right.hour.cmp(&left.hour))
        })
        .copied()
    else {
        return String::new();
    };
    let peak_range = format_hour_range_for_note(peak_bucket.hour);
    let duration = format_duration_for_locale(peak_bucket.duration, locale);
    let active_hours = active_buckets.len();

    match locale {
        AppLocale::ZhCn => format!(
            "> **短结论：** 活跃高峰出现在{peak_range}，该小时记录{duration}；全天共有 {active_hours} 个活跃小时。\n\n"
        ),
        AppLocale::ZhTw => format!(
            "> **短結論：** 活躍高峰出現在{peak_range}，該小時記錄{duration}；全天共有 {active_hours} 個活躍小時。\n\n"
        ),
        AppLocale::En => format!(
            "> **Takeaway:** Activity peaked at {peak_range}, with {duration} recorded in that hour across {active_hours} active hours.\n\n"
        ),
        AppLocale::Ar => format!(
            "> **الخلاصة:** بلغ النشاط ذروته في {peak_range}، حيث تم تسجيل {duration} في تلك الساعة عبر {active_hours} ساعات نشطة.\n\n"
        ),
    }
}

fn domain_usage_table_takeaway(
    stats: &DailyStats,
    locale: AppLocale,
    semantic_overrides: &HashMap<String, String>,
) -> String {
    let Some(top_domain) = stats.domain_usage.first() else {
        return String::new();
    };
    let domain = format_domain_label_local(top_domain, locale, semantic_overrides);
    let duration = format_duration_for_locale(top_domain.duration, locale);
    let domain_count = stats.domain_usage.len();

    match locale {
        AppLocale::ZhCn => format!(
            "> **短结论：** 访问时间最长的网站是 {domain}，累计{duration}；今天共记录 {domain_count} 个网站。\n\n"
        ),
        AppLocale::ZhTw => format!(
            "> **短結論：** 造訪時間最長的網站是 {domain}，累計{duration}；今天共記錄 {domain_count} 個網站。\n\n"
        ),
        AppLocale::En => format!(
            "> **Takeaway:** The longest website visit was {domain}, totaling {duration}; {domain_count} websites were recorded today.\n\n"
        ),
        AppLocale::Ar => format!(
            "> **الخلاصة:** أطول زيارة لموقع ويب كانت {domain}، بإجمالي {duration}؛ تم تسجيل {domain_count} مواقع اليوم.\n\n"
        ),
    }
}

pub fn render_category_table(
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
) -> String {
    if stats.category_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 时间分配\n\n",
        AppLocale::ZhTw => "## 時間分配\n\n",
        AppLocale::En => "## Time Allocation\n\n",
        AppLocale::Ar => "## تخصيص الوقت\n\n",
    });
    out.push_str(&category_table_takeaway(stats, locale, category_overrides));
    out.push_str(match locale {
        AppLocale::ZhCn => "| 类别 | 时长 | 占比 |\n|:--|--:|--:|\n",
        AppLocale::ZhTw => "| 類別 | 時長 | 佔比 |\n|:--|--:|--:|\n",
        AppLocale::En => "| Category | Duration | Share |\n|:--|--:|--:|\n",
        AppLocale::Ar => "| الفئة | المدة | النسبة |\n|:--|--:|--:|\n",
    });
    for cat in &stats.category_usage {
        let percentage = usage_percentage(cat.duration, stats.total_duration);
        out.push_str(&format!(
            "| {} | {} | {}% |\n",
            translate_category_name(&cat.category, locale, category_overrides),
            format_duration_for_locale(cat.duration, locale),
            percentage
        ));
    }
    out.push('\n');
    out
}

pub fn render_app_usage_table(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.app_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 应用使用明细\n\n",
        AppLocale::ZhTw => "## 應用使用明細\n\n",
        AppLocale::En => "## App Details\n\n",
        AppLocale::Ar => "## تفاصيل التطبيقات\n\n",
    });
    out.push_str(&app_usage_table_takeaway(stats, locale));
    out.push_str(match locale {
        AppLocale::ZhCn => "| 序号 | 应用名称 | 使用时长 |\n|--:|:--|--:|\n",
        AppLocale::ZhTw => "| 序號 | 應用名稱 | 使用時長 |\n|--:|:--|--:|\n",
        AppLocale::En => "| # | App | Duration |\n|--:|:--|--:|\n",
        AppLocale::Ar => "| # | التطبيق | المدة |\n|--:|:--|--:|\n",
    });
    for (index, app) in stats.app_usage.iter().enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            index + 1,
            app.app_name,
            format_duration_for_locale(app.duration, locale)
        ));
    }
    out.push('\n');
    out
}

pub fn render_hourly_summary(stats: &DailyStats, locale: AppLocale) -> String {
    let Some(hourly) = generate_hourly_activity_summary_for_locale(stats, locale) else {
        return String::new();
    };
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 按小时活跃度\n\n",
        AppLocale::ZhTw => "## 按小時活躍度\n\n",
        AppLocale::En => "## Hourly Activity\n\n",
        AppLocale::Ar => "## النشاط بالساعة\n\n",
    });
    out.push_str(&hourly_summary_takeaway(stats, locale));
    out.push_str(&hourly);
    out.push('\n');
    out
}

pub fn render_domain_usage_table(
    stats: &DailyStats,
    locale: AppLocale,
    semantic_overrides: &HashMap<String, String>,
) -> String {
    if stats.domain_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 网站访问明细\n\n",
        AppLocale::ZhTw => "## 網站造訪明細\n\n",
        AppLocale::En => "## Website Details\n\n",
        AppLocale::Ar => "## تفاصيل المواقع\n\n",
    });
    out.push_str(&domain_usage_table_takeaway(
        stats,
        locale,
        semantic_overrides,
    ));
    out.push_str(match locale {
        AppLocale::ZhCn => "| 序号 | 网站域名 | 访问时长 |\n|--:|:--|--:|\n",
        AppLocale::ZhTw => "| 序號 | 網站網域 | 造訪時長 |\n|--:|:--|--:|\n",
        AppLocale::En => "| # | Domain | Duration |\n|--:|:--|--:|\n",
        AppLocale::Ar => "| # | الموقع | المدة |\n|--:|:--|--:|\n",
    });
    for (index, domain) in stats.domain_usage.iter().enumerate() {
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            index + 1,
            format_domain_label_local(domain, locale, semantic_overrides),
            format_duration_for_locale(domain.duration, locale)
        ));
    }
    out.push('\n');
    out
}

// ─────────── local mode 的块渲染器（与 summary mode 表格风格不同，是列表风格）───────────

pub fn render_local_overview(stats: &DailyStats, locale: AppLocale) -> String {
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 今日概览\n\n",
        AppLocale::ZhTw => "## 今日概覽\n\n",
        AppLocale::En => "## Overview\n\n",
        AppLocale::Ar => "## نظرة عامة\n\n",
    });
    let line = match locale {
        AppLocale::ZhCn => format!(
            "- **总工作时长**: {}\n- **截图数量**: {} 张\n- **使用应用**: {} 个\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
        AppLocale::ZhTw => format!(
            "- **總工作時長**: {}\n- **截圖數量**: {} 張\n- **使用應用**: {} 個\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
        AppLocale::En => format!(
            "- **Total work duration**: {}\n- **Screenshots**: {}\n- **Apps used**: {}\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
        AppLocale::Ar => format!(
            "- **إجمالي مدة العمل**: {}\n- **لقطات الشاشة**: {}\n- **التطبيقات المستخدمة**: {}\n",
            format_duration_for_locale(stats.total_duration, locale),
            stats.screenshot_count,
            stats.app_usage.len()
        ),
    };
    out.push_str(&line);
    out
}

pub fn render_local_category_list(
    stats: &DailyStats,
    locale: AppLocale,
    category_overrides: &HashMap<String, String>,
) -> String {
    if stats.category_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 时间分配\n\n",
        AppLocale::ZhTw => "## 時間分配\n\n",
        AppLocale::En => "## Time allocation\n\n",
        AppLocale::Ar => "## تخصيص الوقت\n\n",
    });
    for cat in &stats.category_usage {
        let percentage = if stats.total_duration > 0 {
            (cat.duration as f64 / stats.total_duration as f64 * 100.0) as i32
        } else {
            0
        };
        out.push_str(&format!(
            "- **{}**: {} ({}%)\n",
            translate_category_name(&cat.category, locale, category_overrides),
            format_duration_for_locale(cat.duration, locale),
            percentage
        ));
    }
    out
}

pub fn render_local_app_usage_list(stats: &DailyStats, locale: AppLocale) -> String {
    if stats.app_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 应用使用情况\n\n",
        AppLocale::ZhTw => "## 應用使用情況\n\n",
        AppLocale::En => "## App usage\n\n",
        AppLocale::Ar => "## استخدام التطبيقات\n\n",
    });
    for (index, app) in stats.app_usage.iter().take(5).enumerate() {
        out.push_str(&format!(
            "{}. **{}**: {}\n",
            index + 1,
            app.app_name,
            format_duration_for_locale(app.duration, locale)
        ));
    }
    out
}

pub fn render_local_domain_usage_list(
    stats: &DailyStats,
    locale: AppLocale,
    semantic_overrides: &HashMap<String, String>,
) -> String {
    if stats.domain_usage.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(match locale {
        AppLocale::ZhCn => "## 网站访问\n\n",
        AppLocale::ZhTw => "## 網站造訪\n\n",
        AppLocale::En => "## Website visits\n\n",
        AppLocale::Ar => "## زيارات المواقع\n\n",
    });
    for domain in stats.domain_usage.iter().take(5) {
        out.push_str(&format!(
            "- **{}**: {}\n",
            format_domain_label_local(domain, locale, semantic_overrides),
            format_duration_for_locale(domain.duration, locale)
        ));
    }
    out
}

fn format_domain_label_local(
    domain: &DomainUsage,
    locale: AppLocale,
    semantic_overrides: &HashMap<String, String>,
) -> String {
    match domain.semantic_category.as_deref().map(str::trim) {
        Some(semantic_category) if !semantic_category.is_empty() => {
            let semantic_category =
                translate_semantic_category_name(semantic_category, locale, semantic_overrides);
            match locale {
                AppLocale::En => format!("{} ({})", domain.domain, semantic_category),
                AppLocale::Ar => format!("{} ({})", domain.domain, semantic_category),
                _ => format!("{}（{}）", domain.domain, semantic_category),
            }
        }
        _ => domain.domain.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{AppUsage, CategoryUsage, DomainUsage, HourlyActivityBucket, UrlDetail};

    fn sample_stats_for_numbering() -> DailyStats {
        DailyStats {
            total_duration: 3600,
            screenshot_count: 3,
            app_usage: vec![AppUsage {
                app_name: "VS Code".to_string(),
                duration: 2400,
                count: 1,
                executable_path: None,
                screenshot_url: None,
            }],
            category_usage: vec![CategoryUsage {
                category: "development".to_string(),
                duration: 3600,
            }],
            browser_duration: 0,
            url_usage: vec![],
            domain_usage: vec![],
            domain_total_count: 0,
            browser_usage: vec![],
            work_time_duration: 3600,
            overtime_duration: 0,
            hourly_activity_distribution: vec![],
        }
    }

    fn sample_stats_for_editorial_notes() -> DailyStats {
        DailyStats {
            total_duration: 7200,
            screenshot_count: 8,
            app_usage: vec![
                AppUsage {
                    app_name: "VS Code".to_string(),
                    duration: 4200,
                    count: 3,
                    executable_path: None,
                    screenshot_url: None,
                },
                AppUsage {
                    app_name: "Chrome".to_string(),
                    duration: 1800,
                    count: 2,
                    executable_path: None,
                    screenshot_url: None,
                },
            ],
            category_usage: vec![
                CategoryUsage {
                    category: "development".to_string(),
                    duration: 5400,
                },
                CategoryUsage {
                    category: "browser".to_string(),
                    duration: 1800,
                },
            ],
            browser_duration: 1800,
            url_usage: vec![],
            domain_total_count: 1,
            domain_usage: vec![DomainUsage {
                domain: "github.com".to_string(),
                duration: 1500,
                semantic_category: Some("开发协作".to_string()),
                urls: vec![UrlDetail {
                    url: "https://github.com/example/project".to_string(),
                    duration: 1500,
                }],
            }],
            browser_usage: vec![],
            work_time_duration: 7200,
            overtime_duration: 0,
            hourly_activity_distribution: vec![
                HourlyActivityBucket {
                    hour: 9,
                    duration: 1200,
                },
                HourlyActivityBucket {
                    hour: 10,
                    duration: 3000,
                },
            ],
        }
    }

    #[test]
    fn wrap_block_skips_empty() {
        assert!(wrap_block(BLOCK_CATEGORY_TABLE, "").is_empty());
    }

    #[test]
    fn wrap_block_adds_markers_and_trailing_newline() {
        let wrapped = wrap_block(BLOCK_CATEGORY_TABLE, "hello\n");
        assert!(wrapped.starts_with("<!-- WR_BLOCK_START:CATEGORY_TABLE -->\n"));
        assert!(wrapped.contains("hello\n"));
        assert!(wrapped
            .trim_end()
            .ends_with("<!-- WR_BLOCK_END:CATEGORY_TABLE -->"));
    }

    #[test]
    fn replace_block_overwrites_content_between_markers() {
        let original = "before\n<!-- WR_BLOCK_START:APP_USAGE_TABLE -->\nstale rows\n<!-- WR_BLOCK_END:APP_USAGE_TABLE -->\nafter\n";
        let updated = replace_block(original, BLOCK_APP_USAGE_TABLE, "fresh rows\n");
        assert!(updated.contains("fresh rows"));
        assert!(!updated.contains("stale rows"));
        assert!(updated.starts_with("before\n"));
        assert!(updated.ends_with("after\n"));
    }

    #[test]
    fn replace_block_returns_input_when_no_markers() {
        let original = "no markers here\n";
        let updated = replace_block(original, BLOCK_CATEGORY_TABLE, "ignored");
        assert_eq!(updated, original);
    }

    #[test]
    fn replace_block_handles_unmatched_start() {
        let original = "<!-- WR_BLOCK_START:CATEGORY_TABLE -->\nnever closed\n";
        let updated = replace_block(original, BLOCK_CATEGORY_TABLE, "ignored");
        assert_eq!(updated, original);
    }

    #[test]
    fn render_report_with_live_stats_passes_through_legacy_content() {
        let legacy = "# 工作日报\n\n直接写死的旧版本，没有任何标记\n";
        let stats = DailyStats {
            total_duration: 0,
            screenshot_count: 0,
            app_usage: vec![],
            category_usage: vec![],
            browser_duration: 0,
            url_usage: vec![],
            domain_usage: vec![],
            domain_total_count: 0,
            browser_usage: vec![],
            work_time_duration: 0,
            overtime_duration: 0,
            hourly_activity_distribution: vec![],
        };
        assert_eq!(
            render_report_with_live_stats(
                legacy,
                &stats,
                AppLocale::ZhCn,
                &HashMap::new(),
                &HashMap::new()
            ),
            legacy
        );
    }

    #[test]
    fn assemble应按实际区块顺序动态编号并返回段落数() {
        let stats = sample_stats_for_numbering();
        let (markdown, section_count) = assemble_with_section_count(
            &[StatsBlock::AppUsageTable, StatsBlock::CategoryTable],
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );

        assert_eq!(section_count, 2);
        assert!(markdown.contains("## 一、应用使用明细"));
        assert!(markdown.contains("## 二、时间分配"));
        assert!(!markdown.contains("## 三、应用使用明细"));
    }

    #[test]
    fn 英文段落标题应按实际顺序使用阿拉伯数字编号() {
        let stats = sample_stats_for_numbering();
        let (markdown, section_count) = assemble_with_section_count(
            &[StatsBlock::CategoryTable, StatsBlock::AppUsageTable],
            &stats,
            AppLocale::En,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );

        assert_eq!(section_count, 2);
        assert!(markdown.contains("## 1. Time Allocation"));
        assert!(markdown.contains("## 2. App Details"));
    }

    #[test]
    fn summary四个数据区块应保留并在标题后给出短结论() {
        let stats = sample_stats_for_editorial_notes();
        let markdown = assemble(
            &default_summary_order(),
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );

        assert!(markdown.contains("## 一、时间分配\n\n> **短结论：**"));
        assert!(markdown.contains("主要时间集中在"));
        assert!(markdown.contains("| 类别 | 时长 | 占比 |"));

        assert!(markdown.contains("## 二、应用使用明细\n\n> **短结论：**"));
        assert!(markdown.contains("使用最久的应用是 VS Code"));
        assert!(markdown.contains("| 序号 | 应用名称 | 使用时长 |"));

        assert!(markdown.contains("## 三、按小时活跃度\n\n> **短结论：**"));
        assert!(markdown.contains("活跃高峰出现在"));
        assert!(markdown.contains("- 高峰时段:"));

        assert!(markdown.contains("## 四、网站访问明细\n\n> **短结论：**"));
        assert!(markdown.contains("访问时间最长的网站是 github.com"));
        assert!(markdown.contains("| 序号 | 网站域名 | 访问时长 |"));
    }

    #[test]
    fn 英文summary数据区块应使用takeaway短结论() {
        let stats = sample_stats_for_editorial_notes();
        let markdown = assemble(
            &[
                StatsBlock::CategoryTable,
                StatsBlock::AppUsageTable,
                StatsBlock::HourlySummary,
                StatsBlock::DomainUsageTable,
            ],
            &stats,
            AppLocale::En,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );

        assert!(markdown.contains("## 1. Time Allocation\n\n> **Takeaway:**"));
        assert!(markdown.contains("Most tracked time went to"));
        assert!(markdown.contains("## 2. App Details\n\n> **Takeaway:**"));
        assert!(markdown.contains("The most-used app was VS Code"));
        assert!(markdown.contains("## 3. Hourly Activity\n\n> **Takeaway:**"));
        assert!(markdown.contains("Activity peaked at"));
        assert!(markdown.contains("## 4. Website Details\n\n> **Takeaway:**"));
        assert!(markdown.contains("The longest website visit was github.com"));
    }

    #[test]
    fn gitlab块应锚定在按小时活跃度之前() {
        // 模拟用户缓存的 AI 块序（未覆盖 GITLAB_TODAY，会被 merge 追加到末尾）
        let blocks = place_gitlab_before_hourly(
            merge_ai_order(
                &[
                    "APP_USAGE_TABLE".to_string(),
                    "CATEGORY_TABLE".to_string(),
                    "DOMAIN_USAGE_TABLE".to_string(),
                    "HOURLY_SUMMARY".to_string(),
                ],
                &default_summary_order(),
            ),
            &[],
        );
        let names: Vec<&str> = blocks.iter().map(|b| b.block_name()).collect();
        let gitlab = names
            .iter()
            .position(|n| *n == BLOCK_GITLAB_TODAY)
            .expect("应包含 GITLAB_TODAY");
        let hourly = names
            .iter()
            .position(|n| *n == BLOCK_HOURLY_SUMMARY)
            .expect("应包含 HOURLY_SUMMARY");
        let domain = names
            .iter()
            .position(|n| *n == BLOCK_DOMAIN_USAGE_TABLE)
            .expect("应包含 DOMAIN_USAGE_TABLE");
        assert_eq!(
            gitlab + 1,
            hourly,
            "GitLab 应紧跟在按小时活跃度之前: {names:?}"
        );
        assert!(domain < gitlab, "网站访问明细应在 GitLab 之前: {names:?}");
    }

    #[test]
    fn gitlab块被显式钉选时应尊重钉选不再重锚() {
        let pinned = [BLOCK_GITLAB_TODAY.to_string()];
        let blocks = place_gitlab_before_hourly(
            apply_preferences(default_summary_order(), &pinned),
            &pinned,
        );
        assert_eq!(blocks[0].block_name(), BLOCK_GITLAB_TODAY, "钉选应置顶");
    }

    #[test]
    fn gitlab小节经装配应被编号并带块标记() {
        let stats = sample_stats_for_editorial_notes();
        let markdown = assemble(
            &place_gitlab_before_hourly(default_summary_order(), &[]),
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "## GitLab 今日提交

- [g/p] [main] abc fix: x
",
            "",
        );
        assert!(
            markdown.contains("## 三、GitLab 今日提交"),
            "GitLab 应按所在位置获得序号: {markdown}"
        );
        assert!(
            markdown.contains("<!-- WR_BLOCK_START:GITLAB_TODAY -->"),
            "应带可钉选/隐藏的块标记"
        );
        assert!(
            markdown.contains("## 四、按小时活跃度"),
            "按小时活跃度应顺延编号"
        );
    }

    #[test]
    fn 未配置gitlab时装配应跳过该块() {
        let stats = sample_stats_for_editorial_notes();
        let (markdown, count) = assemble_with_section_count(
            &place_gitlab_before_hourly(default_summary_order(), &[]),
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );
        assert!(!markdown.contains("GITLAB_TODAY"), "未配置不应出现该块");
        assert_eq!(count, 4, "未配置时仍为四个数据小节");
    }

    #[test]
    fn plane块应锚定在gitlab之后且在按小时活跃度之前() {
        let blocks = place_plane_before_hourly(
            place_gitlab_before_hourly(default_summary_order(), &[]),
            &[],
        );
        let names: Vec<&str> = blocks.iter().map(|b| b.block_name()).collect();
        let gitlab = names
            .iter()
            .position(|n| *n == BLOCK_GITLAB_TODAY)
            .expect("应包含 GITLAB_TODAY");
        let plane = names
            .iter()
            .position(|n| *n == BLOCK_PLANE_TODAY)
            .expect("应包含 PLANE_TODAY");
        let hourly = names
            .iter()
            .position(|n| *n == BLOCK_HOURLY_SUMMARY)
            .expect("应包含 HOURLY_SUMMARY");
        assert_eq!(
            plane + 1,
            hourly,
            "Plane 应紧跟在按小时活跃度之前: {names:?}"
        );
        assert!(gitlab < plane, "Plane 应排在 GitLab 小节之后: {names:?}");
    }

    #[test]
    fn plane块被显式钉选时应尊重钉选不再重锚() {
        let pinned = [BLOCK_PLANE_TODAY.to_string()];
        let blocks =
            place_plane_before_hourly(apply_preferences(default_summary_order(), &pinned), &pinned);
        assert_eq!(blocks[0].block_name(), BLOCK_PLANE_TODAY, "钉选应置顶");
    }

    #[test]
    fn plane小节应含状态名称项目且无数据时给占位说明() {
        use crate::database::PlaneWorkItemRow;

        let item = |name: &str, updated_at: i64| PlaneWorkItemRow {
            plane_item_id: "wi-1".to_string(),
            project_name: "Alpha".to_string(),
            name: name.to_string(),
            state_name: "进行中".to_string(),
            updated_at,
            url: None,
        };
        let section = build_plane_local_section(
            &[item("搭建设计稿评审流程", 1_790_217_000)],
            AppLocale::ZhCn,
        );
        assert!(
            section.starts_with("## Plane 工作项\n\n"),
            "应含无编号标题: {section}"
        );
        assert!(
            section.contains("- [进行中] 搭建设计稿评审流程 · Alpha · "),
            "每条一行: {section}"
        );
        // 时间用 chrono::Local 渲染，期望值按同一时钟推导，避免对测试机时区的硬编码
        let expected_time = chrono::DateTime::from_timestamp(1_790_217_000, 0)
            .map(|dt| dt.with_timezone(&chrono::Local).format("%H:%M").to_string())
            .unwrap_or_default();
        assert!(
            section.ends_with(&expected_time),
            "应含本地更新时间 HH:MM: {section}"
        );

        let placeholder = build_plane_local_section(&[], AppLocale::En);
        assert_eq!(
            placeholder,
            "## Plane Work Items\n\nNo Plane work items today."
        );

        // 多语言标题与占位（zh-TW / ar）
        assert_eq!(plane_section_title(AppLocale::ZhTw), "Plane 工作项");
        assert_eq!(plane_section_title(AppLocale::Ar), "عناصر عمل Plane");
        assert!(
            build_plane_local_section(&[], AppLocale::ZhTw).contains("今日無 Plane 工作項記錄。")
        );
        assert!(
            build_plane_local_section(&[], AppLocale::Ar).contains("لا توجد عناصر عمل Plane اليوم.")
        );
    }

    #[test]
    fn plane小节经装配应被编号并带块标记() {
        let stats = sample_stats_for_editorial_notes();
        let markdown = assemble(
            &place_plane_before_hourly(
                place_gitlab_before_hourly(default_summary_order(), &[]),
                &[],
            ),
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "## GitLab 今日提交\n\n- [g/p] [main] abc fix: x\n",
            "## Plane 工作项\n\n- [进行中] 搭建设计稿评审流程 · Alpha · 14:30\n",
        );
        assert!(
            markdown.contains("## 三、GitLab 今日提交") && markdown.contains("## 四、Plane 工作项"),
            "Plane 应排在 GitLab 小节之后并按所在位置获得序号: {markdown}"
        );
        assert!(
            markdown.contains("<!-- WR_BLOCK_START:PLANE_TODAY -->"),
            "应带可钉选/隐藏的块标记"
        );
        assert!(
            markdown.contains("## 五、按小时活跃度"),
            "按小时活跃度应顺延编号"
        );
        assert!(
            markdown.contains("## 六、网站访问明细"),
            "网站访问明细应顺延编号"
        );

        // 未配置 Plane（空串）时整块跳过；GitLab 同样未传时只剩四个数据小节
        let (markdown, count) = assemble_with_section_count(
            &place_plane_before_hourly(
                place_gitlab_before_hourly(default_summary_order(), &[]),
                &[],
            ),
            &stats,
            AppLocale::ZhCn,
            &HashMap::new(),
            &HashMap::new(),
            "",
            "",
        );
        assert!(!markdown.contains("PLANE_TODAY"), "未配置不应出现该块");
        assert_eq!(count, 4);
    }
}
