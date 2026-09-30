use crate::config::{AppConfig, PrivacyConfig, PrivacyLevel};
use crate::database::DailyStats;
use regex::Regex;

/// 隐私检查结果
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrivacyAction {
    /// 正常记录（截图 + 统计）
    Record,
    /// 内容脱敏（只统计时长，不保存截图）
    Anonymize,
    /// 完全跳过（不记录任何信息）
    Skip,
}

/// 隐私过滤器
pub struct PrivacyFilter {
    config: PrivacyConfig,
    sensitive_patterns: Vec<SensitivePattern>,
}

/// 敏感信息模式：regex + 邻位延续判定。
/// regex 的 `\b` 是 Unicode 词边界——CJK 汉字属词字符，中文与号码/邮箱直接
/// 相邻（中文窗口标题/OCR 输出极常见）时首尾不构成边界、整条漏配。故数字与
/// 邮箱模式一律不带 `\b`，匹配后手动校验前后邻位（与 ocr.rs 的 IP/银行卡
/// 过滤同款），命中面相对旧版只增不减。
struct SensitivePattern {
    regex: Regex,
    /// 邻位字符满足此判定说明匹配只是更长 token 的一部分（如更长数字串），跳过
    is_continuation: fn(char) -> bool,
}

/// 更长数字串（订单号/流水号片段）的一部分
fn digit_continuation(c: char) -> bool {
    c.is_ascii_digit()
}

/// ASCII 词字符延续（邮箱粘着在更长英文标识符里）；CJK 不算延续——那正是要修的漏配
fn word_continuation(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// 密码/密钥类模式无 \b，无需邻位判定
fn no_continuation(_: char) -> bool {
    false
}

/// `\b` 替代款替换：仅当匹配前后邻位都不构成"更长 token"时才替换
fn replace_all_checked(
    text: &str,
    re: &Regex,
    replacement: &str,
    is_continuation: fn(char) -> bool,
) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_end = 0;
    for m in re.find_iter(text) {
        let prev_is_continuation = text[..m.start()]
            .chars()
            .next_back()
            .is_some_and(is_continuation);
        let next_is_continuation = text[m.end()..].chars().next().is_some_and(is_continuation);
        if prev_is_continuation || next_is_continuation {
            continue;
        }
        out.push_str(&text[last_end..m.start()]);
        out.push_str(replacement);
        last_end = m.end();
    }
    out.push_str(&text[last_end..]);
    out
}

impl PrivacyFilter {
    /// 从配置创建隐私过滤器
    pub fn from_config(config: &PrivacyConfig) -> Self {
        let sensitive_patterns = vec![
            // 信用卡号
            SensitivePattern {
                regex: Regex::new(r"\d{4}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}").unwrap(),
                is_continuation: digit_continuation,
            },
            // 手机号
            SensitivePattern {
                regex: Regex::new(r"1[3-9]\d{9}").unwrap(),
                is_continuation: digit_continuation,
            },
            // 身份证号
            SensitivePattern {
                regex: Regex::new(r"\d{17}[\dXx]").unwrap(),
                is_continuation: digit_continuation,
            },
            // 邮箱
            SensitivePattern {
                regex: Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap(),
                is_continuation: word_continuation,
            },
            // 密码字段
            SensitivePattern {
                regex: Regex::new(r"(?i)(password|密码|pwd|passwd)[\s:：=]+\S+").unwrap(),
                is_continuation: no_continuation,
            },
            // API Key
            SensitivePattern {
                regex: Regex::new(r"(?i)(api[_-]?key|secret|token)[\s:：=]+\S+").unwrap(),
                is_continuation: no_continuation,
            },
        ];

        Self {
            config: config.clone(),
            sensitive_patterns,
        }
    }

    /// 检查应用和窗口的隐私操作
    /// 返回应该采取的隐私行动
    pub fn check_privacy(&self, app_name: &str, window_title: &str) -> PrivacyAction {
        // 1. 先检查应用级别的隐私规则
        let app_level = self.config.get_app_privacy_level(app_name);

        match app_level {
            PrivacyLevel::Ignored => {
                log::debug!("应用 {app_name} 设置为完全忽略");
                return PrivacyAction::Skip;
            }
            PrivacyLevel::Anonymized => {
                log::debug!("应用 {app_name} 设置为内容脱敏");
                return PrivacyAction::Anonymize;
            }
            PrivacyLevel::Full => {
                // 继续检查窗口标题
            }
        }

        // 2. 检查窗口标题关键词（匹配时使用脱敏模式）
        if self.config.should_anonymize_by_keyword(window_title) {
            // 不回显标题本身：命中敏感关键词的标题恰是最敏感的内容，
            // debug 排障日志里只记「命中」事实即可（同第八轮采集日志脱敏口径）
            log::debug!(
                "窗口标题命中敏感关键词（长度 {}），使用脱敏模式",
                window_title.chars().count()
            );
            return PrivacyAction::Anonymize;
        }

        PrivacyAction::Record
    }

    /// 检查 URL 域名是否在黑名单中
    /// 如果匹配，返回 Skip；否则返回 Record。
    /// 复用统计侧的 `matches_excluded_domain`（含合并域名兼容匹配）——
    /// 此前采集侧只做精确/后缀匹配,比统计侧窄:统计里被隐藏的域名
    /// 实际仍在被截图与 OCR。两侧必须是同一套语义。
    pub fn check_url_privacy(&self, url: Option<&str>) -> PrivacyAction {
        if let Some(url) = url {
            if !url.is_empty() && matches_excluded_domain(url, &self.config.excluded_domains) {
                log::debug!("URL 命中排除域名名单，跳过记录");
                return PrivacyAction::Skip;
            }
        }
        PrivacyAction::Record
    }

    /// 综合检查：应用 + 窗口标题 + URL
    pub fn check_privacy_full(
        &self,
        app_name: &str,
        window_title: &str,
        browser_url: Option<&str>,
    ) -> PrivacyAction {
        // 1. 先检查应用和窗口标题
        let app_action = self.check_privacy(app_name, window_title);
        if app_action == PrivacyAction::Skip {
            return PrivacyAction::Skip;
        }

        // 2. 检查 URL 域名黑名单
        let url_action = self.check_url_privacy(browser_url);
        if url_action == PrivacyAction::Skip {
            return PrivacyAction::Skip;
        }

        // 3. 返回应用级别的结果（可能是 Record 或 Anonymize）
        app_action
    }

    /// 兼容旧接口：检查是否应该跳过
    pub fn should_skip(&self, app_name: &str, window_title: &str) -> bool {
        self.check_privacy(app_name, window_title) == PrivacyAction::Skip
    }

    /// 过滤OCR文本中的敏感信息
    pub fn filter_text(&self, text: &str) -> String {
        let mut filtered = text.to_string();

        for pattern in &self.sensitive_patterns {
            filtered = replace_all_checked(
                &filtered,
                &pattern.regex,
                "[已过滤]",
                pattern.is_continuation,
            );
        }

        filtered
    }

    /// 更新配置
    pub fn update_config(&mut self, config: &PrivacyConfig) {
        self.config = config.clone();
    }
}

/// 判断 app 是否命中忽略名单。匹配前先经过 `normalize_display_app_name` 归一化，
/// 再做大小写不敏感的单向子串匹配（应用名包含忽略条目才算命中；不做反向包含，
/// 避免忽略 "Google Chrome Canary" 时误伤 "Google Chrome"）。归一化是防御性的：
/// 即便上游传入的是裸可执行名（如 `chrome.exe`），也能与配置里以显示名
/// （如 `Google Chrome`）登记的规则对齐。忽略名单在收集时已统一为小写。
pub fn matches_ignored_app(app_name: &str, ignored_apps: &[String]) -> bool {
    let app_lower = crate::categorize::normalize_display_app_name(app_name)
        .to_lowercase()
        .trim()
        .to_string();
    if app_lower.is_empty() {
        return false;
    }

    ignored_apps
        .iter()
        .any(|ignored| !ignored.is_empty() && app_lower.contains(ignored))
}

/// 判断目标 URL/域名是否命中排除域名名单。先提取域名，再做精确后缀匹配或
/// 合并域名（merged domain）的兼容匹配。
pub fn matches_excluded_domain(target: &str, excluded_domains: &[String]) -> bool {
    let domain = PrivacyConfig::extract_domain(target);
    if domain.is_empty() {
        return false;
    }

    excluded_domains.iter().any(|excluded| {
        let excluded_domain = PrivacyConfig::extract_domain(excluded);
        !excluded_domain.is_empty()
            && (PrivacyConfig::domain_matches(&domain, &excluded_domain)
                || merged_domain_matches_excluded(&domain, &excluded_domain))
    })
}

/// 合并域名（merged domain）兼容匹配：当目标域名本身是一个「合并入口」
/// （如 `github.com` 被合并成 `github.io` 之类的二级入口）时，允许与排除名单中
/// 的同类入口做前缀匹配。
pub fn merged_domain_matches_excluded(domain: &str, excluded_domain: &str) -> bool {
    if !crate::categorize::is_merged_domain(domain) {
        return false;
    }

    let domain = domain.trim_end_matches('.').to_lowercase();
    let excluded_domain = excluded_domain.trim_end_matches('.').to_lowercase();
    let domain_labels: Vec<&str> = domain.split('.').collect();
    let excluded_labels: Vec<&str> = excluded_domain.split('.').collect();

    domain_labels.len() == 2
        && excluded_labels.len() == 2
        && domain_labels[0] == excluded_labels[0]
        && domain_labels[1].starts_with(excluded_labels[1])
        && domain_labels[1].len() > excluded_labels[1].len()
}

/// 从 `AppConfig` 收集隐私过滤所需的两组名单：
/// 返回 `(ignored_apps, excluded_domains)`。
pub fn collect_privacy_filters(config: &AppConfig) -> (Vec<String>, Vec<String>) {
    (
        config.privacy.collect_ignored_app_names(),
        config.privacy.collect_excluded_domains(),
    )
}

/// 把忽略应用名单应用到 `DailyStats`：移除命中名单的应用/浏览器，并重算合计时长。
pub fn apply_ignored_apps_to_stats(mut stats: DailyStats, ignored_apps: &[String]) -> DailyStats {
    if ignored_apps.is_empty() {
        return stats;
    }

    let filtered_app_usage: Vec<_> = stats
        .app_usage
        .into_iter()
        .filter(|app| !matches_ignored_app(&app.app_name, ignored_apps))
        .collect();

    stats.total_duration = filtered_app_usage.iter().map(|app| app.duration).sum();
    stats.app_usage = filtered_app_usage;
    stats
        .browser_usage
        .retain(|browser| !matches_ignored_app(&browser.browser_name, ignored_apps));
    stats.browser_duration = stats
        .browser_usage
        .iter()
        .map(|browser| browser.duration)
        .sum();

    if stats.work_time_duration > stats.total_duration {
        stats.work_time_duration = stats.total_duration;
    }

    stats
}

/// 把排除域名名单应用到 `DailyStats`：移除命中名单的 URL/域名/浏览器子域名，
/// 重算浏览器时长并丢弃时长归零的浏览器。
pub fn apply_excluded_domains_to_stats(
    mut stats: DailyStats,
    excluded_domains: &[String],
) -> DailyStats {
    if excluded_domains.is_empty() {
        return stats;
    }

    stats
        .url_usage
        .retain(|url| !matches_excluded_domain(&url.domain, excluded_domains));
    stats
        .domain_usage
        .retain(|domain| !matches_excluded_domain(&domain.domain, excluded_domains));

    for browser in &mut stats.browser_usage {
        browser
            .domains
            .retain(|domain| !matches_excluded_domain(&domain.domain, excluded_domains));
        browser.duration = browser.domains.iter().map(|domain| domain.duration).sum();
    }
    stats.browser_usage.retain(|browser| browser.duration > 0);
    stats.browser_usage.sort_by(|left, right| {
        right
            .duration
            .cmp(&left.duration)
            .then_with(|| left.browser_name.cmp(&right.browser_name))
    });
    stats.browser_duration = stats
        .browser_usage
        .iter()
        .map(|browser| browser.duration)
        .sum();

    stats
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::*;
    use crate::config::AppPrivacyRule;

    #[test]
    fn test_privacy_levels() {
        let mut config = PrivacyConfig::default();
        config.app_rules.push(AppPrivacyRule {
            app_name: "WeChat".to_string(),
            level: PrivacyLevel::Anonymized,
        });

        let filter = PrivacyFilter::from_config(&config);

        // 完全忽略
        assert_eq!(
            filter.check_privacy("1Password", "Main"),
            PrivacyAction::Skip
        );

        // 内容脱敏（通过规则）
        assert_eq!(
            filter.check_privacy("WeChat", "Chat"),
            PrivacyAction::Anonymize
        );

        // 正常记录
        assert_eq!(
            filter.check_privacy("VS Code", "main.rs"),
            PrivacyAction::Record
        );
    }

    #[test]
    fn test_keyword_anonymize() {
        let config = PrivacyConfig::default();
        let filter = PrivacyFilter::from_config(&config);

        // 关键词触发脱敏
        assert_eq!(
            filter.check_privacy("Chrome", "Bank Login"),
            PrivacyAction::Anonymize
        );
        assert_eq!(
            filter.check_privacy("Firefox", "Enter password"),
            PrivacyAction::Anonymize
        );

        // 正常记录
        assert_eq!(
            filter.check_privacy("Chrome", "GitHub"),
            PrivacyAction::Record
        );
    }

    #[test]
    fn 忽略名单应只做单向小写子串匹配() {
        let ignored = vec!["google chrome".to_string()];
        assert!(matches_ignored_app("Google Chrome", &ignored));
        // 裸可执行名先归一化成显示名后命中
        assert!(matches_ignored_app("chrome", &ignored));
        // 不再做反向包含：忽略 Canary 不应误伤 Google Chrome
        let canary_only = vec!["google chrome canary".to_string()];
        assert!(!matches_ignored_app("Google Chrome", &canary_only));
        // 空条目不应导致忽略一切
        assert!(!matches_ignored_app("VS Code", &[String::new()]));
    }

    #[test]
    fn 采集侧域名排除应与统计侧同一套匹配() {
        let mut config = PrivacyConfig::default();
        config.excluded_domains = vec!["example.com".to_string()];
        let filter = PrivacyFilter::from_config(&config);

        // 精确与子域名后缀命中(与统计侧 matches_excluded_domain 同语义)
        assert_eq!(
            filter.check_url_privacy(Some("https://example.com/page")),
            PrivacyAction::Skip
        );
        assert_eq!(
            filter.check_url_privacy(Some("https://sub.example.com/page")),
            PrivacyAction::Skip
        );
        // 未命中与空值照常记录
        assert_eq!(
            filter.check_url_privacy(Some("https://other.com")),
            PrivacyAction::Record
        );
        assert_eq!(filter.check_url_privacy(None), PrivacyAction::Record);
        assert_eq!(filter.check_url_privacy(Some("")), PrivacyAction::Record);
    }

    #[test]
    fn test_filter_text() {
        let config = PrivacyConfig::default();
        let filter = PrivacyFilter::from_config(&config);

        let text = "My phone is 13812345678 and card is 1234-5678-9012-3456";
        let filtered = filter.filter_text(text);

        assert!(filtered.contains("[已过滤]"));
        assert!(!filtered.contains("13812345678"));
    }

    #[test]
    fn 中文与号码直接相邻时不应漏配() {
        // regex 的 \b 是 Unicode 词边界，CJK 属词字符：中文紧邻号码时旧实现整条漏配
        let config = PrivacyConfig::default();
        let filter = PrivacyFilter::from_config(&config);

        let filtered = filter.filter_text("手机号13812345678请查收");
        assert!(!filtered.contains("13812345678"), "{filtered}");

        let filtered = filter.filter_text("卡号6222020200112233到账");
        assert!(!filtered.contains("6222020200112233"), "{filtered}");

        let filtered = filter.filter_text("邮箱zhang.san@example.com发我");
        assert!(!filtered.contains("zhang.san@example.com"), "{filtered}");

        // 更长数字串（订单号）不算手机号，保持不误伤
        let filtered = filter.filter_text("订单号1138123456780请查收");
        assert!(filtered.contains("1138123456780"), "{filtered}");

        // TLD 后粘着数字（x.com3）属更长 token：与旧 \b 语义一致，不当作邮箱拆掉
        let filtered = filter.filter_text("contact user@x.com3 now");
        assert!(filtered.contains("user@x.com3"), "{filtered}");
    }
}
