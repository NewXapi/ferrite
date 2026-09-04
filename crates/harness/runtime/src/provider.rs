//! Injected chat-completion provider contract.
//!
//! Runtime consumes a normalized OpenAI-compatible stream. HTTP, SSE and
//! provider-specific protocol adapters live above this crate.

use std::pin::Pin;

use futures_util::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use harness_prompt::AgentModelMessage;
use harness_tools::{AgentModelTool, ToolChoice};

use crate::cancel::CancellationToken;

/// Request handed to an injected [`ChatProvider`].
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Streamed increment from a provider.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProviderDelta {
    pub text: Option<String>,
    pub reasoning: Option<String>,
    pub tool_call: Option<ToolCallFragment>,
    pub usage: Option<ProviderUsage>,
    pub finish_reason: Option<ProviderFinishReason>,
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
    #[error("provider error: {0}")]
    Failed(String),
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
    }
}
