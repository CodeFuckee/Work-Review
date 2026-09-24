//! Auto-extracted from the historical `commands.rs`. Behavior unchanged.

use crate::AppState;
use chrono::NaiveDate;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, State};
use work_review_core::analysis::AppLocale;
use work_review_core::config::AppConfig;
use work_review_core::database::DailyReport;
use work_review_core::error::AppError;
use work_review_core::gitlab::{gitlab_unavailable_section, GitlabReportSupplement};

use super::shared::{collect_privacy_filters, filter_activities_by_privacy, persist_app_config};

/// 拉取 GitLab 今日（或指定日报日期）提交并转成日报补充内容。
/// - 未配置 URL/Token 或开关关闭：返回空补充（不生成小节）。
/// - 已配置即生成「GitLab 今日提交」小节：拉取失败/超时（10 秒）降级为
///   失败说明占位小节；无提交为占位说明；成功为提交列表（超限截断并提示）。
async fn fetch_gitlab_supplement(
    config: &AppConfig,
    date: &str,
    locale: AppLocale,
) -> GitlabReportSupplement {
    if !config.gitlab.report_enabled {
        return GitlabReportSupplement::default();
    }
    let (Some(url), Some(token)) = (config.gitlab.url.as_deref(), config.gitlab.token.as_deref())
    else {
        return GitlabReportSupplement::default();
    };
    if url.trim().is_empty() || token.trim().is_empty() {
        return GitlabReportSupplement::default();
    }

    let date_local = NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap_or_else(|_| chrono::Local::now().date_naive());
    let offset = work_review_core::gitlab::local_offset_now();
    let project_filter = config.gitlab.project_filter.clone();
    let work_authors = config.gitlab.work_authors.clone();
    let client = work_review_core::gitlab::GitlabClient::new(url, token);

    match tokio::time::timeout(
        std::time::Duration::from_secs(10),
        client.fetch_today_commits(offset, date_local, &project_filter, &work_authors),
    )
    .await
    {
        Ok(Ok(result)) => result.to_supplement(offset, locale),
        _ => GitlabReportSupplement {
            ai_context: None,
            local_section: gitlab_unavailable_section(locale),
            entries: Vec::new(),
        },
    }
}

fn resolve_saved_report_metadata(
    configured_mode: &work_review_core::config::AiMode,
    configured_model_name: &str,
    used_ai: bool,
) -> (String, Option<String>) {
    let configured_mode = format!("{configured_mode:?}").to_lowercase();

    match (configured_mode.as_str(), used_ai) {
        ("summary", false) => ("local".to_string(), None),
        ("cloud", false) => ("local".to_string(), None),
        (_, false) => (configured_mode, None),
        _ => {
            let model_name = configured_model_name.trim();
            (
                configured_mode,
                if model_name.is_empty() {
                    None
                } else {
                    Some(model_name.to_string())
                },
            )
        }
    }
}

fn update_daily_report_ai_order_cache(
    config: &mut AppConfig,
    ai_order: Option<Vec<String>>,
) -> bool {
    let Some(ai_order) = ai_order else {
        return false;
    };
    if ai_order.is_empty() || config.daily_report_last_ai_order == ai_order {
        return false;
    }

    config.daily_report_last_ai_order = ai_order;
    true
}

fn build_daily_report_export_path(export_dir: &Path, date: &str) -> PathBuf {
    let safe_date = date.replace(['/', '\\'], "-");
    export_dir.join(format!("{safe_date}.md"))
}

fn export_daily_report_markdown(
    export_dir: &Path,
    date: &str,
    content: &str,
) -> Result<(), AppError> {
    let output_path = build_daily_report_export_path(export_dir, date);
    // Retry up to 3 times to handle transient issues (disk busy, antivirus lock, etc.)
    let mut last_err = None;
    for attempt in 0..3u32 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(500 * attempt as u64));
            log::warn!(
                "日报自动导出重试第 {} 次: {}",
                attempt,
                output_path.display()
            );
        }
        match std::fs::create_dir_all(export_dir) {
            Ok(()) => {}
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        }
        match std::fs::write(&output_path, content) {
            Ok(()) => {
                log::info!("日报自动导出成功: {}", output_path.display());
                return Ok(());
            }
            Err(e) => {
                last_err = Some(e);
                continue;
            }
        }
    }
    Err(AppError::Unknown(format!(
        "日报导出失败（已重试3次）: {}",
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "未知错误".to_string())
    )))
}

/// 生成日报
pub(crate) async fn generate_report_inner(
    date: String,
    force: Option<bool>,
    locale: Option<String>,
    app: &AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> Result<String, AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let report_locale_code = report_locale.as_code();
    // 如果不是强制重新生成，先检查缓存
    if !force.unwrap_or(false) {
        let state_guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if let Ok(Some(cached)) = state_guard
            .database
            .get_report(&date, Some(report_locale_code))
        {
            log::info!("使用缓存日报: {date}");
            return Ok(cached.content);
        }
    }

    let (config, stats, activities, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let (ignored_apps, excluded_domains) = collect_privacy_filters(&state);
        let stats = super::stats::load_daily_stats_for_overview(&state, &date)?;
        // 生成日报时获取最多 2000 条记录
        let raw_activities = state.database.get_timeline(&date, Some(2000), None)?;
        let activities =
            filter_activities_by_privacy(raw_activities, &ignored_apps, &excluded_domains);
        (
            state.config.clone(),
            stats,
            activities,
            state.data_dir.clone(),
        )
    };

    // 文本模型兜底：用户未配置（出厂默认）且编译期注入了内置模型时，改用内置模型生成日报
    let text_model = work_review_core::config::effective_text_model(&config.text_model);

    let report_timeout =
        std::time::Duration::from_secs(config.report_generation_timeout_secs.max(60));

    // 创建分析器（使用兜底解析后的 text_model 配置）
    let analyzer = work_review_core::analysis::create_analyzer(
        config.ai_mode,
        text_model.provider,
        &text_model.endpoint,
        &text_model.model,
        text_model.api_key.as_deref(),
        &config.daily_report_custom_prompt,
        config.daily_report_system_prompt_override.as_deref(),
        report_locale,
        config.daily_report_pinned_blocks.clone(),
        if config.daily_report_last_ai_order.is_empty() {
            None
        } else {
            Some(config.daily_report_last_ai_order.clone())
        },
        report_timeout,
    );

    // 生成报告（spawn 隔离 panic，防止内部错误杀死整个 tokio 线程）
    // 外层超时可配置（默认 300 秒），到期 abort 后台任务。
    let screenshots_dir = data_dir.clone();
    let date_gen = date.clone();
    let category_name_overrides: std::collections::HashMap<String, String> = config
        .custom_categories
        .iter()
        .map(|c| (c.key.clone(), c.name.clone()))
        .collect();
    let semantic_name_overrides: std::collections::HashMap<String, String> = config
        .custom_semantic_categories
        .iter()
        .map(|c| (c.key.clone(), c.name.clone()))
        .collect();

    // 拉取该日报日期（默认今日）的私有化 GitLab 提交，作为日报补充内容。
    // 已配置即生成「GitLab 今日提交」独立小节（失败/无提交为占位说明），
    // 未配置为空补充；失败不阻塞日报生成。
    let gitlab_supplement = fetch_gitlab_supplement(&config, &date, report_locale).await;

    // 提交入库（时间线页与日报时间线共用数据源）；入库失败仅记日志不阻塞日报
    if !gitlab_supplement.entries.is_empty() {
        let rows = work_review_core::gitlab::entries_to_rows(&gitlab_supplement.entries, &date);
        let state_guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if let Err(error) = state_guard.database.upsert_gitlab_commits(&rows) {
            log::warn!("GitLab 提交入库失败（不影响日报生成）: {error}");
        }
    }

    // 「Plane 工作项」小节：读当日已入库数据（由时间线页补采写入，日报不现拉
    let spawn_result = tokio::spawn(async move {
        analyzer
            .generate_report(
                &date_gen,
                &stats,
                &activities,
                &screenshots_dir,
                report_locale,
                category_name_overrides,
                semantic_name_overrides,
                &gitlab_supplement,
            )
            .await
    });
    let abort = spawn_result.abort_handle();

    let report_result = match tokio::time::timeout(report_timeout, spawn_result).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(work_review_core::error::AppError::Analysis(
            match report_locale {
                AppLocale::ZhCn => "日报生成过程中发生内部错误，请重试".to_string(),
                AppLocale::ZhTw => "日報生成過程中發生內部錯誤，請重試".to_string(),
                AppLocale::En => {
                    "Internal error during report generation, please retry".to_string()
                }
                AppLocale::Ar => {
                    "حدث خطأ داخلي أثناء إنشاء التقرير، يرجى المحاولة مرة أخرى".to_string()
                }
            },
        )),
        Err(_) => {
            abort.abort();
            Err(work_review_core::error::AppError::Analysis(
                match report_locale {
                    AppLocale::ZhCn => "日报生成超时，请稍后重试".to_string(),
                    AppLocale::ZhTw => "日報生成逾時，請稍後重試".to_string(),
                    AppLocale::En => {
                        "Report generation timed out, please try again later".to_string()
                    }
                    AppLocale::Ar => {
                        "انتهت مهلة إنشاء التقرير، يرجى المحاولة مرة أخرى لاحقاً".to_string()
                    }
                },
            ))
        }
    };

    let generated_report = report_result?;
    let report = generated_report.content.clone();
    let (saved_ai_mode, saved_model_name) =
        resolve_saved_report_metadata(&config.ai_mode, &text_model.model, generated_report.used_ai);

    // 保存报告
    {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let daily_report = DailyReport {
            date: date.clone(),
            locale: report_locale_code.to_string(),
            content: report.clone(),
            ai_mode: saved_ai_mode,
            model_name: saved_model_name,
            fallback_reason: generated_report.fallback_reason.clone(),
            created_at: chrono::Utc::now().timestamp(),
        };
        state.database.save_report(&daily_report)?;
    }

    if let Some(ai_order) = generated_report.ai_order.clone() {
        let config_to_persist = {
            let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
            let mut next_config = state.config.clone();
            if update_daily_report_ai_order_cache(&mut next_config, Some(ai_order)) {
                Some(next_config)
            } else {
                None
            }
        };

        if let Some(next_config) = config_to_persist {
            if let Err(error) = persist_app_config(next_config, app.clone(), state) {
                log::warn!("缓存 AI 段落编排顺序失败: {error}");
            }
        }
    }

    if config.daily_report_auto_export {
        if let Some(export_dir) = config.daily_report_export_dir.as_deref() {
            // 与手动/合并导出同款敏感位置校验（拒绝 .ssh、LaunchAgents 等目录，
            // 防止配置同步载荷把导出指向敏感位置）。自动导出目录允许尚不存在
            // （导出函数会 create_dir_all），故只做组件检查、不走要求已存在的
            // validate_export_dir；失败不传播——日报已落库，仅导出文件失败
            let export_result = ensure_export_dir_components_safe(Path::new(export_dir))
                .and_then(|()| export_daily_report_markdown(Path::new(export_dir), &date, &report));
            if let Err(e) = export_result {
                log::error!("日报自动导出失败（日报已保存到数据库，仅导出文件失败）: {e:?}");
            }
        }
    }

    Ok(report)
}

struct ReportGenerationGuard {
    state: Arc<Mutex<AppState>>,
}

impl Drop for ReportGenerationGuard {
    fn drop(&mut self) {
        if let Ok(mut s) = self.state.lock() {
            s.generating_report = false;
        }
    }
}

#[tauri::command]
pub async fn generate_report(
    date: String,
    force: Option<bool>,
    locale: Option<String>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    {
        let mut s = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        if s.generating_report {
            return Err(AppError::Unknown("日报正在生成中，请稍候".to_string()));
        }
        s.generating_report = true;
    }
    let _guard = ReportGenerationGuard {
        state: state.inner().clone(),
    };
    generate_report_inner(date, force, locale, &app, state.inner()).await
}

/// 获取已保存的日报
pub(crate) fn get_saved_report_inner(
    date: String,
    locale: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<Option<DailyReport>, AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let saved = state
        .database
        .get_report(&date, Some(report_locale.as_code()))?;
    let Some(mut report) = saved else {
        return Ok(None);
    };

    // 用最新的 stats 重新渲染统计区块，解决 issue #80：保存的 markdown 里固化的时长
    // 数字会随着工作日继续推进而变得陈旧。老报告若没有占位符标记则原样返回。
    if let Ok(live_stats) = super::stats::load_daily_stats_for_overview(&state, &date) {
        let category_name_overrides: std::collections::HashMap<String, String> = state
            .config
            .custom_categories
            .iter()
            .map(|c| (c.key.clone(), c.name.clone()))
            .collect();
        let semantic_name_overrides: std::collections::HashMap<String, String> = state
            .config
            .custom_semantic_categories
            .iter()
            .map(|c| (c.key.clone(), c.name.clone()))
            .collect();
        report.content = work_review_core::analysis::report_blocks::render_report_with_live_stats(
            &report.content,
            &live_stats,
            report_locale,
            &category_name_overrides,
            &semantic_name_overrides,
        );
    }

    Ok(Some(report))
}

#[tauri::command]
pub async fn get_saved_report(
    date: String,
    locale: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Option<DailyReport>, AppError> {
    get_saved_report_inner(date, locale, state.inner())
}

/// 更新已保存日报的内容（用于结构化编辑）
#[tauri::command]
pub async fn update_report_content(
    date: String,
    locale: Option<String>,
    content: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let locale_code = report_locale.as_code();
    let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
    let existing = state
        .database
        .get_report(&date, Some(locale_code))?
        .ok_or_else(|| {
            AppError::Database(rusqlite::Error::InvalidParameterName(
                "报告不存在".to_string(),
            ))
        })?;
    let updated = DailyReport {
        content,
        ..existing
    };
    state.database.save_report(&updated)?;
    Ok(())
}

/// 设置日报段落的钉选/隐藏偏好
#[tauri::command]
pub async fn set_report_block_preference(
    pinned_blocks: Vec<String>,
    hidden_blocks: Vec<String>,
    app: AppHandle,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), AppError> {
    let config = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state.config.daily_report_pinned_blocks = pinned_blocks;
        state.config.daily_report_hidden_blocks = hidden_blocks;
        state.config.clone()
    };
    persist_app_config(config, app, state.inner())?;
    Ok(())
}

/// 校验规范化后的导出目录路径组件是否安全（纯函数，便于测试）：
/// - 任何组件不得以 '.' 开头（.ssh/.config/.config/autostart 等隐藏目录一并覆盖）
/// - 拒绝 Library/LaunchAgents、Library/LaunchDaemons 等自启动敏感目录
fn ensure_export_dir_components_safe(canonical: &Path) -> Result<(), AppError> {
    let components: Vec<String> = canonical
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    if components.iter().any(|name| name.starts_with('.')) {
        return Err(AppError::Config(
            "导出目录不能位于以 . 开头的隐藏目录内（如 .ssh、.config）".to_string(),
        ));
    }

    for pair in components.windows(2) {
        let parent = pair[0].to_ascii_lowercase();
        let child = pair[1].to_ascii_lowercase();
        if parent == "library" && (child == "launchagents" || child == "launchdaemons") {
            return Err(AppError::Config(
                "导出目录位于系统自启动目录，已拒绝写入".to_string(),
            ));
        }
    }
    Ok(())
}

/// 校验日报导出目录：必须是已存在的真实目录，且不落在隐藏目录 / 自启动等敏感位置。
/// 导出目录可由本地 API 请求方或前端传入，写入前必须校验，防止向 .ssh、
/// LaunchAgents 等位置写文件。返回规范化（已解析符号链接）后的路径。
pub(crate) fn validate_export_dir(dir: &Path) -> Result<PathBuf, AppError> {
    let canonical = dir
        .canonicalize()
        .map_err(|_| AppError::Config("导出目录不存在或无法访问，请先创建该目录".to_string()))?;
    if !canonical.is_dir() {
        return Err(AppError::Config("导出路径必须是已存在的目录".to_string()));
    }
    ensure_export_dir_components_safe(&canonical)?;
    Ok(canonical)
}

pub(crate) fn export_report_markdown_inner(
    date: String,
    content: Option<String>,
    export_dir: Option<String>,
    locale: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<String, AppError> {
    let report_locale = AppLocale::from_option(locale.as_deref());
    let (export_dir, saved_content) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let requested_export_dir = export_dir
            .as_deref()
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(|dir| dir.to_string());
        let configured_export_dir = state
            .config
            .daily_report_export_dir
            .as_deref()
            .map(str::trim)
            .filter(|dir| !dir.is_empty())
            .map(|dir| dir.to_string());
        let export_dir = requested_export_dir
            .or(configured_export_dir)
            .ok_or_else(|| {
                AppError::Config(
                    "请先选择导出目录，或在设置中配置日报 Markdown 导出目录".to_string(),
                )
            })?;
        let saved_content = if let Some(content) = content {
            content
        } else {
            state
                .database
                .get_report(&date, Some(report_locale.as_code()))?
                .ok_or_else(|| AppError::Config("未找到可导出的日报".to_string()))?
                .content
        };
        (export_dir, saved_content)
    };

    // 目录来自请求方/配置，写入前校验（要求已存在，拒绝隐藏/敏感目录）
    let export_dir_path = validate_export_dir(Path::new(&export_dir))?;
    export_daily_report_markdown(&export_dir_path, &date, &saved_content)?;
    Ok(build_daily_report_export_path(&export_dir_path, &date)
        .to_string_lossy()
        .to_string())
}

#[tauri::command]
pub async fn export_report_markdown(
    date: String,
    content: Option<String>,
    export_dir: Option<String>,
    locale: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<String, AppError> {
    export_report_markdown_inner(date, content, export_dir, locale, state.inner())
}

/// 验证 ISO `YYYY-MM-DD` 日期字符串
fn ensure_iso_date(value: &str, field: &str) -> Result<(), AppError> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map(|_| ())
        .map_err(|_| AppError::Config(format!("{field} 日期格式应为 YYYY-MM-DD")))
}

/// 合并导出 Markdown 的脚手架文案（标题、字段名、冒号），按界面语言输出：
/// [标题, 日期范围, 导出时间, 日报数量, 语言, 冒号]
fn range_export_scaffold_labels(locale: AppLocale) -> [&'static str; 6] {
    match locale {
        AppLocale::ZhCn => [
            "工作日报合并导出",
            "日期范围",
            "导出时间",
            "日报数量",
            "语言",
            "：",
        ],
        AppLocale::ZhTw => [
            "工作日報合併導出",
            "日期範圍",
            "導出時間",
            "日報數量",
            "語言",
            "：",
        ],
        AppLocale::En => [
            "Merged Daily Reports Export",
            "Date range",
            "Exported at",
            "Report count",
            "Language",
            ": ",
        ],
        AppLocale::Ar => [
            "تصدير التقارير اليومية المدمجة",
            "نطاق التاريخ",
            "وقت التصدير",
            "عدد التقارير",
            "اللغة",
            ": ",
        ],
    }
}

/// 将范围内的日报合并成一个 Markdown 文件
///
/// - 范围按 ISO 日期字符串比较（lexicographic 与日期序一致）
/// - locale 默认 zh-CN，与 `get_report` 一致
/// - 范围内一个日报都没有时返回错误，避免写出空文件让用户困惑
pub(crate) fn export_reports_range_inner(
    start_date: String,
    end_date: String,
    target_path: String,
    locale: Option<String>,
    state: &Arc<Mutex<AppState>>,
) -> Result<(String, usize), AppError> {
    let start = start_date.trim();
    let end = end_date.trim();
    let target = target_path.trim();
    if start.is_empty() || end.is_empty() {
        return Err(AppError::Config("起止日期不能为空".to_string()));
    }
    if target.is_empty() {
        return Err(AppError::Config("请先选择导出文件路径".to_string()));
    }
    ensure_iso_date(start, "起始")?;
    ensure_iso_date(end, "结束")?;
    if start > end {
        return Err(AppError::Config("起始日期不能晚于结束日期".to_string()));
    }

    let report_locale =
        AppLocale::from_option(locale.as_deref().map(str::trim).filter(|s| !s.is_empty()));
    let locale_code = report_locale.as_code();

    let reports = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        state
            .database
            .get_reports_in_range(start, end, Some(locale_code))?
    };

    if reports.is_empty() {
        return Err(AppError::Config(format!(
            "{start} 至 {end} 范围内未找到日报"
        )));
    }

    // 脚手架文案随界面语言切换，避免非中文用户导出中文标题/字段名
    let [title, range_label, exported_at_label, count_label, language_label, colon] =
        range_export_scaffold_labels(report_locale);
    let mut markdown = String::new();
    markdown.push_str(&format!("# {title}\n\n"));
    markdown.push_str(&format!("- {range_label}{colon}{start} ~ {end}\n"));
    markdown.push_str(&format!(
        "- {exported_at_label}{colon}{}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    ));
    markdown.push_str(&format!("- {count_label}{colon}{}\n", reports.len()));
    markdown.push_str(&format!("- {language_label}{colon}{locale_code}\n\n"));
    markdown.push_str("---\n\n");

    for report in &reports {
        markdown.push_str(&format!("## {}\n\n", report.date));
        markdown.push_str(report.content.trim());
        markdown.push_str("\n\n---\n\n");
    }

    let target_path_buf = PathBuf::from(target);
    if let Some(parent) = target_path_buf.parent() {
        if !parent.as_os_str().is_empty() {
            // 目录来自请求方，与单篇导出同样需要校验（拒绝隐藏/敏感目录，
            // 防止向 .ssh、LaunchAgents 等位置写文件），缺失时才创建
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
            validate_export_dir(parent)?;
        }
    }
    std::fs::write(&target_path_buf, markdown)?;
    Ok((target_path_buf.to_string_lossy().to_string(), reports.len()))
}

#[tauri::command]
pub async fn export_reports_range(
    start_date: String,
    end_date: String,
    target_path: String,
    locale: Option<String>,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<ExportReportsRangeResult, AppError> {
    let (path, count) =
        export_reports_range_inner(start_date, end_date, target_path, locale, state.inner())?;
    Ok(ExportReportsRangeResult { path, count })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReportsRangeResult {
    pub path: String,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use work_review_core::config::AiMode;

    #[tokio::test]
    async fn gitlab未配置时日报补充内容应为空() {
        let config = AppConfig::default();
        let supplement = fetch_gitlab_supplement(&config, "2026-09-02", AppLocale::ZhCn).await;
        assert_eq!(supplement, GitlabReportSupplement::default());
    }

    #[tokio::test]
    async fn gitlab开关关闭时即使已配置也不请求() {
        let mut config = AppConfig::default();
        config.gitlab.url = Some("https://git.example.com".to_string());
        config.gitlab.token = Some("stub-token".to_string());
        config.gitlab.report_enabled = false;
        let supplement = fetch_gitlab_supplement(&config, "2026-09-02", AppLocale::ZhCn).await;
        assert_eq!(supplement, GitlabReportSupplement::default());
    }

    #[tokio::test]
    async fn gitlab空白配置应视为未配置() {
        let mut config = AppConfig::default();
        config.gitlab.url = Some("   ".to_string());
        config.gitlab.token = Some("stub".to_string());
        let supplement = fetch_gitlab_supplement(&config, "2026-09-02", AppLocale::ZhCn).await;
        assert_eq!(supplement, GitlabReportSupplement::default());

        let mut missing_token = AppConfig::default();
        missing_token.gitlab.url = Some("https://git.example.com".to_string());
        let supplement = fetch_gitlab_supplement(&missing_token, "2026-09-02", AppLocale::En).await;
        assert_eq!(supplement, GitlabReportSupplement::default());
    }

    #[test]
    fn plane配置不齐全时日报不应产出工作项小节() {
        use super::super::plane::plane_configured;

        let mut config = AppConfig::default();
        assert!(!plane_configured(&config.plane), "默认配置未启用");

        config.plane.enabled = true;
        config.plane.url = Some("https://plane.example.com".to_string());
        config.plane.api_key = Some("stub-key".to_string());
        config.plane.workspace_slug = Some("my-team".to_string());
        assert!(plane_configured(&config.plane), "配置齐全应产出小节");

        config.plane.enabled = false;
        assert!(
            !plane_configured(&config.plane),
            "开关关闭时即使配置齐全也不产出"
        );
    }

    #[test]
    fn summary回退到基础模板时不应保留_ai_模型标签() {
        let (ai_mode, model_name) =
            resolve_saved_report_metadata(&AiMode::Summary, "gpt-5.4", false);

        assert_eq!(ai_mode, "local");
        assert_eq!(model_name, None);
    }

    #[test]
    fn 新_ai_段落顺序应缓存到配置且空结果不覆盖已有缓存() {
        let mut config = work_review_core::config::AppConfig::default();
        let first_order = vec!["APP_USAGE_TABLE".to_string(), "CATEGORY_TABLE".to_string()];

        assert!(update_daily_report_ai_order_cache(
            &mut config,
            Some(first_order.clone())
        ));
        assert_eq!(config.daily_report_last_ai_order, first_order);

        assert!(!update_daily_report_ai_order_cache(&mut config, None));
        assert_eq!(
            config.daily_report_last_ai_order,
            vec!["APP_USAGE_TABLE".to_string(), "CATEGORY_TABLE".to_string(),]
        );

        assert!(!update_daily_report_ai_order_cache(
            &mut config,
            Some(Vec::new())
        ));
        assert_eq!(
            config.daily_report_last_ai_order,
            vec!["APP_USAGE_TABLE".to_string(), "CATEGORY_TABLE".to_string(),]
        );
    }

    #[test]
    fn ai成功生成时应保留实际配置的模式与模型() {
        let (ai_mode, model_name) =
            resolve_saved_report_metadata(&AiMode::Summary, "gpt-5.4", true);

        assert_eq!(ai_mode, "summary");
        assert_eq!(model_name, Some("gpt-5.4".to_string()));
    }

    #[test]
    fn 日报导出路径应按日期生成_markdown_文件名() {
        let export_path = build_daily_report_export_path(Path::new("/tmp/reports"), "2026-03-29");

        assert_eq!(
            export_path,
            PathBuf::from("/tmp/reports").join("2026-03-29.md")
        );
    }

    #[test]
    fn 日报导出应写入_markdown_文件() {
        let temp_dir =
            std::env::temp_dir().join(format!("work-review-export-{}", uuid::Uuid::new_v4()));
        export_daily_report_markdown(&temp_dir, "2026-03-29", "# 工作日报\n\n测试内容")
            .expect("应能导出 Markdown");

        let output_path = temp_dir.join("2026-03-29.md");
        let content = std::fs::read_to_string(&output_path).expect("应能读取导出内容");
        assert_eq!(content, "# 工作日报\n\n测试内容");

        let _ = std::fs::remove_file(&output_path);
        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn 导出目录组件校验应拦截隐藏目录与自启动目录() {
        use super::ensure_export_dir_components_safe;

        assert!(
            ensure_export_dir_components_safe(Path::new("/Users/demo/Documents/reports")).is_ok()
        );
        assert!(ensure_export_dir_components_safe(Path::new("/Users/demo/.ssh")).is_err());
        assert!(
            ensure_export_dir_components_safe(Path::new("/home/demo/.config/autostart")).is_err()
        );
        assert!(
            ensure_export_dir_components_safe(Path::new("/Users/demo/Library/LaunchAgents"))
                .is_err()
        );
        assert!(ensure_export_dir_components_safe(Path::new(
            "/Users/demo/Library/LaunchDaemons/sub"
        ))
        .is_err());
        // Library 下的普通目录不受影响
        assert!(ensure_export_dir_components_safe(Path::new(
            "/Users/demo/Library/CloudStorage/reports"
        ))
        .is_ok());
    }

    #[test]
    fn 导出目录必须已存在() {
        use super::validate_export_dir;

        assert!(
            validate_export_dir(Path::new("/nonexistent-work-review-export-dir-a1b2c3")).is_err()
        );
    }
}
