//! Provider 适配器枚举与解析。
//!
//! 整文件照抄自 TauriTavern
//! `tt-application/src/services/agent_model_gateway/providers/mod.rs`
//! 和 `tt-application/src/services/agent_model_gateway/format.rs`。

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentProviderAdapter {
    OpenAi,
    OpenAiResponses,
    ClaudeMessages,
    Gemini,
    GeminiInteractions,
}

impl AgentProviderAdapter {
    /// 不同 provider 对 JSON Schema 字段的剔除策略。
    /// OpenAI 几乎不剔除；Claude 剔除 `$schema`/`$id`；
    /// Gemini 广泛剔除（不支持 `$defs`、`additionalProperties`、组合子等）。
    pub const fn schema_keys_to_remove(self) -> &'static [&'static str] {
        match self {
            Self::OpenAi | Self::OpenAiResponses => &[],
            Self::ClaudeMessages => &["$schema", "$id"],
            Self::Gemini | Self::GeminiInteractions => &[
                "$schema",
                "$id",
                "$defs",
                "definitions",
                "additionalProperties",
                "patternProperties",
                "unevaluatedProperties",
                "dependencies",
                "dependentRequired",
                "dependentSchemas",
                "allOf",
                "anyOf",
                "oneOf",
                "not",
                "if",
                "then",
                "else",
                "const",
                "default",
                "examples",
                "title",
            ],
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AdapterError {
    #[error("agent.model_request_invalid_source: unsupported chat completion source `{0}`")]
    UnsupportedSource(String),
}

/// 从 `chat_completion_source` 字符串解析 adapter。
///
/// 接受 `openai` / `openai_responses` / `claude` / `gemini` / `gemini_interactions`。
/// 默认 `openai`。
pub fn resolve_request_adapter(chat_completion_source: &str) -> Result<AgentProviderAdapter, AdapterError> {
    let normalized = chat_completion_source.trim();
    let adapter = match normalized {
        "" | "openai" => AgentProviderAdapter::OpenAi,
        "openai_responses" | "responses" => AgentProviderAdapter::OpenAiResponses,
        "claude" => AgentProviderAdapter::ClaudeMessages,
        "gemini" => AgentProviderAdapter::Gemini,
        "gemini_interactions" => AgentProviderAdapter::GeminiInteractions,
        other => {
            return Err(AdapterError::UnsupportedSource(other.to_string()));
        }
    };
    Ok(adapter)
}

pub fn string_value<'a>(state: &'a Value, key: &str) -> Option<&'a str> {
    state
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn usize_value(state: &Value, key: &str) -> Option<usize> {
    state
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}