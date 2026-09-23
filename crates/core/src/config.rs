use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn default_true() -> bool {
    true
}

fn default_locale() -> String {
    "zh-CN".to_string()
}

/// AI 提供商类型
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AiProvider {
    /// 本地 Ollama
    #[default]
    Ollama,
    /// OpenAI / OpenAI Compatible
    OpenAI,
    /// Google Gemini
    Gemini,
    /// Anthropic Claude
    Claude,
    /// 硅基流动 SiliconFlow
    #[serde(rename = "siliconflow")]
    SiliconFlow,
    /// DeepSeek
    #[serde(rename = "deepseek")]
    DeepSeek,
    /// 通义千问 Qwen
    #[serde(rename = "qwen")]
    Qwen,
    /// 智谱 ChatGLM
    #[serde(rename = "zhipu")]
    Zhipu,
    /// 月之暗面 Moonshot
    #[serde(rename = "moonshot")]
    Moonshot,
    /// 火山引擎 豆包
    #[serde(rename = "doubao")]
    Doubao,
    /// 稀宇科技 MiniMax
    #[serde(rename = "minimax")]
    MiniMax,
    /// OpenRouter（多模型聚合）
    #[serde(rename = "openrouter")]
    OpenRouter,
    /// Groq（高速推理）
    #[serde(rename = "groq")]
    Groq,
    /// xAI Grok
    #[serde(rename = "xai")]
    XAI,
    /// Mistral AI
    #[serde(rename = "mistral")]
    Mistral,
    /// LM Studio（本地）
    #[serde(rename = "lmstudio")]
    LmStudio,
    /// 自定义 OpenAI 兼容接口
    #[serde(rename = "custom")]
    Custom,
    /// 软件内置的兜底大模型（编译期注入，无需任何配置）
    #[serde(rename = "builtin")]
    Builtin,
}

impl AiProvider {
    /// 获取提供商的显示名称（仅用于新建 profile 时的默认命名，见 `default_profile_name`）。
    ///
    /// 运行时显示由前端 `providerDisplayNames`（Ask.svelte）按 locale 提供，
    /// 这里只有中文一种，已不再作为运行时显示来源。
    #[deprecated(
        note = "运行时显示请用前端 providerDisplayNames；此处仅保留用于 default_profile_name 的初始命名"
    )]
    pub fn display_name(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "Ollama (本地)",
            AiProvider::OpenAI => "OpenAI / 兼容API",
            AiProvider::Gemini => "Google Gemini",
            AiProvider::Claude => "Anthropic Claude",
            AiProvider::SiliconFlow => "硅基流动 SiliconFlow",
            AiProvider::DeepSeek => "DeepSeek",
            AiProvider::Qwen => "通义千问 Qwen",
            AiProvider::Zhipu => "智谱 ChatGLM",
            AiProvider::Moonshot => "月之暗面 Kimi",
            AiProvider::Doubao => "火山引擎 豆包",
            AiProvider::MiniMax => "稀宇科技 MiniMax",
            AiProvider::OpenRouter => "OpenRouter",
            AiProvider::Groq => "Groq",
            AiProvider::XAI => "xAI Grok",
            AiProvider::Mistral => "Mistral",
            AiProvider::LmStudio => "LM Studio (本地)",
            AiProvider::Custom => "自定义接口",
            AiProvider::Builtin => "内置模型",
        }
    }

    /// 获取默认的 API 地址
    pub fn default_endpoint(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "http://localhost:11434",
            AiProvider::OpenAI => "https://api.openai.com/v1",
            AiProvider::Gemini => "https://generativelanguage.googleapis.com/v1",
            AiProvider::Claude => "https://api.anthropic.com/v1",
            AiProvider::SiliconFlow => "https://api.siliconflow.cn/v1",
            AiProvider::DeepSeek => "https://api.deepseek.com",
            AiProvider::Qwen => "https://dashscope.aliyuncs.com/compatible-mode/v1",
            AiProvider::Zhipu => "https://open.bigmodel.cn/api/paas/v4",
            AiProvider::Moonshot => "https://api.moonshot.cn/v1",
            AiProvider::Doubao => "https://ark.cn-beijing.volces.com/api/v3",
            AiProvider::MiniMax => "https://api.minimaxi.com/v1",
            AiProvider::OpenRouter => "https://openrouter.ai/api/v1",
            AiProvider::Groq => "https://api.groq.com/openai/v1",
            AiProvider::XAI => "https://api.x.ai/v1",
            AiProvider::Mistral => "https://api.mistral.ai/v1",
            AiProvider::LmStudio => "http://localhost:1234/v1",
            AiProvider::Custom => "",
            AiProvider::Builtin => "",
        }
    }

    /// 获取默认模型名称
    pub fn default_model(&self) -> &'static str {
        match self {
            AiProvider::Ollama => "qwen3",
            AiProvider::OpenAI => "gpt-5.4",
            AiProvider::Gemini => "gemini-3-flash",
            AiProvider::Claude => "claude-sonnet-4-6",
            AiProvider::SiliconFlow => "Qwen/Qwen3-8B",
            AiProvider::DeepSeek => "deepseek-chat",
            AiProvider::Qwen => "qwen-flash",
            AiProvider::Zhipu => "glm-5-turbo",
            AiProvider::Moonshot => "moonshot-v1-8k",
            AiProvider::Doubao => "doubao-lite-4k",
            AiProvider::MiniMax => "MiniMax-M2.5",
            AiProvider::OpenRouter => "openrouter/auto",
            AiProvider::Groq => "llama-3.3-70b-versatile",
            AiProvider::XAI => "grok-2-latest",
            AiProvider::Mistral => "mistral-small-latest",
            AiProvider::LmStudio => "local-model",
            AiProvider::Custom => "",
            AiProvider::Builtin => "",
        }
    }

    /// 是否使用 OpenAI 兼容格式
    pub fn is_openai_compatible(&self) -> bool {
        matches!(
            self,
            AiProvider::OpenAI
                | AiProvider::SiliconFlow
                | AiProvider::DeepSeek
                | AiProvider::Qwen
                | AiProvider::Zhipu
                | AiProvider::Moonshot
                | AiProvider::Doubao
                | AiProvider::MiniMax
                | AiProvider::OpenRouter
                | AiProvider::Groq
                | AiProvider::XAI
                | AiProvider::Mistral
                | AiProvider::LmStudio
                | AiProvider::Custom
                | AiProvider::Builtin
        )
    }
}

/// AI分析模式
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AiMode {
    /// 本地多模态模型（分析截图）
    #[default]
    Local,
    /// 摘要模式（只上传统计摘要）
    Summary,
    /// 云端视觉模型（上传截图到云端分析）
    Cloud,
}

/// 单个模型配置（独立的提供商、地址、密钥、模型名）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    /// 提供商类型
    pub provider: AiProvider,
    /// API 地址
    pub endpoint: String,
    /// API Key
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
    /// 思考模式：None=跟随服务端，Some(true/false)=显式开关
    #[serde(default)]
    pub enable_thinking: Option<bool>,
    /// 思考 token 预算；0 或空表示不发送
    #[serde(default)]
    pub thinking_budget: Option<u32>,
    /// 最大输出 token；0 或空表示使用链路默认值
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
}

impl ModelConfig {
    /// 创建默认的文本模型配置
    /// 注意：model 默认为空，强制用户手动配置，避免 UI 误显示"已配置"
    pub fn default_text() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: String::new(), // 默认为空，用户需手动填写
            enable_thinking: None,
            thinking_budget: None,
            max_output_tokens: None,
        }
    }

    /// 创建默认的视觉模型配置
    pub fn default_vision() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: "llava".to_string(),
            enable_thinking: None,
            thinking_budget: None,
            max_output_tokens: None,
        }
    }

    /// 出厂默认判定：从未被用户修改过的文本模型配置视为「未配置」。
    ///
    /// 覆盖两种形态：
    /// 1. `default_text()` 原样（Ollama 本地端点 + 空 Key + 空模型名）；
    /// 2. 旧版迁移/前端默认值产生的幽灵配置——模型名恰为 Ollama 默认模型
    ///    （`migrate_legacy_config` 会把旧版 `ai_provider` 的 qwen3 迁入空的
    ///    `text_model`，且随后保存落盘，从未配置过的用户磁盘上即是此形态）。
    ///
    /// 真实配置过的用户要么换过端点/填了 Key，要么从模型列表选过具体名字
    /// （如 `qwen3:8b`），不会被误判。
    pub fn is_factory_default_text(&self) -> bool {
        // 内置模型供应商：等价于「使用内置」，无论端点/模型字段为何
        if self.provider == AiProvider::Builtin {
            return true;
        }
        if self.provider != AiProvider::Ollama
            || self.endpoint.trim() != AiProvider::Ollama.default_endpoint()
            || !self
                .api_key
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return false;
        }
        let model = self.model.trim();
        model.is_empty() || model == AiProvider::Ollama.default_model()
    }
}

/// 内置兜底模型（编译期注入，不写入源码、不落入配置文件）。
///
/// API Key 通过编译期环境变量 `WORK_REVIEW_BUILTIN_MODEL_API_KEY` 注入（CI 变量）；
/// 本地开发未设置该变量时返回 `None`，所有调用点的行为与未引入兜底机制前完全一致。
pub fn builtin_text_model() -> Option<ModelConfig> {
    builtin_text_model_with(option_env!("WORK_REVIEW_BUILTIN_MODEL_API_KEY"))
}

fn builtin_text_model_with(api_key: Option<&'static str>) -> Option<ModelConfig> {
    let api_key = api_key.map(str::trim).filter(|key| !key.is_empty())?;
    Some(ModelConfig {
        provider: AiProvider::OpenAI,
        endpoint: "https://laneai.dev/v1".to_string(),
        api_key: Some(api_key.to_string()),
        model: "deepseek-v4-flash".to_string(),
        enable_thinking: None,
        thinking_budget: None,
        max_output_tokens: None,
    })
}

/// 文本模型兜底解析：用户已配置 → 原样返回；出厂默认（未配置）且编译期注入了内置模型 → 内置模型。
pub fn effective_text_model(config: &ModelConfig) -> ModelConfig {
    effective_text_model_with(builtin_text_model().as_ref(), config)
}

fn effective_text_model_with(builtin: Option<&ModelConfig>, config: &ModelConfig) -> ModelConfig {
    if config.is_factory_default_text() {
        if let Some(builtin) = builtin {
            return builtin.clone();
        }
    }
    config.clone()
}

/// 可保存的文本模型档案
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextModelProfile {
    /// 档案唯一标识
    pub id: String,
    /// 档案显示名称
    pub name: String,
    /// 对应模型配置
    pub model_config: ModelConfig,
    /// 最近一次测试状态：untested / success / error
    #[serde(default = "default_connection_status")]
    pub test_status: String,
    /// 最近一次测试时间（毫秒时间戳）
    #[serde(default)]
    pub last_tested_at: Option<u64>,
    /// 最近一次测试结果描述
    #[serde(default)]
    pub last_test_message: Option<String>,
}

/// AI 提供商配置（旧版，保留用于向后兼容）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AiProviderConfig {
    /// 提供商类型
    pub provider: AiProvider,
    /// API 地址（可自定义，支持代理）
    pub endpoint: String,
    /// API Key
    pub api_key: Option<String>,
    /// 模型名称
    pub model: String,
    /// 视觉模型名称（用于分析截图）
    pub vision_model: Option<String>,
}

impl Default for AiProviderConfig {
    fn default() -> Self {
        Self {
            provider: AiProvider::Ollama,
            endpoint: AiProvider::Ollama.default_endpoint().to_string(),
            api_key: None,
            model: AiProvider::Ollama.default_model().to_string(),
            vision_model: Some("llava".to_string()),
        }
    }
}

/// 应用隐私级别
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PrivacyLevel {
    /// 完全记录（截图 + 统计）
    #[default]
    Full,
    /// 内容脱敏（只统计时长，不保存截图）
    Anonymized,
    /// 完全忽略（不记录任何信息）
    Ignored,
}

/// 应用隐私规则
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppPrivacyRule {
    /// 应用名称
    pub app_name: String,
    /// 隐私级别（默认为 Full，兼容旧配置）
    #[serde(default)]
    pub level: PrivacyLevel,
}

/// 应用分类规则
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct AppCategoryRule {
    /// 应用名称
    pub app_name: String,
    /// 目标分类
    pub category: String,
}

/// 用户自定义分类
#[derive(Serialize, Deserialize, Debug, Clone, Hash)]
pub struct CustomCategory {
    /// 唯一标识（slug 格式，如 "project-mgmt"）
    pub key: String,
    /// 显示名称（用户输入，如 "项目管理"）
    pub name: String,
    /// 颜色（hex 格式，如 "#8B5CF6"）
    pub color: String,
    /// 图标（emoji，如 "📋"）
    pub icon: String,
}

/// 用户自定义语义分类
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CustomSemanticCategory {
    /// 唯一标识（slug 格式，如 "project-mgmt"）
    pub key: String,
    /// 显示名称（用户输入，如 "项目管理"）
    pub name: String,
}

/// 日报提示词预设模板
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PromptPreset {
    /// 预设名称
    pub name: String,
    /// 提示词内容
    pub prompt: String,
}

/// 网站语义分类规则
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebsiteSemanticRule {
    /// 域名
    pub domain: String,
    /// 目标语义分类
    pub semantic_category: String,
}

/// 隐私配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PrivacyConfig {
    /// 应用隐私规则列表
    #[serde(default)]
    pub app_rules: Vec<AppPrivacyRule>,
    /// 排除的窗口标题关键词（触发时使用 Anonymized 级别）
    #[serde(default)]
    pub excluded_keywords: Vec<String>,
    /// URL 域名黑名单（匹配时完全忽略，不记录）
    #[serde(default)]
    pub excluded_domains: Vec<String>,
    /// 已弃用：敏感词过滤始终启用，此字段仅保留反序列化兼容
    #[serde(default = "default_true")]
    pub filter_sensitive: bool,

    // 兼容旧版
    #[serde(default)]
    pub excluded_apps: Vec<String>,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            app_rules: vec![
                AppPrivacyRule {
                    app_name: "1Password".to_string(),
                    level: PrivacyLevel::Ignored,
                },
                AppPrivacyRule {
                    app_name: "Bitwarden".to_string(),
                    level: PrivacyLevel::Ignored,
                },
                AppPrivacyRule {
                    app_name: "Keychain".to_string(),
                    level: PrivacyLevel::Ignored,
                },
            ],
            excluded_keywords: vec![
                "bank".to_string(),
                "login".to_string(),
                "password".to_string(),
                "密码".to_string(),
                "银行".to_string(),
                "支付".to_string(),
            ],
            excluded_domains: vec![], // 默认无域名黑名单
            filter_sensitive: true,   // 已弃用，保留兼容
            excluded_apps: vec![],
        }
    }
}

impl PrivacyConfig {
    /// 获取应用的隐私级别
    pub fn get_app_privacy_level(&self, app_name: &str) -> PrivacyLevel {
        let normalized = crate::categorize::normalize_display_app_name(app_name).to_lowercase();
        // 先检查新的规则（规范化后精确匹配 + 包含匹配）
        for rule in &self.app_rules {
            let rule_normalized =
                crate::categorize::normalize_display_app_name(&rule.app_name).to_lowercase();
            // 精确匹配或包含匹配（应用名包含规则名，处理 "Google Chrome" 包含 "Chrome" 的情况）
            if normalized == rule_normalized || normalized.contains(&rule_normalized) {
                log::debug!(
                    "应用 {} 匹配规则 {}, 级别: {:?}",
                    app_name,
                    rule.app_name,
                    rule.level
                );
                return rule.level;
            }
        }
        // 兼容旧版 excluded_apps（视为 Ignored）
        for excluded in &self.excluded_apps {
            let excluded_normalized =
                crate::categorize::normalize_display_app_name(excluded).to_lowercase();
            if normalized.contains(&excluded_normalized) {
                return PrivacyLevel::Ignored;
            }
        }
        PrivacyLevel::Full
    }

    /// 检查窗口标题是否触发隐私保护
    pub fn should_anonymize_by_keyword(&self, window_title: &str) -> bool {
        let title_lower = window_title.to_lowercase();
        self.excluded_keywords
            .iter()
            .any(|k| title_lower.contains(&k.to_lowercase()))
    }

    /// 从 URL 中提取域名
    pub fn extract_domain(url: &str) -> String {
        let without_protocol = url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        without_protocol
            .split('/')
            .next()
            .unwrap_or("")
            .to_lowercase()
    }

    /// 后缀匹配域名：domain == pattern 或 domain 以 .pattern 结尾
    pub fn domain_matches(domain: &str, pattern: &str) -> bool {
        if domain == pattern {
            return true;
        }
        domain.ends_with(&format!(".{pattern}"))
    }

    /// 迁移旧版 excluded_apps 到 app_rules
    pub fn migrate_legacy_excluded_apps(&mut self) -> bool {
        if self.excluded_apps.is_empty() {
            return false;
        }
        let existing_names: std::collections::HashSet<String> = self
            .app_rules
            .iter()
            .map(|r| r.app_name.to_lowercase())
            .collect();
        let mut migrated = false;
        for app in self.excluded_apps.drain(..) {
            // 空条目迁移成空规则名会命中一切应用，跳过
            if !app.trim().is_empty() && !existing_names.contains(&app.to_lowercase()) {
                self.app_rules.push(AppPrivacyRule {
                    app_name: app,
                    level: PrivacyLevel::Ignored,
                });
                migrated = true;
            }
        }
        migrated
    }

    /// 收集所有应忽略的应用名（小写），包含 app_rules 和遗留 excluded_apps
    pub fn collect_ignored_app_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .app_rules
            .iter()
            .filter(|rule| rule.level == PrivacyLevel::Ignored)
            .map(|rule| rule.app_name.to_lowercase())
            .collect();
        for app in &self.excluded_apps {
            let lower = app.to_lowercase();
            if !names.contains(&lower) {
                names.push(lower);
            }
        }
        names
    }

    /// 收集所有域名黑名单（已提取域名、已去空）
    pub fn collect_excluded_domains(&self) -> Vec<String> {
        self.excluded_domains
            .iter()
            .map(|d| Self::extract_domain(d))
            .filter(|d| !d.is_empty())
            .collect()
    }
}

/// 存储配置
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ScreenshotDisplayMode {
    /// 仅截活动窗口所在屏幕
    #[default]
    ActiveWindow,
    /// 截取所有屏幕
    All,
}

/// 截图宽度模式
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ScreenshotWidthMode {
    /// 自适应：根据屏幕分辨率动态计算（取屏幕宽度的 70%）
    #[default]
    Auto,
    /// 固定值：使用 max_image_width 设定的值
    Fixed,
}

/// 存储配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageConfig {
    /// 截图保留天数（超过后删除截图文件）
    pub screenshot_retention_days: u32,
    /// 元数据保留天数（超过后删除数据库记录）
    pub metadata_retention_days: u32,
    /// 存储空间上限（MB），超过后自动清理最旧的数据
    pub storage_limit_mb: u32,
    /// JPEG 质量 (1-100)
    pub jpeg_quality: u8,
    /// 最大图片宽度（超过会缩放）
    pub max_image_width: u32,
    /// 是否启用截图与 OCR
    #[serde(default = "default_screenshots_enabled")]
    pub screenshots_enabled: bool,
    /// 截图屏幕范围
    #[serde(default)]
    pub screenshot_display_mode: ScreenshotDisplayMode,
    /// 截图宽度模式：auto 自适应屏幕，fixed 使用固定值
    #[serde(default)]
    pub screenshot_width_mode: ScreenshotWidthMode,
}

fn default_screenshots_enabled() -> bool {
    true
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            screenshot_retention_days: 7, // 默认保留7天截图
            metadata_retention_days: 30,  // 默认保留30天元数据
            storage_limit_mb: 2048,       // 默认2GB上限
            jpeg_quality: 85,             // 85%质量，更清晰
            max_image_width: 1280,        // 最大宽度1280px
            screenshots_enabled: true,
            screenshot_display_mode: ScreenshotDisplayMode::ActiveWindow,
            screenshot_width_mode: ScreenshotWidthMode::Auto,
        }
    }
}

/// 远程存储提供商
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RemoteStorageProvider {
    #[default]
    None,
    S3,
    #[serde(rename = "webdav")]
    WebDav,
}

/// S3 兼容存储（MinIO）配置
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct S3Config {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub access_key: String,
    #[serde(default)]
    pub secret_key: String,
    #[serde(default = "default_s3_region")]
    pub region: String,
    #[serde(default)]
    pub path_prefix: String,
    #[serde(default)]
    pub public_url_base: Option<String>,
}

fn default_s3_region() -> String {
    "us-east-1".to_string()
}

/// WebDAV 存储配置
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct WebDavConfig {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub path_prefix: String,
    #[serde(default)]
    pub public_url_base: Option<String>,
}

/// 远程存储配置
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct RemoteStorageConfig {
    #[serde(default)]
    pub provider: RemoteStorageProvider,
    #[serde(default)]
    pub s3: S3Config,
    #[serde(default)]
    pub webdav: WebDavConfig,
    /// 将常用应用配置同步到 WebDAV（最后写入获胜）。本机端口、窗口位置等不覆盖。
    #[serde(default)]
    pub sync_app_config: bool,
}

/// 本地文件监控配置
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct FileMonitorConfig {
    /// 是否启用文件监控
    pub enabled: bool,
    /// 监听的工作文件夹列表（递归监听）
    pub folders: Vec<String>,
    /// 排除规则：目录段名或 `*` 通配文件名（如 .git、node_modules、*.log）
    pub excludes: Vec<String>,
}

impl Default for FileMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            folders: Vec::new(),
            excludes: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "dist".to_string(),
                "build".to_string(),
                "__pycache__".to_string(),
                "*.tmp".to_string(),
                "~$*".to_string(),
            ],
        }
    }
}

/// 私有化 GitLab 集成配置（日报补充今日提交）
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GitlabConfig {
    /// GitLab 私有化部署地址（如 https://git.example.com）
    #[serde(default)]
    pub url: Option<String>,
    /// 个人访问令牌（明文，与 AI api_key 相同的存储策略；仅存本机配置）
    #[serde(default)]
    pub token: Option<String>,
    /// 项目允许列表（path_with_namespace，一行一个；空 = 全部有权限项目）
    #[serde(default)]
    pub project_filter: Vec<String>,
    /// 纳入日报的作者名单（姓名或邮箱，忽略大小写与提交的 author_email/
    /// author_name 精确比对；与 token 账号取并集；空 = 仅当前账号）
    #[serde(default)]
    pub work_authors: Vec<String>,
    /// 日报生成时是否注入 GitLab 今日提交（快速开关；需已配置 url+token）
    #[serde(default = "default_true")]
    pub report_enabled: bool,
}

pub const DEFAULT_LOCALHOST_API_PORT: u16 = 47_831;

fn default_localhost_api_port() -> u16 {
    DEFAULT_LOCALHOST_API_PORT
}

fn normalize_localhost_api_port(port: u16) -> u16 {
    if port == 0 {
        DEFAULT_LOCALHOST_API_PORT
    } else {
        port
    }
}

/// 配置加载状态，用于区分首次启动、正常加载和故障恢复。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigLoadStatus {
    Loaded,
    Missing,
    RecoveredFromBackup,
    Corrupted,
}

impl ConfigLoadStatus {
    /// 当前加载状态是否允许启动流程自动写回配置。
    pub fn allows_automatic_save(self) -> bool {
        matches!(self, Self::Loaded | Self::Missing)
    }

    /// 当前加载状态是否必须以故障安全模式启动。
    pub fn requires_fail_safe(self) -> bool {
        matches!(self, Self::Corrupted)
    }
}

/// 返回在完整配置文件名后追加 `.bak` 的备份路径。
pub fn config_backup_path(path: &Path) -> PathBuf {
    let mut backup_name = path.as_os_str().to_os_string();
    backup_name.push(".bak");
    PathBuf::from(backup_name)
}

fn config_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn unique_config_temp_path(target: &Path) -> io::Result<PathBuf> {
    let file_name = target
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "配置路径必须包含文件名"))?;
    let mut temp_name = OsString::from(".");
    temp_name.push(file_name);
    temp_name.push(".tmp-");
    temp_name.push(uuid::Uuid::new_v4().simple().to_string());
    Ok(config_parent(target).join(temp_name))
}

fn open_secure_temp_file(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

struct PendingConfigFile {
    path: PathBuf,
    pending: bool,
}

impl PendingConfigFile {
    fn create(target: &Path) -> io::Result<(Self, File)> {
        let path = unique_config_temp_path(target)?;
        let file = open_secure_temp_file(&path)?;
        Ok((
            Self {
                path,
                pending: true,
            },
            file,
        ))
    }

    fn replace(&mut self, target: &Path) -> io::Result<()> {
        atomic_replace(&self.path, target)?;
        self.pending = false;
        Ok(())
    }
}

impl Drop for PendingConfigFile {
    fn drop(&mut self) {
        if self.pending {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn update_config_backup_with_sync<F>(primary_path: &Path, sync_parent: F) -> io::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let mut primary_file = match File::open(primary_path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    let backup_path = config_backup_path(primary_path);
    let (mut pending_backup, mut backup_file) = PendingConfigFile::create(&backup_path)?;
    io::copy(&mut primary_file, &mut backup_file)?;
    backup_file.flush()?;
    backup_file.sync_all()?;
    drop(backup_file);
    commit_pending_config(
        &mut pending_backup,
        &backup_path,
        config_parent(primary_path),
        sync_parent,
    )
}

fn update_config_backup(primary_path: &Path) -> io::Result<()> {
    update_config_backup_with_sync(primary_path, sync_parent_directory)
}

#[cfg(unix)]
fn atomic_replace(source: &Path, target: &Path) -> io::Result<()> {
    std::fs::rename(source, target)
}

#[cfg(windows)]
fn atomic_replace(source: &Path, target: &Path) -> io::Result<()> {
    use std::iter;
    use std::os::windows::ffi::OsStrExt;

    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;

    #[link(name = "Kernel32")]
    extern "system" {
        fn MoveFileExW(
            existing_file_name: *const u16,
            new_file_name: *const u16,
            flags: u32,
        ) -> i32;
    }

    let source_wide = source
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let target_wide = target
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        MoveFileExW(
            source_wide.as_ptr(),
            target_wide.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if replaced == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn atomic_replace(source: &Path, target: &Path) -> io::Result<()> {
    std::fs::rename(source, target)
}

#[cfg(unix)]
fn sync_parent_directory(parent: &Path) -> io::Result<()> {
    File::open(parent)?.sync_all()
}

#[cfg(not(unix))]
fn sync_parent_directory(_parent: &Path) -> io::Result<()> {
    // Windows 原子替换已请求 MOVEFILE_WRITE_THROUGH；其他平台无通用目录句柄同步接口。
    Ok(())
}

fn commit_pending_config<F>(
    pending_config: &mut PendingConfigFile,
    path: &Path,
    parent: &Path,
    sync_parent: F,
) -> io::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    pending_config.replace(path)?;
    if let Err(error) = sync_parent(parent) {
        // 原子替换是提交点；此后只能报告持久化告警，不能伪装成“保存未发生”。
        log::warn!(
            "配置已提交，但同步父目录 {} 失败: {error}",
            parent.display()
        );
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct NodeGatewayConfig {
    #[serde(default)]
    pub device_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDevice {
    pub name: String,
    pub url: String,
    pub token: String,
}

/// 工作时间段
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WorkTimeSegment {
    pub start_hour: u8,
    #[serde(default)]
    pub start_minute: u8,
    pub end_hour: u8,
    #[serde(default)]
    pub end_minute: u8,
}

/// 应用配置
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// 截屏间隔（秒）
    pub screenshot_interval: u64,
    /// 界面语言（zh-CN / en / zh-TW），用于后端托盘菜单等本地化
    #[serde(default = "default_locale")]
    pub locale: String,
    /// 空闲检测阈值（分钟）：超过此时间无键鼠输入则进入疑似空闲
    #[serde(default = "default_idle_threshold_minutes")]
    pub idle_threshold_minutes: u32,
    /// AI分析模式
    pub ai_mode: AiMode,
    /// 文本模型配置
    #[serde(default = "ModelConfig::default_text")]
    pub text_model: ModelConfig,
    /// 助手整次请求绝对超时（秒）
    #[serde(default = "default_assistant_timeout_secs")]
    pub assistant_timeout_secs: u64,
    /// 日报生成外层超时（秒）
    #[serde(default = "default_report_generation_timeout_secs")]
    pub report_generation_timeout_secs: u64,
    /// 可保存的文本模型档案
    #[serde(default)]
    pub text_model_profiles: Vec<TextModelProfile>,
    /// 各服务商的模型配置快照（key 为 provider id；切换服务商时保留各家的地址/模型/Key，随配置一起保存）
    #[serde(default)]
    pub text_model_provider_cache: HashMap<String, ModelConfig>,
    /// 视觉模型配置
    #[serde(default = "ModelConfig::default_vision")]
    pub vision_model: ModelConfig,
    /// 隐私配置
    #[serde(default)]
    pub privacy: PrivacyConfig,
    /// 应用分类覆盖规则
    #[serde(default)]
    pub app_category_rules: Vec<AppCategoryRule>,
    /// 用户自定义分类
    #[serde(default)]
    pub custom_categories: Vec<CustomCategory>,
    /// 网站语义分类覆盖规则
    #[serde(default)]
    pub website_semantic_rules: Vec<WebsiteSemanticRule>,
    /// 用户自定义语义分类
    #[serde(default)]
    pub custom_semantic_categories: Vec<CustomSemanticCategory>,
    /// 已删除的内置应用分类 key（防止 seed 复活）
    #[serde(default)]
    pub deleted_default_categories: Vec<String>,
    /// 已删除的内置语义分类 key（防止 seed 复活）
    #[serde(default)]
    pub deleted_default_semantic_categories: Vec<String>,
    /// 存储配置
    #[serde(default)]
    pub storage: StorageConfig,
    /// 远程存储配置（S3/MinIO 或 WebDAV）
    #[serde(default)]
    pub remote_storage: RemoteStorageConfig,
    /// 最近一次成功参与 WebDAV 配置同步的时间戳（毫秒）。0 表示从未同步。
    #[serde(default)]
    pub config_synced_at: u64,
    /// 日报附加提示词
    #[serde(default)]
    pub daily_report_custom_prompt: String,
    /// 日报提示词预设模板列表
    #[serde(default)]
    pub daily_report_prompt_presets: Vec<PromptPreset>,
    /// 日报系统提示词覆盖（有值时替代硬编码的系统提示词）
    #[serde(default)]
    pub daily_report_system_prompt_override: Option<String>,
    /// 日报 Markdown 导出目录
    #[serde(default)]
    pub daily_report_export_dir: Option<String>,
    /// 日报自动生成后是否自动导出 Markdown
    #[serde(default)]
    pub daily_report_auto_export: bool,
    /// 日报自动生成时间 (HH:MM)，为空时使用工作结束时间
    #[serde(default)]
    pub daily_report_auto_generate_time: Option<String>,
    /// 日报钉选段落（永远排在最前），值为 StatsBlock 的 block_name
    #[serde(default)]
    pub daily_report_pinned_blocks: Vec<String>,
    /// 日报隐藏段落（不显示），值为 StatsBlock 的 block_name
    #[serde(default)]
    pub daily_report_hidden_blocks: Vec<String>,
    /// 上次 AI 编排的段落顺序（缓存，避免每次生成都调 LLM 排序）
    #[serde(default)]
    pub daily_report_last_ai_order: Vec<String>,
    /// 是否启用本地 localhost API
    #[serde(default)]
    pub localhost_api_enabled: bool,
    /// 本地 localhost API 监听地址
    #[serde(default)]
    pub localhost_api_host: Option<String>,
    /// 本地 localhost API 端口
    #[serde(default = "default_localhost_api_port")]
    pub localhost_api_port: u16,
    /// 是否启用飞书 Bot
    #[serde(default)]
    pub feishu_bot_enabled: bool,
    /// 飞书 App ID
    #[serde(default)]
    pub feishu_app_id: Option<String>,
    /// 飞书 App Secret
    #[serde(default)]
    pub feishu_app_secret: Option<String>,
    /// 飞书 Verification Token
    #[serde(default)]
    pub feishu_verification_token: Option<String>,
    /// 飞书 Encrypt Key
    #[serde(default)]
    pub feishu_encrypt_key: Option<String>,
    /// 是否启用企业微信 Bot
    #[serde(default)]
    pub wecom_bot_enabled: bool,
    /// 企业微信 Corp ID
    #[serde(default)]
    pub wecom_corp_id: Option<String>,
    /// 企业微信回调 Token
    #[serde(default)]
    pub wecom_token: Option<String>,
    /// 企业微信 EncodingAESKey
    #[serde(default)]
    pub wecom_encoding_aes_key: Option<String>,
    /// 企业微信智能机器人 Bot ID（长连接）
    #[serde(default)]
    pub wecom_bot_id: Option<String>,
    /// 企业微信智能机器人 Secret（长连接）
    #[serde(default)]
    pub wecom_bot_secret: Option<String>,
    /// 是否启用钉钉 Bot
    #[serde(default)]
    pub dingtalk_bot_enabled: bool,
    /// 钉钉应用 App Secret
    #[serde(default)]
    pub dingtalk_app_secret: Option<String>,
    /// 是否启用 MCP Server
    #[serde(default)]
    pub mcp_server_enabled: bool,
    /// 已注册的远程设备列表
    #[serde(default)]
    pub node_devices: Vec<NodeDevice>,
    /// 设备节点 / 控制面配置
    #[serde(default)]
    pub node_gateway: NodeGatewayConfig,
    /// 是否开机自启
    pub auto_start: bool,
    /// 开机自启动时是否静默驻留而不显示主界面
    #[serde(default)]
    pub auto_start_silent: bool,
    /// macOS 录屏权限是否已经主动弹过引导，避免每次启动都重复请求
    #[serde(default)]
    pub macos_screen_capture_permission_prompted: bool,
    /// 上次运行的应用版本，用于检测版本更新后重置权限引导
    #[serde(default)]
    pub last_app_version: Option<String>,
    /// 主题模式: system, light, dark
    pub theme: String,
    /// 界面风格: a=Quiet Pro, b=当前柔和层次, c=Compact Data
    #[serde(default = "default_ui_visual_style")]
    pub ui_visual_style: String,
    /// 上班开始时间（0-23）
    #[serde(default = "default_work_start")]
    pub work_start_hour: u8,
    /// 上班结束时间（0-23）
    #[serde(default = "default_work_end")]
    pub work_end_hour: u8,
    /// 上班开始分钟（0-59）
    #[serde(default)]
    pub work_start_minute: u8,
    /// 上班结束分钟（0-59）
    #[serde(default)]
    pub work_end_minute: u8,
    /// 分段工作时间（优先级高于上面的单段时间）
    /// 为空时使用 work_start_hour/work_end_hour 作为一段
    /// 例：[{start_hour:9,start_minute:0,end_hour:12,end_minute:0},{start_hour:13,start_minute:0,end_hour:18,end_minute:0}]
    #[serde(default)]
    pub work_time_segments: Vec<WorkTimeSegment>,
    /// 是否启用工作时间过滤。关闭后所有活动都算"工作时间"，不再区分
    #[serde(default = "default_true")]
    pub work_time_enabled: bool,
    /// 每日标准工时（小时），用于计算加班时长。默认 8.0
    #[serde(default = "default_standard_work_hours")]
    pub standard_work_hours: f64,

    // 兼容旧版配置
    #[serde(default)]
    pub ai_provider: AiProviderConfig,
    #[serde(default)]
    pub ollama_host: String,
    #[serde(default)]
    pub ollama_model: String,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default)]
    pub openai_model: String,
    /// 隐藏 Dock 图标（仅保留菜单栏）
    #[serde(default)]
    pub hide_dock_icon: bool,
    /// 轻量模式：关闭主界面时释放主 Webview，仅保留后台录制与托盘
    #[serde(default)]
    pub lightweight_mode: bool,
    /// 每日工作时长目标（分钟），None = 不设目标
    #[serde(default)]
    pub daily_work_goal_minutes: Option<u32>,
    /// 达到目标时桌宠庆祝提醒
    #[serde(default)]
    pub goal_notifications: bool,
    /// AI 工作记忆：自动合成洞察
    #[serde(default)]
    pub memory_enabled: bool,
    /// 上次记忆合成日期（防同天重复）
    #[serde(default)]
    pub memory_last_synthesis_date: Option<String>,
    /// 助手显式长期记忆开关。默认关闭，关闭不会删除已保存的记忆。
    #[serde(default)]
    pub assistant_memory_enabled: bool,
    /// 助手联网能力总开关（隐私默认关：开启后问题可能发给搜索服务商/目标网站）
    #[serde(default)]
    pub assistant_web_access_enabled: bool,
    /// 搜索服务商："tavily" / "bocha"
    #[serde(default = "default_assistant_search_provider")]
    pub assistant_search_provider: String,
    /// 搜索服务 API Key（为空时 web_search 工具不注册，网页读取/天气不受影响）
    #[serde(default)]
    pub assistant_search_api_key: Option<String>,
    /// 语义记忆总开关（隐私默认关：开启后 OCR/标题会送往嵌入模型；
    /// 本地 Ollama 时数据不出机，云端 API 时明示出网）
    #[serde(default)]
    pub memory_semantic_enabled: bool,
    /// 嵌入服务商："ollama"（本地默认）/ "openai"（OpenAI 兼容 /v1/embeddings）
    #[serde(default = "default_embedding_provider")]
    pub embedding_provider: String,
    /// 嵌入服务地址（Ollama 默认 http://localhost:11434）
    #[serde(default = "default_embedding_endpoint")]
    pub embedding_endpoint: String,
    /// 嵌入模型名（Ollama 默认 nomic-embed-text；OpenAI 兼容常用 BAAI/bge-m3 等）
    #[serde(default = "default_embedding_model")]
    pub embedding_model: String,
    /// 嵌入服务 API Key（Ollama 不需要）
    #[serde(default)]
    pub embedding_api_key: Option<String>,
    /// 隐藏系统标题栏装饰
    #[serde(default)]
    pub hide_decorations: bool,
    /// 背景图片文件名（存储在 data 目录下）
    #[serde(default)]
    pub background_image: Option<String>,
    /// 背景图片不透明度 (0.01 - 0.60)
    #[serde(default = "default_bg_opacity")]
    pub background_opacity: f32,
    /// 背景图片模糊程度 (0 = 清晰, 1 = 轻微, 2 = 中等)
    #[serde(default = "default_bg_blur")]
    pub background_blur: u8,
    /// GitLab 私有化集成配置（日报补充今日提交）
    #[serde(default)]
    pub gitlab: GitlabConfig,
    /// 本地文件监控配置（工作文件夹监听，概览页展示）
    #[serde(default)]
    pub file_monitor: FileMonitorConfig,
}

/// 配置加载结果，保留恢复状态和主备文件错误信息。
#[derive(Debug)]
pub struct ConfigLoadResult {
    pub config: AppConfig,
    pub status: ConfigLoadStatus,
    pub primary_error: Option<String>,
    pub backup_error: Option<String>,
}

#[allow(clippy::large_enum_variant)]
enum ConfigFileLoad {
    Loaded(AppConfig),
    Missing,
    Invalid(String),
}

fn default_work_start() -> u8 {
    9
}
fn default_work_end() -> u8 {
    18
}
fn default_bg_opacity() -> f32 {
    0.25
}
fn default_bg_blur() -> u8 {
    1
}
fn default_assistant_search_provider() -> String {
    "tavily".to_string()
}
fn default_embedding_provider() -> String {
    "ollama".to_string()
}
fn default_embedding_endpoint() -> String {
    "http://localhost:11434".to_string()
}
fn default_embedding_model() -> String {
    "nomic-embed-text".to_string()
}
fn default_standard_work_hours() -> f64 {
    8.0
}
fn default_ui_visual_style() -> String {
    "c".to_string()
}
pub fn default_assistant_timeout_secs() -> u64 {
    120
}
pub fn default_report_generation_timeout_secs() -> u64 {
    300
}
fn normalize_assistant_timeout_secs(value: u64) -> u64 {
    value.clamp(30, 900)
}
fn normalize_report_generation_timeout_secs(value: u64) -> u64 {
    value.clamp(60, 1800)
}
fn normalize_positive_token(value: Option<u32>) -> Option<u32> {
    value.filter(|n| *n > 0)
}

/// 纳入作者名单规范化：去首尾空白、剔空条目、按忽略大小写去重（保留首次写法）。
fn normalize_work_author_list(items: &[String]) -> Vec<String> {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut result = Vec::new();
    for item in items {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.to_lowercase()) {
            result.push(trimmed.to_string());
        }
    }
    result
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            screenshot_interval: 30,
            locale: default_locale(),
            idle_threshold_minutes: default_idle_threshold_minutes(),
            ai_mode: AiMode::Local,
            text_model: ModelConfig::default_text(),
            assistant_timeout_secs: default_assistant_timeout_secs(),
            report_generation_timeout_secs: default_report_generation_timeout_secs(),
            text_model_profiles: Vec::new(),
            text_model_provider_cache: HashMap::new(),
            vision_model: ModelConfig::default_vision(),
            privacy: PrivacyConfig::default(),
            app_category_rules: Vec::new(),
            custom_categories: Vec::new(),
            website_semantic_rules: Vec::new(),
            custom_semantic_categories: Vec::new(),
            deleted_default_categories: Vec::new(),
            deleted_default_semantic_categories: Vec::new(),
            storage: StorageConfig::default(),
            remote_storage: RemoteStorageConfig::default(),
            config_synced_at: 0,
            gitlab: GitlabConfig::default(),
            file_monitor: FileMonitorConfig::default(),
            daily_report_custom_prompt: String::new(),
            daily_report_prompt_presets: Vec::new(),
            daily_report_system_prompt_override: None,
            daily_report_export_dir: None,
            daily_report_auto_export: false,
            daily_report_pinned_blocks: Vec::new(),
            daily_report_hidden_blocks: Vec::new(),
            daily_report_last_ai_order: Vec::new(),
            daily_report_auto_generate_time: None,
            localhost_api_enabled: false,
            localhost_api_host: None,
            localhost_api_port: DEFAULT_LOCALHOST_API_PORT,
            feishu_bot_enabled: false,
            feishu_app_id: None,
            feishu_app_secret: None,
            feishu_verification_token: None,
            feishu_encrypt_key: None,
            wecom_bot_enabled: false,
            wecom_corp_id: None,
            wecom_token: None,
            wecom_encoding_aes_key: None,
            wecom_bot_id: None,
            wecom_bot_secret: None,
            dingtalk_bot_enabled: false,
            dingtalk_app_secret: None,
            mcp_server_enabled: false,
            node_devices: Vec::new(),
            node_gateway: NodeGatewayConfig::default(),
            auto_start: false,
            auto_start_silent: false,
            macos_screen_capture_permission_prompted: false,
            last_app_version: None,
            theme: "system".to_string(),
            ui_visual_style: default_ui_visual_style(),
            work_start_hour: 9,
            work_end_hour: 18,
            work_start_minute: 0,
            work_end_minute: 0,
            work_time_segments: vec![
                WorkTimeSegment {
                    start_hour: 9,
                    start_minute: 0,
                    end_hour: 12,
                    end_minute: 0,
                },
                WorkTimeSegment {
                    start_hour: 13,
                    start_minute: 0,
                    end_hour: 18,
                    end_minute: 0,
                },
            ],
            work_time_enabled: true,
            standard_work_hours: default_standard_work_hours(),
            // 旧版兼容字段
            ai_provider: AiProviderConfig::default(),
            ollama_host: "http://localhost:11434".to_string(),
            ollama_model: "llava".to_string(),
            openai_api_key: None,
            openai_model: "gpt-5.4".to_string(),
            hide_dock_icon: false,
            lightweight_mode: false,
            daily_work_goal_minutes: None,
            goal_notifications: false,
            memory_enabled: false,
            memory_last_synthesis_date: None,
            assistant_memory_enabled: false,
            assistant_web_access_enabled: false,
            assistant_search_provider: default_assistant_search_provider(),
            assistant_search_api_key: None,
            memory_semantic_enabled: false,
            embedding_provider: default_embedding_provider(),
            embedding_endpoint: default_embedding_endpoint(),
            embedding_model: default_embedding_model(),
            embedding_api_key: None,
            hide_decorations: false,
            background_image: None,
            background_opacity: 0.25,
            background_blur: 1,
        }
    }
}

impl AppConfig {
    /// 应用远端同步快照，保留本机专属字段（端口、窗口位置、自启、导出目录等）。
    pub fn apply_synced_remote(&mut self, remote: AppConfig) {
        let localhost_api_enabled = self.localhost_api_enabled;
        let localhost_api_host = self.localhost_api_host.clone();
        let localhost_api_port = self.localhost_api_port;
        let auto_start = self.auto_start;
        let auto_start_silent = self.auto_start_silent;
        let macos_screen_capture_permission_prompted =
            self.macos_screen_capture_permission_prompted;
        let last_app_version = self.last_app_version.clone();
        let daily_report_export_dir = self.daily_report_export_dir.clone();
        let node_gateway = self.node_gateway.clone();
        let background_image = self.background_image.clone();

        *self = remote;

        self.localhost_api_enabled = localhost_api_enabled;
        self.localhost_api_host = localhost_api_host;
        self.localhost_api_port = localhost_api_port;
        self.auto_start = auto_start;
        self.auto_start_silent = auto_start_silent;
        self.macos_screen_capture_permission_prompted = macos_screen_capture_permission_prompted;
        self.last_app_version = last_app_version;
        self.daily_report_export_dir = daily_report_export_dir;
        self.node_gateway = node_gateway;
        self.background_image = background_image;
    }

    /// 规范化配置，兼容旧字段并补齐助手可用的文本模型档案
    pub fn normalize(&mut self) {
        self.migrate_legacy_config();
        // 空白规则名/空关键词会以子串 contains("") 恒真命中一切：
        // 空规则名静默停掉全部记录、空关键词让所有窗口永久触发脱敏，
        // 在规范化阶段直接剔除
        self.privacy
            .app_rules
            .retain(|rule| !rule.app_name.trim().is_empty());
        self.privacy
            .excluded_keywords
            .retain(|k| !k.trim().is_empty());
        self.gitlab.work_authors = normalize_work_author_list(&self.gitlab.work_authors);
        normalize_custom_categories(&mut self.custom_categories);
        normalize_app_category_rules(&mut self.app_category_rules, &self.custom_categories);
        // Seed defaults BEFORE rule validation so built-in categories are present
        seed_default_categories(
            &mut self.custom_categories,
            &self.deleted_default_categories,
        );
        seed_default_semantic_categories(
            &mut self.custom_semantic_categories,
            &self.deleted_default_semantic_categories,
        );
        normalize_custom_semantic_categories(&mut self.custom_semantic_categories);
        normalize_website_semantic_rules(
            &mut self.website_semantic_rules,
            &self.custom_semantic_categories,
        );
        self.screenshot_interval = normalize_screenshot_interval(self.screenshot_interval);
        self.idle_threshold_minutes = normalize_idle_threshold_minutes(self.idle_threshold_minutes);
        // storage_limit_mb=0 会让周期清理把上限算成 0 字节，"删到低于上限"的循环
        // 永不满足而逐个删光全部截图目录；与设置页下限(256)对齐做后端钳制，
        // 挡住配置同步/手改配置文件直通入库的 0 值
        self.storage.storage_limit_mb = self.storage.storage_limit_mb.max(256);
        // 保留天数无上限时，清理路径 `date_naive() - Duration::days(n)` 会越过
        // NaiveDate::MIN 直接 panic（配置同步/手改配置可写入 u32 任意值）；
        // 0 = 永久保留语义保留，非 0 钳到 10 年（与 parse_temporal_range 同族的兜底钳制）
        self.storage.screenshot_retention_days = self.storage.screenshot_retention_days.min(3650);
        self.ui_visual_style = normalize_ui_visual_style(&self.ui_visual_style);
        self.standard_work_hours = self.standard_work_hours.clamp(1.0, 24.0);
        self.assistant_timeout_secs = normalize_assistant_timeout_secs(self.assistant_timeout_secs);
        self.report_generation_timeout_secs =
            normalize_report_generation_timeout_secs(self.report_generation_timeout_secs);
        self.text_model.thinking_budget = normalize_positive_token(self.text_model.thinking_budget);
        self.text_model.max_output_tokens =
            normalize_positive_token(self.text_model.max_output_tokens);
        self.daily_report_custom_prompt = self.daily_report_custom_prompt.trim().to_string();
        normalize_prompt_presets(&mut self.daily_report_prompt_presets);
        self.daily_report_export_dir =
            normalize_optional_string(self.daily_report_export_dir.take());
        self.localhost_api_port = normalize_localhost_api_port(self.localhost_api_port);
        self.localhost_api_host = normalize_optional_string(self.localhost_api_host.take());
        self.wecom_bot_id = normalize_optional_string(self.wecom_bot_id.take());
        self.wecom_bot_secret = normalize_optional_string(self.wecom_bot_secret.take());
        self.remote_storage.s3.public_url_base =
            normalize_optional_string(self.remote_storage.s3.public_url_base.take());
        self.remote_storage.webdav.public_url_base =
            normalize_optional_string(self.remote_storage.webdav.public_url_base.take());
        self.node_gateway.device_name =
            normalize_optional_string(self.node_gateway.device_name.take());
        self.sync_text_model_profiles();
    }

    /// 从文件加载配置
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            Self::load_existing(path)
        } else {
            Ok(Self::default())
        }
    }

    /// 加载配置并返回可供启动流程判定的状态。
    pub fn load_with_recovery(path: &Path) -> ConfigLoadResult {
        let backup_path = config_backup_path(path);
        match Self::load_file(path) {
            ConfigFileLoad::Loaded(config) => {
                let backup_error = match Self::load_file(&backup_path) {
                    ConfigFileLoad::Invalid(error) => Some(error),
                    ConfigFileLoad::Loaded(_) | ConfigFileLoad::Missing => None,
                };
                ConfigLoadResult {
                    config,
                    status: ConfigLoadStatus::Loaded,
                    primary_error: None,
                    backup_error,
                }
            }
            ConfigFileLoad::Missing => match Self::load_file(&backup_path) {
                ConfigFileLoad::Loaded(config) => ConfigLoadResult {
                    config,
                    status: ConfigLoadStatus::RecoveredFromBackup,
                    primary_error: None,
                    backup_error: None,
                },
                ConfigFileLoad::Missing => ConfigLoadResult {
                    config: Self::default(),
                    status: ConfigLoadStatus::Missing,
                    primary_error: None,
                    backup_error: None,
                },
                ConfigFileLoad::Invalid(backup_error) => ConfigLoadResult {
                    config: Self::fail_safe(),
                    status: ConfigLoadStatus::Corrupted,
                    primary_error: None,
                    backup_error: Some(backup_error),
                },
            },
            ConfigFileLoad::Invalid(primary_error) => match Self::load_file(&backup_path) {
                ConfigFileLoad::Loaded(config) => ConfigLoadResult {
                    config,
                    status: ConfigLoadStatus::RecoveredFromBackup,
                    primary_error: Some(primary_error),
                    backup_error: None,
                },
                ConfigFileLoad::Missing => ConfigLoadResult {
                    config: Self::fail_safe(),
                    status: ConfigLoadStatus::Corrupted,
                    primary_error: Some(primary_error),
                    backup_error: None,
                },
                ConfigFileLoad::Invalid(backup_error) => ConfigLoadResult {
                    config: Self::fail_safe(),
                    status: ConfigLoadStatus::Corrupted,
                    primary_error: Some(primary_error),
                    backup_error: Some(backup_error),
                },
            },
        }
    }

    fn load_file(path: &Path) -> ConfigFileLoad {
        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return ConfigFileLoad::Missing;
            }
            Err(error) => {
                return ConfigFileLoad::Invalid(format!(
                    "读取配置文件 {} 失败: {error}",
                    path.display()
                ));
            }
        };

        match serde_json::from_str::<AppConfig>(&content) {
            Ok(mut config) => {
                config.normalize();
                ConfigFileLoad::Loaded(config)
            }
            Err(error) => {
                ConfigFileLoad::Invalid(format!("解析配置文件 {} 失败: {error}", path.display()))
            }
        }
    }

    fn load_existing(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let mut config: AppConfig = serde_json::from_str(&content)?;
        config.normalize();
        Ok(config)
    }

    fn fail_safe() -> Self {
        let mut config = Self::default();
        config.storage.screenshots_enabled = false;
        config.localhost_api_enabled = false;
        config.feishu_bot_enabled = false;
        config.wecom_bot_enabled = false;
        config.dingtalk_bot_enabled = false;
        config.mcp_server_enabled = false;
        config.remote_storage.provider = RemoteStorageProvider::None;
        config
    }

    /// 保存配置到文件
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_vec_pretty(self)?;
        let parent = config_parent(path);
        std::fs::create_dir_all(parent)?;

        let (mut pending_config, mut config_file) = PendingConfigFile::create(path)?;
        config_file.write_all(&content)?;
        config_file.flush()?;
        config_file.sync_all()?;
        drop(config_file);

        update_config_backup(path)?;

        commit_pending_config(&mut pending_config, path, parent, sync_parent_directory)?;
        Ok(())
    }

    /// 迁移旧版配置到新结构
    fn migrate_legacy_config(&mut self) {
        // 迁移旧版单段工作时间到 work_time_segments
        if self.work_time_segments.is_empty() {
            self.work_time_segments = vec![WorkTimeSegment {
                start_hour: self.work_start_hour,
                start_minute: self.work_start_minute,
                end_hour: self.work_end_hour,
                end_minute: self.work_end_minute,
            }];
        }
        // 如果新配置为空但旧配置有值，则迁移。
        // 例外：旧字段本身是应用的幽灵默认值（Ollama 默认端点 + 无 Key + 默认模型名
        // qwen3）时不迁移——它不是用户的选择，迁入会把「未配置」永久污染成
        // 「已配置 qwen3」，导致内置兜底模型永远无法生效。
        let legacy_ai_is_ghost_default = self.ai_provider.provider == AiProvider::Ollama
            && self.ai_provider.endpoint.trim() == AiProvider::Ollama.default_endpoint()
            && self
                .ai_provider
                .api_key
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
            && self.ai_provider.model.trim() == AiProvider::Ollama.default_model();
        if self.text_model.model.is_empty()
            && !self.ai_provider.model.is_empty()
            && !legacy_ai_is_ghost_default
        {
            self.text_model = ModelConfig {
                provider: self.ai_provider.provider,
                endpoint: self.ai_provider.endpoint.clone(),
                api_key: self.ai_provider.api_key.clone(),
                model: self.ai_provider.model.clone(),
                enable_thinking: None,
                thinking_budget: None,
                max_output_tokens: None,
            };
        }

        if self.vision_model.model.is_empty() {
            if let Some(ref vision_model) = self.ai_provider.vision_model {
                self.vision_model = ModelConfig {
                    provider: self.ai_provider.provider,
                    endpoint: self.ai_provider.endpoint.clone(),
                    api_key: self.ai_provider.api_key.clone(),
                    model: vision_model.clone(),
                    enable_thinking: None,
                    thinking_budget: None,
                    max_output_tokens: None,
                };
            }
        }

        // 如果旧版 ollama 配置存在
        if !self.ollama_host.is_empty() && self.text_model.endpoint.is_empty() {
            self.text_model.endpoint = self.ollama_host.clone();
            self.vision_model.endpoint = self.ollama_host.clone();
        }

        // 迁移旧版背景不透明度（0.05 太低，用户看不到背景）
        if self.background_opacity <= 0.05 && self.background_image.is_some() {
            self.background_opacity = 0.25;
        }

        self.sync_text_model_profiles();
    }

    /// 获取文本模型端点
    pub fn get_text_endpoint(&self) -> &str {
        &self.text_model.endpoint
    }

    /// 获取视觉模型端点
    pub fn get_vision_endpoint(&self) -> &str {
        &self.vision_model.endpoint
    }

    /// 获取有效的工作时间段列表
    /// - work_time_enabled=false: 返回全天段 (0:00-23:59)，所有活动都算工作时间
    /// - work_time_enabled=true + segments 非空: 使用自定义时间段
    /// - work_time_enabled=true + segments 为空: 从旧字段构造一段
    pub fn effective_work_segments(&self) -> Vec<WorkTimeSegment> {
        if !self.work_time_enabled {
            return vec![WorkTimeSegment {
                start_hour: 0,
                start_minute: 0,
                end_hour: 23,
                end_minute: 59,
            }];
        }
        if !self.work_time_segments.is_empty() {
            return self.work_time_segments.clone();
        }
        vec![WorkTimeSegment {
            start_hour: self.work_start_hour,
            start_minute: self.work_start_minute,
            end_hour: self.work_end_hour,
            end_minute: self.work_end_minute,
        }]
    }

    fn sync_text_model_profiles(&mut self) {
        if !is_model_configured(&self.text_model) {
            return;
        }

        if let Some(profile) = self
            .text_model_profiles
            .iter_mut()
            .find(|profile| profile.id == "default-text-model")
        {
            profile.name = default_profile_name(&self.text_model);
            profile.model_config = self.text_model.clone();
            if profile.test_status.trim().is_empty() {
                profile.test_status = default_connection_status();
            }
            return;
        }

        if self
            .text_model_profiles
            .iter()
            .any(|profile| same_model_config(&profile.model_config, &self.text_model))
        {
            return;
        }

        self.text_model_profiles.insert(
            0,
            TextModelProfile {
                id: "default-text-model".to_string(),
                name: default_profile_name(&self.text_model),
                model_config: self.text_model.clone(),
                test_status: default_connection_status(),
                last_tested_at: None,
                last_test_message: None,
            },
        );
    }
}

fn is_model_configured(model_config: &ModelConfig) -> bool {
    !model_config.endpoint.trim().is_empty() && !model_config.model.trim().is_empty()
}

fn same_model_config(left: &ModelConfig, right: &ModelConfig) -> bool {
    left.provider == right.provider
        && left.endpoint.trim() == right.endpoint.trim()
        && left.model.trim() == right.model.trim()
        && left.api_key.as_deref().unwrap_or("").trim()
            == right.api_key.as_deref().unwrap_or("").trim()
        && left.enable_thinking == right.enable_thinking
        && left.thinking_budget == right.thinking_budget
        && left.max_output_tokens == right.max_output_tokens
}

fn default_profile_name(model_config: &ModelConfig) -> String {
    // 仅新建 profile 时的一次性默认命名；运行时显示走前端。
    #[allow(deprecated)]
    let provider_name = model_config.provider.display_name();
    format!("{} · {}", provider_name, model_config.model.trim())
}

fn default_connection_status() -> String {
    "untested".to_string()
}

pub fn normalize_category_key_private(value: &str, custom_keys: &[String]) -> String {
    let trimmed = value.trim().to_lowercase();
    match trimmed.as_str() {
        "development" | "browser" | "communication" | "office" | "design" | "entertainment"
        | "other" => trimmed,
        _ if custom_keys.iter().any(|k| k == &trimmed) => trimmed,
        _ => "other".to_string(),
    }
}

fn normalize_custom_categories(categories: &mut Vec<CustomCategory>) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    categories.retain(|c| {
        let key = c.key.trim().to_lowercase();
        // key 只允许小写字母、数字、连字符
        let valid_key = !key.is_empty()
            && key
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-');
        // name 非空，color 以 # 开头且 7 字符
        let valid_color = c.color.starts_with('#') && c.color.len() == 7;
        if !valid_key || c.name.trim().is_empty() || !valid_color {
            return false;
        }
        seen.insert(key)
    });
}

fn normalize_app_category_rules(
    rules: &mut Vec<AppCategoryRule>,
    custom_categories: &[CustomCategory],
) {
    use std::collections::HashSet;

    let mut normalized_rules = Vec::new();
    let mut seen = HashSet::new();

    for rule in rules.iter().rev() {
        let app_name = rule.app_name.trim();
        if app_name.is_empty() {
            continue;
        }

        let normalized_app_name = crate::categorize::normalize_display_app_name(app_name);
        let dedupe_key = normalized_app_name.to_lowercase();
        if !seen.insert(dedupe_key) {
            continue;
        }

        let custom_keys: Vec<String> = custom_categories.iter().map(|c| c.key.clone()).collect();
        normalized_rules.push(AppCategoryRule {
            app_name: normalized_app_name,
            category: normalize_category_key_private(&rule.category, &custom_keys),
        });
    }

    normalized_rules.reverse();
    *rules = normalized_rules;
}

fn normalize_website_semantic_rules(
    rules: &mut Vec<WebsiteSemanticRule>,
    custom_semantic_categories: &[CustomSemanticCategory],
) {
    use std::collections::HashSet;

    let custom_keys: Vec<String> = custom_semantic_categories
        .iter()
        .map(|c| c.key.clone())
        .collect();
    let mut normalized_rules = Vec::new();
    let mut seen = HashSet::new();

    for rule in rules.iter().rev() {
        let Some(domain) = crate::categorize::normalize_domain_rule(&rule.domain) else {
            continue;
        };

        let semantic_category = rule.semantic_category.trim();
        if semantic_category.is_empty() {
            continue;
        }

        // 验证语义分类：必须在自定义分类列表中
        let is_custom = custom_keys.iter().any(|k| k == semantic_category);
        if !is_custom {
            continue;
        }

        if !seen.insert(domain.clone()) {
            continue;
        }

        normalized_rules.push(WebsiteSemanticRule {
            domain,
            semantic_category: semantic_category.to_string(),
        });
    }

    normalized_rules.reverse();
    *rules = normalized_rules;
}

pub fn is_valid_builtin_semantic_category(category: &str) -> bool {
    matches!(
        category,
        "编码开发"
            | "内容撰写"
            | "资料阅读"
            | "资料调研"
            | "任务规划"
            | "设计创作"
            | "AI 协作"
            | "即时聊天"
            | "会议沟通"
            | "视频内容"
            | "音乐音频"
            | "休息娱乐"
            | "未知活动"
    )
}

fn normalize_custom_semantic_categories(categories: &mut Vec<CustomSemanticCategory>) {
    use std::collections::HashSet;

    let mut seen = HashSet::new();
    categories.retain(|c| {
        let key = c.key.trim();
        let name = c.name.trim();
        if key.is_empty() || name.is_empty() {
            return false;
        }
        seen.insert(key.to_string())
    });
}

/// 内置应用分类 key 列表（统一维护点）
pub const DEFAULT_CATEGORY_KEYS: &[&str] = &[
    "development",
    "browser",
    "communication",
    "office",
    "design",
    "entertainment",
    "other",
];

/// 内置语义分类 key 列表（统一维护点）
pub const DEFAULT_SEMANTIC_CATEGORY_KEYS: &[&str] = &[
    "编码开发",
    "内容撰写",
    "资料阅读",
    "资料调研",
    "任务规划",
    "设计创作",
    "AI 协作",
    "即时聊天",
    "会议沟通",
    "视频内容",
    "音乐音频",
    "休息娱乐",
    "未知活动",
];

fn seed_default_categories(categories: &mut Vec<CustomCategory>, deleted: &[String]) {
    let defaults: Vec<(&str, &str, &str, &str)> = vec![
        ("development", "开发工具", "#3B82F6", "⚡"),
        ("browser", "浏览器", "#22C55E", "🌐"),
        ("communication", "通讯协作", "#EAB308", "💬"),
        ("office", "办公软件", "#8B5CF6", "📝"),
        ("design", "设计工具", "#EC4899", "🎨"),
        ("entertainment", "娱乐摸鱼", "#EF4444", "🎮"),
        ("other", "其他", "#6B7280", "📁"),
    ];
    let existing_keys: Vec<String> = categories.iter().map(|c| c.key.clone()).collect();
    for (key, name, color, icon) in defaults {
        if deleted.iter().any(|d| d == key) {
            continue;
        }
        if !existing_keys.iter().any(|k| k == key) {
            categories.push(CustomCategory {
                key: key.to_string(),
                name: name.to_string(),
                color: color.to_string(),
                icon: icon.to_string(),
            });
        }
    }
}

fn seed_default_semantic_categories(
    categories: &mut Vec<CustomSemanticCategory>,
    deleted: &[String],
) {
    let defaults: Vec<(&str, &str)> = vec![
        ("编码开发", "编码开发"),
        ("内容撰写", "内容撰写"),
        ("资料阅读", "资料阅读"),
        ("资料调研", "资料调研"),
        ("任务规划", "任务规划"),
        ("设计创作", "设计创作"),
        ("AI 协作", "AI 协作"),
        ("即时聊天", "即时聊天"),
        ("会议沟通", "会议沟通"),
        ("视频内容", "视频内容"),
        ("音乐音频", "音乐音频"),
        ("休息娱乐", "休息娱乐"),
        ("未知活动", "未知活动"),
    ];
    let existing_keys: Vec<String> = categories.iter().map(|c| c.key.clone()).collect();
    for (key, name) in defaults {
        if deleted.iter().any(|d| d == key) {
            continue;
        }
        if !existing_keys.iter().any(|k| k == key) {
            categories.push(CustomSemanticCategory {
                key: key.to_string(),
                name: name.to_string(),
            });
        }
    }
}

fn normalize_prompt_presets(presets: &mut Vec<PromptPreset>) {
    for p in presets.iter_mut() {
        p.name = p.name.trim().to_string();
        p.prompt = p.prompt.trim().to_string();
    }
    presets.retain(|p| !p.name.is_empty() && !p.prompt.is_empty());
}

fn normalize_ui_visual_style(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "a" | "b" | "c" => value.trim().to_ascii_lowercase(),
        _ => default_ui_visual_style(),
    }
}

/// 截屏间隔最低 5 秒，防止配置值过小导致 CPU/磁盘占用过高
fn normalize_screenshot_interval(value: u64) -> u64 {
    value.clamp(5, 600)
}

fn default_idle_threshold_minutes() -> u32 {
    5
}

fn normalize_idle_threshold_minutes(value: u32) -> u32 {
    value.clamp(1, 60)
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::field_reassign_with_default)]

    use super::{
        commit_pending_config, config_backup_path, default_ui_visual_style,
        normalize_app_category_rules, normalize_ui_visual_style, normalize_work_author_list,
        update_config_backup_with_sync, AiProvider, AppCategoryRule, AppConfig, ConfigLoadStatus,
        PendingConfigFile, RemoteStorageProvider, ScreenshotDisplayMode, WebsiteSemanticRule,
        DEFAULT_LOCALHOST_API_PORT,
    };
    use std::io::{self, Write};
    use std::path::PathBuf;

    #[test]
    fn 纳入作者名单规范化应去空白去空并按大小写去重() {
        let input = vec![
            "  李四 ".to_string(),
            "LISI@corp.com".to_string(),
            "lisi@CORP.com".to_string(),
            String::new(),
            "   ".to_string(),
            "王五".to_string(),
        ];
        assert_eq!(
            normalize_work_author_list(&input),
            vec![
                "李四".to_string(),
                "LISI@corp.com".to_string(),
                "王五".to_string()
            ],
            "去首尾空白、剔空条目、忽略大小写去重并保留首次写法"
        );
        assert!(normalize_work_author_list(&[]).is_empty());
    }

    #[test]
    fn 配置规范化应处理纳入作者名单() {
        let mut config = AppConfig::default();
        config.gitlab.work_authors = vec![
            " a@x.com ".to_string(),
            "A@X.COM".to_string(),
            String::new(),
        ];
        config.normalize();
        assert_eq!(config.gitlab.work_authors, vec!["a@x.com".to_string()]);
    }

    #[test]
    fn 配置规范化应把存储上限钳到安全下限() {
        // 0 会让周期清理把上限算成 0 字节而删光全部截图目录（配置同步/手改
        // 配置可绕过 UI 下限直通入库），normalize 必须兜底钳到 256
        let mut config = AppConfig::default();
        config.storage.storage_limit_mb = 0;
        config.normalize();
        assert_eq!(config.storage.storage_limit_mb, 256);

        config.storage.storage_limit_mb = 1;
        config.normalize();
        assert_eq!(config.storage.storage_limit_mb, 256);

        // 合法值不动
        config.storage.storage_limit_mb = 2048;
        config.normalize();
        assert_eq!(config.storage.storage_limit_mb, 2048);
    }

    #[test]
    fn 配置规范化应把截图保留天数钳到安全上限() {
        // 超大值会让清理路径 `date_naive() - Duration::days(n)` 越过 NaiveDate::MIN
        // panic（配置同步/手改配置可写入 u32 任意值），normalize 必须兜底钳制；
        // 0 = 永久保留，必须保留原语义
        let mut config = AppConfig::default();
        config.storage.screenshot_retention_days = 0;
        config.normalize();
        assert_eq!(config.storage.screenshot_retention_days, 0);

        config.storage.screenshot_retention_days = 99999999;
        config.normalize();
        assert_eq!(config.storage.screenshot_retention_days, 3650);

        // 合法值不动
        config.storage.screenshot_retention_days = 7;
        config.normalize();
        assert_eq!(config.storage.screenshot_retention_days, 7);
    }

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "work-review-config-{name}-{}",
            uuid::Uuid::new_v4()
        ))
    }

    #[test]
    fn 出厂默认判定应识别未修改的文本模型配置() {
        assert!(super::ModelConfig::default_text().is_factory_default_text());

        // 旧版迁移产生的幽灵默认（模型名恰为 Ollama 默认模型）同样视为未配置
        assert!(super::ModelConfig {
            model: AiProvider::Ollama.default_model().to_string(),
            ..super::ModelConfig::default_text()
        }
        .is_factory_default_text());

        // 用户改了任意一项（模型名 / 端点 / Key / 提供商）都视为已配置
        let cases = [
            super::ModelConfig {
                model: "qwen2.5".to_string(),
                ..super::ModelConfig::default_text()
            },
            super::ModelConfig {
                model: "qwen3:8b".to_string(),
                ..super::ModelConfig::default_text()
            },
            super::ModelConfig {
                endpoint: "http://192.168.1.8:11434".to_string(),
                model: AiProvider::Ollama.default_model().to_string(),
                ..super::ModelConfig::default_text()
            },
            super::ModelConfig {
                api_key: Some("sk-user".to_string()),
                model: AiProvider::Ollama.default_model().to_string(),
                ..super::ModelConfig::default_text()
            },
            super::ModelConfig {
                provider: AiProvider::OpenAI,
                ..super::ModelConfig::default_text()
            },
        ];
        for case in &cases {
            assert!(
                !case.is_factory_default_text(),
                "带用户修改的配置不应被判为出厂默认: {case:?}"
            );
        }
    }

    #[test]
    fn 空白内置密钥不应生成内置模型() {
        assert!(super::builtin_text_model_with(None).is_none());
        assert!(super::builtin_text_model_with(Some("")).is_none());
        assert!(super::builtin_text_model_with(Some("   ")).is_none());

        let builtin = super::builtin_text_model_with(Some(" sk-builtin ")).expect("应生成内置模型");
        assert_eq!(builtin.api_key.as_deref(), Some("sk-builtin"));
        assert_eq!(builtin.endpoint, "https://laneai.dev/v1");
        assert_eq!(builtin.model, "deepseek-v4-flash");
    }

    #[test]
    fn 兜底解析应仅在未配置时替换为内置模型() {
        let builtin = super::builtin_text_model_with(Some("sk-builtin")).unwrap();

        // 出厂默认 → 内置模型
        let resolved =
            super::effective_text_model_with(Some(&builtin), &super::ModelConfig::default_text());
        assert_eq!(resolved.model, "deepseek-v4-flash");
        assert_eq!(resolved.api_key.as_deref(), Some("sk-builtin"));

        // 用户已配置 → 原样保留
        let configured = super::ModelConfig {
            model: "my-model".to_string(),
            ..super::ModelConfig::default_text()
        };
        let resolved = super::effective_text_model_with(Some(&builtin), &configured);
        assert_eq!(resolved.model, "my-model");
        assert!(resolved.api_key.is_none());

        // 无内置模型（未编译期注入）→ 原样保留出厂默认，行为与旧版一致
        let resolved = super::effective_text_model_with(None, &super::ModelConfig::default_text());
        assert_eq!(resolved.model, "");
        assert!(resolved.api_key.is_none());
    }

    #[test]
    fn 配置文件不存在时应返回首次启动状态() {
        let dir = unique_temp_dir("config-missing");

        let result = AppConfig::load_with_recovery(&dir.join("config.json"));

        assert_eq!(result.status, ConfigLoadStatus::Missing);
        assert!(!result.status.requires_fail_safe());
        assert!(result.status.allows_automatic_save());
    }

    #[test]
    fn 服务商缓存字段应兼容旧配置文件并完整往返() {
        // 模拟旧版配置文件：序列化默认配置后移除新字段再反序列化
        let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
        assert!(legacy
            .as_object_mut()
            .unwrap()
            .remove("text_model_provider_cache")
            .is_some());
        let old: AppConfig = serde_json::from_value(legacy).expect("旧配置缺少缓存字段时应可解析");
        assert!(old.text_model_provider_cache.is_empty());

        // 各家服务商的配置（含 API Key）应无损往返
        let mut config = AppConfig::default();
        config.text_model_provider_cache.insert(
            "deepseek".to_string(),
            super::ModelConfig {
                provider: AiProvider::DeepSeek,
                endpoint: "https://api.deepseek.com".to_string(),
                api_key: Some("sk-test".to_string()),
                model: "deepseek-chat".to_string(),
                ..super::ModelConfig::default_text()
            },
        );
        let round: AppConfig =
            serde_json::from_value(serde_json::to_value(&config).unwrap()).unwrap();
        assert_eq!(
            round
                .text_model_provider_cache
                .get("deepseek")
                .and_then(|m| m.api_key.as_deref()),
            Some("sk-test")
        );
    }

    #[test]
    fn 应用远端同步快照时应保留本机端口与窗口位置() {
        let mut local = AppConfig::default();
        local.localhost_api_port = 12345;
        local.locale = "zh-CN".to_string();
        let mut remote = AppConfig::default();
        remote.localhost_api_port = 47831;
        remote.locale = "en".to_string();
        remote.assistant_timeout_secs = 180;
        local.apply_synced_remote(remote);
        assert_eq!(local.localhost_api_port, 12345);
        assert_eq!(local.locale, "en");
        assert_eq!(local.assistant_timeout_secs, 180);
    }

    #[test]
    fn 旧配置缺少超时与思考字段时应使用默认值() {
        let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
        let object = legacy.as_object_mut().unwrap();
        assert!(object.remove("assistant_timeout_secs").is_some());
        assert!(object.remove("report_generation_timeout_secs").is_some());
        let text_model = object
            .get_mut("text_model")
            .unwrap()
            .as_object_mut()
            .unwrap();
        text_model.remove("enable_thinking");
        text_model.remove("thinking_budget");
        text_model.remove("max_output_tokens");
        let old: AppConfig =
            serde_json::from_value(legacy).expect("旧配置缺少超时与思考字段时应可解析");
        assert_eq!(old.assistant_timeout_secs, 120);
        assert_eq!(old.report_generation_timeout_secs, 300);
        assert_eq!(old.text_model.enable_thinking, None);
        assert_eq!(old.text_model.thinking_budget, None);
        assert_eq!(old.text_model.max_output_tokens, None);
    }

    #[test]
    fn 超时与思考预算应规范化到合法范围() {
        let mut config = AppConfig::default();
        config.assistant_timeout_secs = 1;
        config.report_generation_timeout_secs = 10_000;
        config.text_model.thinking_budget = Some(0);
        config.text_model.max_output_tokens = Some(0);
        config.normalize();
        assert_eq!(config.assistant_timeout_secs, 30);
        assert_eq!(config.report_generation_timeout_secs, 1800);
        assert_eq!(config.text_model.thinking_budget, None);
        assert_eq!(config.text_model.max_output_tokens, None);
    }

    #[test]
    fn 旧配置缺少企微长连接字段时应可解析() {
        let mut legacy = serde_json::to_value(AppConfig::default()).unwrap();
        let object = legacy.as_object_mut().unwrap();
        assert!(object.remove("wecom_bot_id").is_some());
        assert!(object.remove("wecom_bot_secret").is_some());
        let old: AppConfig =
            serde_json::from_value(legacy).expect("旧配置缺少企微长连接字段时应可解析");
        assert_eq!(old.wecom_bot_id, None);
        assert_eq!(old.wecom_bot_secret, None);
    }

    #[test]
    fn 配置规范化应清理企微长连接空白凭证() {
        let mut config = AppConfig {
            wecom_bot_id: Some("  bot-id  ".to_string()),
            wecom_bot_secret: Some("   ".to_string()),
            ..AppConfig::default()
        };
        config.normalize();
        assert_eq!(config.wecom_bot_id.as_deref(), Some("bot-id"));
        assert_eq!(config.wecom_bot_secret, None);
    }

    #[test]
    fn 合法主配置应返回正常加载状态() {
        let dir = unique_temp_dir("config-loaded");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        std::fs::write(
            &path,
            serde_json::to_vec(&AppConfig::default()).expect("配置应可序列化"),
        )
        .expect("应写入合法主配置");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::Loaded);
        assert!(result.primary_error.is_none());
        assert!(result.backup_error.is_none());
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置有效但备份损坏时应保留正常加载状态并报告备份错误() {
        let dir = unique_temp_dir("config-valid-primary-broken-backup");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        let mut primary = AppConfig::default();
        primary.locale = "en".to_string();
        std::fs::write(
            &path,
            serde_json::to_vec(&primary).expect("主配置应可序列化"),
        )
        .expect("应写入合法主配置");
        std::fs::write(&backup_path, b"broken-backup").expect("应写入损坏备份");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::Loaded);
        assert_eq!(result.config.locale, "en");
        assert!(result.primary_error.is_none());
        assert!(result.backup_error.is_some());
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置缺失但备份有效时应从备份恢复() {
        let dir = unique_temp_dir("config-missing-primary-valid-backup");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        let mut backup = AppConfig::default();
        backup.locale = "en".to_string();
        std::fs::write(
            &backup_path,
            serde_json::to_vec(&backup).expect("备份配置应可序列化"),
        )
        .expect("应写入合法备份");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::RecoveredFromBackup);
        assert_eq!(result.config.locale, "en");
        assert!(result.primary_error.is_none());
        assert!(result.backup_error.is_none());
        assert!(!result.status.allows_automatic_save());
        assert!(!path.exists());
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置缺失且备份损坏时应进入故障安全状态() {
        let dir = unique_temp_dir("config-missing-primary-broken-backup");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        std::fs::write(&backup_path, b"broken-backup").expect("应写入损坏备份");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::Corrupted);
        assert!(result.status.requires_fail_safe());
        assert!(result.primary_error.is_none());
        assert!(result.backup_error.is_some());
        assert!(!result.config.storage.screenshots_enabled);
        assert!(!path.exists());
        assert_eq!(
            std::fs::read_to_string(&backup_path).expect("应保留损坏备份"),
            "broken-backup"
        );
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置损坏且备份缺失时应进入故障安全状态() {
        let dir = unique_temp_dir("config-broken-primary-missing-backup");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        std::fs::write(&path, b"broken-primary").expect("应写入损坏主配置");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::Corrupted);
        assert!(result.status.requires_fail_safe());
        assert!(result.primary_error.is_some());
        assert!(result.backup_error.is_none());
        assert!(!result.config.storage.screenshots_enabled);
        assert_eq!(
            std::fs::read_to_string(&path).expect("应保留损坏主配置"),
            "broken-primary"
        );
        assert!(!backup_path.exists());
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 备份路径应在完整文件名后追加bak() {
        let path = PathBuf::from("/tmp/work-review/config.json");

        assert_eq!(
            config_backup_path(&path),
            PathBuf::from("/tmp/work-review/config.json.bak")
        );
    }

    #[test]
    fn 主配置损坏时应从备份恢复且不覆盖主文件() {
        let dir = unique_temp_dir("config-backup-recovery");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        std::fs::write(&path, b"{ broken json").expect("应写入损坏主配置");
        let mut backup = AppConfig::default();
        backup.locale = "en".to_string();
        std::fs::write(
            &backup_path,
            serde_json::to_vec(&backup).expect("备份配置应可序列化"),
        )
        .expect("应写入合法备份");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::RecoveredFromBackup);
        assert_eq!(result.config.locale, "en");
        assert!(result.primary_error.is_some());
        assert!(result.backup_error.is_none());
        assert_eq!(
            std::fs::read_to_string(&path).expect("应读取原主配置"),
            "{ broken json"
        );
        assert!(!result.status.allows_automatic_save());
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置与备份都损坏时应返回关闭采集和网络服务的安全配置() {
        let dir = unique_temp_dir("config-fail-safe");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        std::fs::write(&path, b"broken-primary").expect("应写入损坏主配置");
        std::fs::write(&backup_path, b"broken-backup").expect("应写入损坏备份");

        let result = AppConfig::load_with_recovery(&path);

        assert_eq!(result.status, ConfigLoadStatus::Corrupted);
        assert!(result.status.requires_fail_safe());
        assert!(!result.status.allows_automatic_save());
        assert!(result.primary_error.is_some());
        assert!(result.backup_error.is_some());
        assert!(!result.config.storage.screenshots_enabled);
        assert!(!result.config.localhost_api_enabled);
        assert!(!result.config.feishu_bot_enabled);
        assert!(!result.config.wecom_bot_enabled);
        assert!(!result.config.dingtalk_bot_enabled);
        assert!(!result.config.mcp_server_enabled);
        assert_eq!(
            result.config.remote_storage.provider,
            RemoteStorageProvider::None
        );
        assert_eq!(
            std::fs::read_to_string(&path).expect("应读取原主配置"),
            "broken-primary"
        );
        assert_eq!(
            std::fs::read_to_string(&backup_path).expect("应读取原备份"),
            "broken-backup"
        );
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 覆盖保存应生成可解析备份并写入新配置() {
        let dir = unique_temp_dir("config-atomic-save");
        let path = dir.join("config.json");
        let mut old = AppConfig::default();
        old.locale = "en".to_string();
        old.save(&path).expect("首次保存应成功");
        let mut new = old.clone();
        new.locale = "zh-CN".to_string();

        new.save(&path).expect("覆盖保存应成功");

        assert_eq!(
            AppConfig::load(&path).expect("主配置应可加载").locale,
            "zh-CN"
        );
        assert_eq!(
            AppConfig::load(&config_backup_path(&path))
                .expect("备份应可加载")
                .locale,
            "en"
        );
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主文件提交后父目录同步失败不应返回保存失败() {
        let dir = unique_temp_dir("config-directory-sync-failure");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let (mut pending_config, mut config_file) =
            PendingConfigFile::create(&path).expect("应创建配置临时文件");
        config_file
            .write_all(b"new-config")
            .expect("应写入临时文件");
        config_file.flush().expect("应刷新临时文件");
        config_file.sync_all().expect("应同步临时文件");
        drop(config_file);

        let result = commit_pending_config(&mut pending_config, &path, &dir, |_| {
            Err(io::Error::other("模拟父目录同步失败"))
        });

        assert!(result.is_ok());
        assert_eq!(
            std::fs::read(&path).expect("提交后应读取主文件"),
            b"new-config"
        );
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 备份提交后父目录同步失败不应返回保存失败() {
        let dir = unique_temp_dir("config-backup-directory-sync-failure");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        let original_bytes = b"old-config";
        std::fs::write(&path, original_bytes).expect("应写入原主配置");

        let result =
            update_config_backup_with_sync(&path, |_| Err(io::Error::other("模拟父目录同步失败")));

        assert!(result.is_ok());
        assert_eq!(
            std::fs::read(&backup_path).expect("提交后应读取备份"),
            original_bytes
        );
        assert_eq!(
            std::fs::read(&path).expect("备份更新不应修改主配置"),
            original_bytes
        );
        let names = std::fs::read_dir(&dir)
            .expect("应列出临时目录")
            .map(|entry| {
                entry
                    .expect("目录项应有效")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 主配置不存在时首次保存应成功且不生成备份() {
        let dir = unique_temp_dir("config-first-save-without-backup");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        let config = AppConfig::default();

        config.save(&path).expect("首次保存应成功");

        assert!(path.is_file());
        assert!(!backup_path.exists());
        let names = std::fs::read_dir(&dir)
            .expect("应列出临时目录")
            .map(|entry| {
                entry
                    .expect("目录项应有效")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[cfg(unix)]
    #[test]
    fn 主配置无法打开时应禁止覆盖并清理临时文件() {
        use std::os::unix::fs::symlink;

        let dir = unique_temp_dir("config-primary-open-failure");
        std::fs::create_dir_all(&dir).expect("应创建临时目录");
        let path = dir.join("config.json");
        let backup_path = config_backup_path(&path);
        symlink(&path, &path).expect("应创建自引用符号链接");

        let result = AppConfig::default().save(&path);

        assert!(result.is_err());
        assert!(std::fs::symlink_metadata(&path)
            .expect("原符号链接应保留")
            .file_type()
            .is_symlink());
        assert_eq!(std::fs::read_link(&path).expect("应读取原符号链接"), path);
        assert!(!backup_path.exists());
        let names = std::fs::read_dir(&dir)
            .expect("应列出临时目录")
            .map(|entry| {
                entry
                    .expect("目录项应有效")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[cfg(unix)]
    #[test]
    fn 配置与备份权限应为0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = unique_temp_dir("config-permissions");
        let path = dir.join("config.json");
        let config = AppConfig::default();
        config.save(&path).expect("首次保存应成功");
        config.save(&path).expect("覆盖保存应成功");

        let primary_mode = std::fs::metadata(&path)
            .expect("应读取主配置元数据")
            .permissions()
            .mode()
            & 0o777;
        let backup_mode = std::fs::metadata(config_backup_path(&path))
            .expect("应读取备份元数据")
            .permissions()
            .mode()
            & 0o777;

        assert_eq!(primary_mode, 0o600);
        assert_eq!(backup_mode, 0o600);
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 备份更新失败时应保留原配置并清理临时文件() {
        let dir = unique_temp_dir("config-save-failure");
        let path = dir.join("config.json");
        let original = AppConfig::default();
        original.save(&path).expect("首次保存应成功");
        let backup_path = config_backup_path(&path);
        std::fs::create_dir_all(&backup_path).expect("应创建阻塞备份目录");
        std::fs::write(backup_path.join("blocker"), b"x").expect("应写入阻塞文件");
        let original_bytes = std::fs::read(&path).expect("应读取原配置");

        let mut changed = original.clone();
        changed.locale = "en".to_string();
        assert!(changed.save(&path).is_err());

        assert_eq!(
            std::fs::read(&path).expect("失败后应读取原配置"),
            original_bytes
        );
        let names = std::fs::read_dir(&dir)
            .expect("应列出临时目录")
            .map(|entry| {
                entry
                    .expect("目录项应有效")
                    .file_name()
                    .to_string_lossy()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name.contains(".tmp-")));
        std::fs::remove_dir_all(dir).expect("应清理临时目录");
    }

    #[test]
    fn 界面风格默认应保持当前柔和层次且只允许三套方案() {
        let config = AppConfig::default();

        assert_eq!(config.ui_visual_style, default_ui_visual_style());
        assert_eq!(config.ui_visual_style, "c");
        assert_eq!(normalize_ui_visual_style(" a "), "a");
        assert_eq!(normalize_ui_visual_style("B"), "b");
        assert_eq!(normalize_ui_visual_style("c"), "c");
        assert_eq!(normalize_ui_visual_style("unknown"), "c");
    }

    #[test]
    fn 轻量模式默认应关闭() {
        let config = AppConfig::default();

        assert!(!config.lightweight_mode);
    }

    #[test]
    fn 助手长期记忆默认应关闭() {
        let config = AppConfig::default();

        assert!(!config.assistant_memory_enabled);
    }

    #[test]
    fn 日报附加提示词默认应为空且不导出() {
        let config = AppConfig::default();

        assert!(config.daily_report_custom_prompt.is_empty());
        assert_eq!(config.daily_report_export_dir, None);
    }

    #[test]
    fn 截图模式默认应为活动窗口所在屏幕() {
        let config = AppConfig::default();

        assert_eq!(
            config.storage.screenshot_display_mode,
            ScreenshotDisplayMode::ActiveWindow
        );
    }

    #[test]
    fn 截图开关默认应开启() {
        let config = AppConfig::default();

        assert!(config.storage.screenshots_enabled);
    }

    #[test]
    fn 开机自启动默认应显示主界面() {
        let config = AppConfig::default();

        assert!(!config.auto_start_silent);
    }

    #[test]
    fn macos录屏权限提示默认不应标记为已弹出() {
        let config = AppConfig::default();

        assert!(!config.macos_screen_capture_permission_prompted);
    }

    #[test]
    fn minimax应使用_openai_兼容配置() {
        assert!(AiProvider::MiniMax.is_openai_compatible());
        assert_eq!(
            AiProvider::MiniMax.default_endpoint(),
            "https://api.minimaxi.com/v1"
        );
        assert_eq!(AiProvider::MiniMax.default_model(), "MiniMax-M2.5");
    }

    #[test]
    fn 应用分类规则应去重并规范化分类值() {
        let mut rules = vec![
            AppCategoryRule {
                app_name: " firefox ".to_string(),
                category: "browser".to_string(),
            },
            AppCategoryRule {
                app_name: "Firefox".to_string(),
                category: "office".to_string(),
            },
            AppCategoryRule {
                app_name: "  ".to_string(),
                category: "design".to_string(),
            },
            AppCategoryRule {
                app_name: "MuMu".to_string(),
                category: "unknown".to_string(),
            },
        ];

        normalize_app_category_rules(&mut rules, &[]);

        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].app_name, "Firefox");
        assert_eq!(rules[0].category, "office");
        assert_eq!(rules[1].category, "other");
    }

    #[test]
    fn 网站语义规则应按域名去重并规范化输入() {
        let mut config = AppConfig::default();
        config.website_semantic_rules = vec![
            WebsiteSemanticRule {
                domain: " https://github.com/issues ".to_string(),
                semantic_category: " 任务规划 ".to_string(),
            },
            WebsiteSemanticRule {
                domain: "GitHub.com".to_string(),
                semantic_category: " 资料调研 ".to_string(),
            },
            WebsiteSemanticRule {
                domain: "   ".to_string(),
                semantic_category: "无效".to_string(),
            },
        ];

        config.normalize();

        assert_eq!(config.website_semantic_rules.len(), 1);
        assert_eq!(config.website_semantic_rules[0].domain, "github.com");
        assert_eq!(
            config.website_semantic_rules[0].semantic_category,
            "资料调研"
        );
    }

    #[test]
    fn 配置规范化应清理空白日报附加设置() {
        let mut config = AppConfig {
            daily_report_custom_prompt: "  输出需要更偏周报风格  ".to_string(),
            daily_report_export_dir: Some("   ".to_string()),
            ..AppConfig::default()
        };

        config.normalize();

        assert_eq!(config.daily_report_custom_prompt, "输出需要更偏周报风格");
        assert_eq!(config.daily_report_export_dir, None);
    }

    #[test]
    fn 配置规范化应清理空白远程截图公开地址() {
        let mut config = AppConfig::default();
        config.remote_storage.s3.public_url_base = Some("   ".to_string());
        config.remote_storage.webdav.public_url_base =
            Some(" https://cdn.example.com/base/ ".to_string());

        config.normalize();

        assert_eq!(config.remote_storage.s3.public_url_base, None);
        assert_eq!(
            config.remote_storage.webdav.public_url_base.as_deref(),
            Some("https://cdn.example.com/base/")
        );
    }

    #[test]
    fn 本地api默认应关闭并使用固定默认端口() {
        let config = AppConfig::default();

        assert!(!config.localhost_api_enabled);
        assert_eq!(config.localhost_api_port, DEFAULT_LOCALHOST_API_PORT);
    }

    #[test]
    fn 本地api端口应在规范化时回退到安全默认值() {
        let mut config = AppConfig {
            localhost_api_port: 0,
            ..AppConfig::default()
        };

        config.normalize();

        assert_eq!(config.localhost_api_port, DEFAULT_LOCALHOST_API_PORT);
    }

    #[test]
    fn 节点网关默认不应预填设备名() {
        let config = AppConfig::default();

        assert_eq!(config.node_gateway.device_name, None);
        assert_eq!(config.node_gateway.device_name, None);
    }
}
