//! 日报「文件修改」小节：把当日文件监控事件聚合为统计 + 目录 Top 列表的
//! 独立小节（与 GitLab 今日提交平级，块名 FILE_EVENTS_TODAY）。
//!
//! 产出两个消费面（镜像 GitlabReportSupplement 的双路径）：
//! - `local_section`：日报 markdown 小节（空事件为空串 → 块整体隐藏）
//! - `ai_context`：追加进 AI prompt 的单行补充上下文
//!
//! 文件监控一天可产生数百条事件，小节只做聚合不列明细：
//! 类型统计行（仅非零类型）+ 完整父目录聚合 Top 5 + 其余合并。
//! 事件经 [`crate::gitlab`] 里的 `strip_control_chars` 同款清洗后进入
//! markdown 与 AI 上下文（路径含换行等控制字符时会被劫持注入伪造小节）。

use std::collections::{HashMap, HashSet};

use crate::analysis::AppLocale;
use crate::database::FileEventRow;

/// 日报小节与 AI 上下文的二元产出
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileEventsSection {
    pub local_section: String,
    pub ai_context: String,
}

/// 目录聚合的展示上限；其余目录合并为「其他 N 个目录」
pub const DIRECTORY_TOP_LIMIT: usize = 5;
/// 目录路径展示的最大字符数，超出取首尾中间省略
pub const DIRECTORY_PATH_MAX_CHARS: usize = 60;

pub fn build_file_events_section(
    events: &[FileEventRow],
    truncated: bool,
    total: usize,
    locale: AppLocale,
) -> FileEventsSection {
    if events.is_empty() {
        return FileEventsSection::default();
    }

    let mut created = 0usize;
    let mut modified = 0usize;
    let mut deleted = 0usize;
    let mut renamed = 0usize;
    let mut files: HashSet<String> = HashSet::new();
    let mut directory_counts: HashMap<String, usize> = HashMap::new();

    for event in events {
        match event.event_type.as_str() {
            "created" => created += 1,
            "deleted" => deleted += 1,
            "renamed" => renamed += 1,
            _ => modified += 1,
        }
        files.insert(clean_single_line(&event.path));
        *directory_counts
            .entry(parent_directory(&event.path))
            .or_insert(0) += 1;
    }

    let file_count = files.len();
    let stat_line = build_stat_line(created, modified, deleted, renamed, file_count, locale);

    let mut directories: Vec<(String, usize)> = directory_counts.into_iter().collect();
    directories.sort_unstable_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let top: Vec<(String, usize)> = directories
        .iter()
        .take(DIRECTORY_TOP_LIMIT)
        .map(|(dir, count)| (shorten_path(dir, DIRECTORY_PATH_MAX_CHARS), *count))
        .collect();
    let rest_directories = directories.len().saturating_sub(top.len());
    let rest_changes: usize = directories.iter().skip(top.len()).map(|(_, c)| c).sum();

    let mut lines: Vec<String> = top
        .iter()
        .map(|(dir, count)| format!("- `{dir}` — {count} {}", changes_word(*count, locale)))
        .collect();
    if rest_directories > 0 {
        lines.push(format!(
            "- {} {rest_directories} {} — {rest_changes} {}",
            other_directories_prefix(locale),
            directories_word(rest_directories, locale),
            changes_word(rest_changes, locale),
        ));
    }

    let mut local_section = format!("## {}\n\n{}\n\n", section_title(locale), stat_line);
    if truncated {
        let shown = events.len();
        if total > shown {
            local_section.push_str(&format!(" {}\n", truncated_hint(shown, locale)));
        }
    }
    local_section.push_str(&lines.join("\n"));

    // AI 上下文：单行（防换行注入 prompt 结构），统计 + Top 目录
    let top_dirs_text = top
        .iter()
        .map(|(dir, count)| format!("{dir}（{count}）"))
        .collect::<Vec<_>>()
        .join("、");
    let mut ai_context = format!("{}：{}", ai_context_prefix(locale), stat_line);
    if !top_dirs_text.is_empty() {
        ai_context.push_str(&format!(
            "；{}：{}",
            top_dirs_directories_label(locale),
            top_dirs_text
        ));
    }

    FileEventsSection {
        local_section,
        ai_context: clean_single_line(&ai_context),
    }
}

/// 统一分隔符后取父目录；无分隔符（裸文件名）归入「（当前目录）」
fn parent_directory(path: &str) -> String {
    let normalized = path.replace('\\', "/");
    match normalized.rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => dir.to_string(),
        Some((_, _)) => "/".to_string(),
        None => "（当前目录）".to_string(),
    }
}

/// 超长路径中间省略（按字符数，避免把半截多字节字符切坏）
fn shorten_path(path: &str, max_chars: usize) -> String {
    let chars: Vec<char> = path.chars().collect();
    if chars.len() <= max_chars {
        return path.to_string();
    }
    let head = max_chars / 2;
    let tail = max_chars.saturating_sub(head).saturating_sub(1);
    let shortened: String = chars[..head].iter().collect();
    let tail_part: String = chars[chars.len() - tail..].iter().collect();
    format!("{shortened}…{tail_part}")
}

/// 与 gitlab.rs 同款清洗：控制字符替换为空格保持单行
fn clean_single_line(text: &str) -> String {
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

fn build_stat_line(
    created: usize,
    modified: usize,
    deleted: usize,
    renamed: usize,
    file_count: usize,
    locale: AppLocale,
) -> String {
    let mut parts: Vec<String> = Vec::new();
    let push = |parts: &mut Vec<String>, label: &str, count: usize| {
        if count > 0 {
            parts.push(format!("{label} {count}"));
        }
    };
    let (l_created, l_modified, l_deleted, l_renamed) = stat_labels(locale);
    push(&mut parts, l_created, created);
    push(&mut parts, l_modified, modified);
    push(&mut parts, l_deleted, deleted);
    push(&mut parts, l_renamed, renamed);
    let summary = match locale {
        AppLocale::ZhCn => format!("共 {file_count} 个文件"),
        AppLocale::ZhTw => format!("共 {file_count} 個文件"),
        AppLocale::En => format!("{file_count} files in total"),
        AppLocale::Ar => format!("إجمالي {file_count} ملفًا"),
    };
    if parts.is_empty() {
        summary
    } else {
        format!("{}，{}", parts.join(" · "), summary)
    }
}

fn stat_labels(locale: AppLocale) -> (&'static str, &'static str, &'static str, &'static str) {
    match locale {
        AppLocale::ZhCn => ("新建", "修改", "删除", "重命名"),
        AppLocale::ZhTw => ("新建", "修改", "刪除", "重新命名"),
        AppLocale::En => ("Created", "Modified", "Deleted", "Renamed"),
        AppLocale::Ar => ("جديد", "معدّل", "محذوف", "معاد تسميته"),
    }
}

fn section_title(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => "文件修改",
        AppLocale::En => "File Changes",
        AppLocale::Ar => "تغييرات الملفات",
    }
}

fn changes_word(count: usize, locale: AppLocale) -> &'static str {
    match locale {
        // 中文无复数，单多同形
        AppLocale::ZhCn | AppLocale::ZhTw => "次变动",
        AppLocale::En => {
            if count == 1 {
                "change"
            } else {
                "changes"
            }
        }
        AppLocale::Ar => "تغييرات",
    }
}

fn other_directories_prefix(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => "其他",
        AppLocale::En => "Other",
        AppLocale::Ar => "أخرى",
    }
}

fn directories_word(count: usize, locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "个目录",
        AppLocale::ZhTw => "個目錄",
        AppLocale::En => {
            if count == 1 {
                "directory"
            } else {
                "directories"
            }
        }
        AppLocale::Ar => "أدلة",
    }
}

fn truncated_hint(shown: usize, locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhCn => format!("（事件过多，仅统计最近 {shown} 条）"),
        AppLocale::ZhTw => format!("（事件過多，僅統計最近 {shown} 條）"),
        AppLocale::En => format!("(Too many events, only the most recent {shown} were counted)"),
        AppLocale::Ar => format!("(أحداث كثيرة، تم احتساب آخر {shown} فقط)"),
    }
}

fn ai_context_prefix(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn => "今日文件修改",
        AppLocale::ZhTw => "今日文件修改",
        AppLocale::En => "File changes today",
        AppLocale::Ar => "تغييرات الملفات اليوم",
    }
}

fn top_dirs_directories_label(locale: AppLocale) -> &'static str {
    match locale {
        AppLocale::ZhCn | AppLocale::ZhTw => "主要目录",
        AppLocale::En => "Top directories",
        AppLocale::Ar => "أهم الأدلة",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(path: &str, event_type: &str) -> FileEventRow {
        FileEventRow {
            path: path.to_string(),
            event_type: event_type.to_string(),
            size: None,
            occurred_at: 0,
            date: "2026-09-29".to_string(),
        }
    }

    #[test]
    fn 空事件应返回空串整节隐藏() {
        let section = build_file_events_section(&[], false, 0, AppLocale::ZhCn);
        assert_eq!(section.local_section, "");
        assert_eq!(section.ai_context, "");
    }

    #[test]
    fn 统计行应按类型计数只显示非零类型并含去重文件数() {
        // a.rs 修改两次（同一文件）、b.rs 新建、c.rs 删除
        let events = vec![
            event("C:/w/src/a.rs", "modified"),
            event("C:/w/src/a.rs", "modified"),
            event("C:/w/src/b.rs", "created"),
            event("C:/w/src/c.rs", "deleted"),
        ];
        let section = build_file_events_section(&events, false, events.len(), AppLocale::ZhCn);
        assert!(
            section.local_section.contains("新建 1"),
            "{}",
            section.local_section
        );
        assert!(
            section.local_section.contains("修改 2"),
            "{}",
            section.local_section
        );
        assert!(
            section.local_section.contains("删除 1"),
            "{}",
            section.local_section
        );
        // 重命名 0 次不应出现
        assert!(
            !section.local_section.contains("重命名"),
            "{}",
            section.local_section
        );
        // 去重文件数：a/b/c 共 3 个文件
        assert!(
            section.local_section.contains("共 3 个文件"),
            "{}",
            section.local_section
        );
    }

    #[test]
    fn 目录聚合应取前五并合并其余() {
        let mut events = Vec::new();
        for i in 0..7 {
            let count = 8 - i; // dir0 最多，dir6 最少
            for _ in 0..count {
                events.push(event(&format!("C:/w/dir{i}/f.rs"), "modified"));
            }
        }
        let section = build_file_events_section(&events, false, events.len(), AppLocale::ZhCn);
        // Top5 目录各占一行
        for i in 0..5 {
            assert!(
                section.local_section.contains(&format!("dir{i}")),
                "dir{i} 应在前五：{}",
                section.local_section
            );
        }
        // dir5/dir6 不单独出现，合并为「其他 2 个目录」
        assert!(
            !section.local_section.contains("dir5"),
            "{}",
            section.local_section
        );
        assert!(
            section.local_section.contains("其他 2 个目录"),
            "{}",
            section.local_section
        );
    }

    #[test]
    fn 超长目录路径应中间截断() {
        let long = format!("C:/w/{}dir/f.rs", "x".repeat(150));
        let events = vec![event(&long, "modified")];
        let section = build_file_events_section(&events, false, 1, AppLocale::ZhCn);
        // 展示行不应包含完整超长路径，应出现省略号
        assert!(
            !section.local_section.contains(&"x".repeat(100)),
            "{}",
            section.local_section
        );
        assert!(
            section.local_section.contains("…"),
            "{}",
            section.local_section
        );
    }

    #[test]
    fn 事件截断时应提示仅统计最近条数() {
        let events = vec![event("C:/w/a.rs", "modified")];
        let section = build_file_events_section(&events, true, 2100, AppLocale::ZhCn);
        assert!(
            section.local_section.contains("仅统计最近 1 条"),
            "{}",
            section.local_section
        );
    }

    #[test]
    fn 四语言标题应各自非空() {
        let events = vec![event("C:/w/a.rs", "modified")];
        for locale in [
            AppLocale::ZhCn,
            AppLocale::ZhTw,
            AppLocale::En,
            AppLocale::Ar,
        ] {
            let section = build_file_events_section(&events, false, 1, locale);
            assert!(section.local_section.starts_with("## "), "{locale:?}");
            assert!(section.local_section.lines().count() >= 3, "{locale:?}");
        }
        // 中英标题应有区别（各语言文案存在）
        let zh = build_file_events_section(&events, false, 1, AppLocale::ZhCn).local_section;
        let en = build_file_events_section(&events, false, 1, AppLocale::En).local_section;
        assert_ne!(zh, en);
    }

    #[test]
    fn ai上下文应为单行且含统计信息() {
        let events = vec![
            event("C:/w/src/a.rs", "modified"),
            event("C:/w/src/b.rs", "created"),
        ];
        let section = build_file_events_section(&events, false, 2, AppLocale::ZhCn);
        assert!(!section.ai_context.is_empty());
        assert!(
            !section.ai_context.contains('\n'),
            "AI 上下文必须单行（防换行注入 prompt 结构）：{}",
            section.ai_context
        );
        assert!(
            section.ai_context.contains("新建 1"),
            "{}",
            section.ai_context
        );
    }
}
