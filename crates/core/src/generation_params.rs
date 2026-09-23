//! 模型生成参数：思考模式 / token 预算按 Provider 协议映射。
//!
//! 只有确认支持的协议才写入请求体；未支持的提供商不发送扩展字段，
//! 避免“设置已保存但实际被忽略”或严格端点 400。

use crate::config::{AiProvider, ModelConfig};
use serde_json::{json, Map, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThinkingProtocol {
    None,
    OllamaThink,
    DashScopeTopLevel,
    SiliconFlowKwargs,
    ClaudeThinking,
    GeminiBudget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationCapabilities {
    pub thinking: ThinkingProtocol,
    pub thinking_streaming_only: bool,
    pub thinking_budget: bool,
    pub max_output_tokens: bool,
}

impl GenerationCapabilities {
    pub fn to_json(self) -> Value {
        json!({
            "thinking": self.thinking != ThinkingProtocol::None,
            "thinkingStreamingOnly": self.thinking_streaming_only,
            "thinkingBudget": self.thinking_budget,
            "maxOutputTokens": self.max_output_tokens,
        })
    }
}

pub fn capabilities_for(provider: AiProvider) -> GenerationCapabilities {
    match provider {
        AiProvider::Ollama | AiProvider::LmStudio => GenerationCapabilities {
            thinking: ThinkingProtocol::OllamaThink,
            thinking_streaming_only: false,
            thinking_budget: false,
            max_output_tokens: true,
        },
        AiProvider::Qwen => GenerationCapabilities {
            thinking: ThinkingProtocol::DashScopeTopLevel,
            thinking_streaming_only: true,
            thinking_budget: true,
            max_output_tokens: true,
        },
        AiProvider::SiliconFlow => GenerationCapabilities {
            thinking: ThinkingProtocol::SiliconFlowKwargs,
            thinking_streaming_only: false,
            thinking_budget: false,
            max_output_tokens: true,
        },
        AiProvider::Claude => GenerationCapabilities {
            thinking: ThinkingProtocol::ClaudeThinking,
            thinking_streaming_only: false,
            thinking_budget: true,
            max_output_tokens: true,
        },
        AiProvider::Gemini => GenerationCapabilities {
            thinking: ThinkingProtocol::GeminiBudget,
            thinking_streaming_only: false,
            thinking_budget: true,
            max_output_tokens: true,
        },
        _ => GenerationCapabilities {
            thinking: ThinkingProtocol::None,
            thinking_streaming_only: false,
            thinking_budget: false,
            max_output_tokens: true,
        },
    }
}

pub fn resolved_max_tokens(config: &ModelConfig, default: u32) -> u32 {
    config
        .max_output_tokens
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

pub fn apply_openai_compatible(body: &mut Value, config: &ModelConfig, streaming: bool) {
    let cap = capabilities_for(config.provider);
    body["max_tokens"] = json!(resolved_max_tokens(config, 8192));

    match cap.thinking {
        ThinkingProtocol::DashScopeTopLevel => {
            if let Some(enabled) = config.enable_thinking {
                if streaming || !enabled {
                    body["enable_thinking"] = json!(enabled);
                    if enabled {
                        if let Some(budget) = config.thinking_budget.filter(|value| *value > 0) {
                            body["thinking_budget"] = json!(budget);
                        }
                    }
                }
            }
        }
        ThinkingProtocol::SiliconFlowKwargs => {
            if let Some(enabled) = config.enable_thinking {
                body["chat_template_kwargs"] = json!({ "enable_thinking": enabled });
            }
        }
        _ => {}
    }
}

pub fn apply_ollama(body: &mut Value, config: &ModelConfig) {
    if let Some(enabled) = config.enable_thinking {
        body["think"] = json!(enabled);
    }
    // 防御性跳过（同 category.rs 口径）：契约靠调用方构造对象字面量维持，
    // 万一违约时跳过参数写入也比 panic 拖垮整个生成流程好
    let Some(obj) = body.as_object_mut() else {
        log::warn!("apply_ollama: body 非对象，跳过 options 写入");
        return;
    };
    let options = obj.entry("options").or_insert_with(|| json!({}));
    options["num_predict"] = json!(resolved_max_tokens(config, 8192));
}

pub fn apply_claude(body: &mut Value, config: &ModelConfig, tools_present: bool) {
    let mut max_tokens = resolved_max_tokens(config, 1600);
    // Claude 开启 thinking 后下一轮必须回传带签名的 thinking 块；
    // 当前消息结构未保留这些块，因此带工具时不发送 thinking，避免多轮被拒。
    if !tools_present {
        match config.enable_thinking {
            Some(true) => {
                let budget = config
                    .thinking_budget
                    .filter(|value| *value >= 1024)
                    .unwrap_or(1024);
                if max_tokens <= budget {
                    max_tokens = budget.saturating_add(1);
                }
                body["thinking"] = json!({
                    "type": "enabled",
                    "budget_tokens": budget
                });
            }
            Some(false) => {
                body["thinking"] = json!({ "type": "disabled" });
            }
            None => {}
        }
    }
    body["max_tokens"] = json!(max_tokens);
}

pub fn apply_gemini(generation_config: &mut Map<String, Value>, config: &ModelConfig) {
    generation_config.insert(
        "maxOutputTokens".to_string(),
        json!(resolved_max_tokens(config, 8192)),
    );
    match config.enable_thinking {
        Some(true) => {
            let budget = config
                .thinking_budget
                .filter(|value| *value > 0)
                .map(i64::from)
                .unwrap_or(-1);
            generation_config.insert(
                "thinkingConfig".to_string(),
                json!({ "thinkingBudget": budget }),
            );
        }
        Some(false) => {
            generation_config.insert("thinkingConfig".to_string(), json!({ "thinkingBudget": 0 }));
        }
        None => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{AiProvider, ModelConfig};

    fn model(provider: AiProvider) -> ModelConfig {
        ModelConfig {
            provider,
            endpoint: "https://example.com".to_string(),
            api_key: None,
            model: "demo".to_string(),
            enable_thinking: Some(true),
            thinking_budget: Some(2048),
            max_output_tokens: Some(4096),
        }
    }

    #[test]
    fn openai兼容默认只发max_tokens不广播chat_template_kwargs() {
        let mut body = json!({ "model": "gpt" });
        apply_openai_compatible(&mut body, &model(AiProvider::OpenAI), true);
        assert_eq!(body["max_tokens"], 4096);
        assert!(body.get("chat_template_kwargs").is_none());
        assert!(body.get("enable_thinking").is_none());
    }

    #[test]
    fn siliconflow应写入chat_template_kwargs() {
        let mut body = json!({});
        apply_openai_compatible(&mut body, &model(AiProvider::SiliconFlow), true);
        assert_eq!(body["chat_template_kwargs"]["enable_thinking"], true);
    }

    #[test]
    fn qwen非流式开启思考时不应发送enable_thinking() {
        let mut body = json!({});
        apply_openai_compatible(&mut body, &model(AiProvider::Qwen), false);
        assert!(body.get("enable_thinking").is_none());
        apply_openai_compatible(&mut body, &model(AiProvider::Qwen), true);
        assert_eq!(body["enable_thinking"], true);
        assert_eq!(body["thinking_budget"], 2048);
    }

    #[test]
    fn ollama应使用think与num_predict() {
        let mut body = json!({});
        apply_ollama(&mut body, &model(AiProvider::Ollama));
        assert_eq!(body["think"], true);
        assert_eq!(body["options"]["num_predict"], 4096);
    }

    #[test]
    fn claude开启思考时max_tokens必须大于budget() {
        let mut config = model(AiProvider::Claude);
        config.max_output_tokens = Some(1000);
        config.thinking_budget = Some(1024);
        let mut body = json!({});
        apply_claude(&mut body, &config, false);
        assert_eq!(body["thinking"]["type"], "enabled");
        assert!(body["max_tokens"].as_u64().unwrap() > 1024);
        let mut with_tools = json!({});
        apply_claude(&mut with_tools, &config, true);
        assert!(with_tools.get("thinking").is_none());
    }

    #[test]
    fn gemini显式开启且无预算时应使用动态负一() {
        let mut config = model(AiProvider::Gemini);
        config.thinking_budget = None;
        let mut gen = Map::new();
        apply_gemini(&mut gen, &config);
        assert_eq!(gen["thinkingConfig"]["thinkingBudget"], -1);
        config.enable_thinking = Some(false);
        let mut off = Map::new();
        apply_gemini(&mut off, &config);
        assert_eq!(off["thinkingConfig"]["thinkingBudget"], 0);
    }

    #[test]
    fn 能力映射应对不支持思考的提供商关闭控件() {
        let openai = capabilities_for(AiProvider::OpenAI);
        assert_eq!(openai.thinking, ThinkingProtocol::None);
        assert!(openai.max_output_tokens);
        assert!(capabilities_for(AiProvider::Qwen).thinking_streaming_only);
    }
}
