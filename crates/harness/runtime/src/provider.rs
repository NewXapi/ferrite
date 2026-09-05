//! Injected chat-completion provider contract.
//!
//! Runtime consumes a normalized OpenAI-compatible stream. HTTP, SSE and
//! provider-specific protocol adapters live above this crate.

use std::pin::Pin;

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use harness_prompt::AgentModelMessage;
use harness_tools::{AgentModelTool, ToolChoice};

use crate::cancel::CancellationToken;

/// Request handed to an injected [`ChatProvider`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderRequest {
    pub model: String,
    pub messages: Vec<AgentModelMessage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AgentModelTool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    /// OpenAI `response_format`（jsonSchema 等）原样透传；`None` 不下发。
    /// 对齐 ST `generateQuietPrompt` 的 jsonSchema 透传（script.js:3019-3043）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
    /// CFG scale（classifier-free guidance）；text-completion 系 provider 支持。
    /// 对齐 ST `cfg-scale.js`。`None` 不下发。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg_scale: Option<f32>,
    /// OpenAI `logit_bias`：token_id 字符串 → 偏置值（-100..100）。
    /// 对齐 ST `logit-bias.js getLogitBiasListResult`。`None` 不下发。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logit_bias: Option<std::collections::BTreeMap<String, i32>>,
    /// OpenAI `logprobs`：启用概率日志并指定 top-K；\`None\` 不下发。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<LogprobsConfig>,
}

/// Streamed increment from an injected [`ChatProvider`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProviderDelta {
    pub text: Option<String>,
    pub reasoning: Option<String>,
    pub tool_call: Option<ToolCallFragment>,
    pub usage: Option<ProviderUsage>,
    pub finish_reason: Option<ProviderFinishReason>,
    /// 流式 logprobs 记录（仅在启用 logprobs 时填充）。
    /// 对齐 ST `logprobs.js`。
    pub logprobs: Option<Vec<LogprobItem>>,
}

/// 单条 logprob 记录。
#[derive(Debug, Clone, PartialEq)]
pub struct LogprobItem {
    pub token: String,
    pub logprob: f32,
    pub bytes: Option<Vec<u8>>,
}
/// \`logprobs\` 子配置。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LogprobsConfig {
    /// 0-20；启用 logprobs 时必填 top_logprobs 数量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_logprobs: Option<u32>,
}

/// Incremental OpenAI-style tool-call fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallFragment {
    pub index: usize,
    pub call_id: Option<String>,
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// Token usage reported by a provider.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

/// Normalized finish reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderFinishReason {
    Stop,
    Length,
    ToolCalls,
    Cancelled,
    Error,
}

/// Provider-side failure.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProviderError {
    #[error("provider cancelled")]
    Cancelled,
    /// 瞬时失败：网络抖动、上游 5xx 等，值得按 retry 预算重试。
    #[error("provider error: {0}")]
    Failed(String),
    /// 确定性失败：401/403、模型不存在、请求非法等。重放同一请求必然
    /// 再失败，重试只浪费配额与时间。
    #[error("provider rejected: {0}")]
    Permanent(String),
}

/// Stream of provider deltas.
pub type ProviderStream = Pin<Box<dyn Stream<Item = Result<ProviderDelta, ProviderError>> + Send>>;

/// Injected chat-completion source.
pub trait ChatProvider: Send + Sync {
    fn stream(&self, request: ProviderRequest, cancel: CancellationToken) -> ProviderStream;
}

/// Convenience constructor for tests.
pub fn empty_request(
    model: impl Into<String>,
    messages: Vec<AgentModelMessage>,
) -> ProviderRequest {
    ProviderRequest {
        model: model.into(),
        messages,
        tools: Vec::new(),
        tool_choice: None,
        temperature: None,
        top_p: None,
        max_tokens: None,
        stop: Vec::new(),
        response_format: None,
        cfg_scale: None,
        logit_bias: None,
        logprobs: None,
    }
}
