//! `adaptor` —— 协议转换的共享词汇：格式标识与错误类型。
//!
//! 转换逻辑本身不在本模块：单格式双向 codec 在 [`crate::format_codec`] 的
//! `FormatCodec` trait 之下（`openai` / `claude` / `gemini` / `responses` 四个实现），
//! 注册与两跳调度在 `FormatRegistry`。
//!
//! 本模块只保留所有 codec 都要引用的两样东西——[`Protocol`] 与 [`AdaptorError`]——
//! 以免它们与任一具体 codec 耦合。

use thiserror::Error;

/// 协议族 —— 值域对应 contract `ChannelRecord.provider_type`。
///
/// 既是「入站格式」（客户端说的协议，由请求路径判定）也是「上游格式」
/// （渠道 `provider_type` 决定），两个方向共用同一枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// OpenAI Chat Completions（`/v1/chat/completions`）。
    OpenAi,
    /// OpenAI Responses API（`/v1/responses`）—— 与 Chat Completions 同名不同形，
    /// 对齐 `gateway_pipeline::ctx::ProtocolKind::OpenAIResp`。
    OpenAIResp,
    /// Anthropic Messages（`/v1/messages`）。
    Claude,
    /// Gemini `generateContent`（`/v1beta/...:generateContent`）。
    Gemini,
    /// 其它厂商：不做转换，字节原样透传。
    Passthrough,
}

/// 协议转换错误。
#[derive(Debug, Error)]
pub enum AdaptorError {
    #[error("no codec registered for {from:?} -> {to:?}")]
    NotRegistered { from: Protocol, to: Protocol },
    #[error("encode failed: {0}")]
    EncodeFailed(String),
    #[error("decode failed: {0}")]
    DecodeFailed(String),
    #[error("unsupported conversion: {from:?} -> {to:?}")]
    Unsupported { from: Protocol, to: Protocol },
}

impl Protocol {
    /// 入站协议判定结果（pipeline 侧）→ 本 crate 的格式标识。
    ///
    /// 两者值域一一对应但分属不同 crate：`ProtocolKind` 是 pipeline 的公开契约
    /// （stage 之间传递），`Protocol` 只服务于协议转换。转换集中在这里，避免
    /// 每个调用点各写一份 match。
    pub fn from_kind(kind: gateway_pipeline::ctx::ProtocolKind) -> Self {
        match kind {
            gateway_pipeline::ctx::ProtocolKind::OpenAI => Protocol::OpenAi,
            gateway_pipeline::ctx::ProtocolKind::OpenAIResp => Protocol::OpenAIResp,
            gateway_pipeline::ctx::ProtocolKind::Anthropic => Protocol::Claude,
            gateway_pipeline::ctx::ProtocolKind::Gemini => Protocol::Gemini,
        }
    }

    /// 渠道 `provider_type` → 上游格式。未知值按透传处理（零转换最安全）。
    pub fn from_provider_type(provider_type: &str) -> Self {
        match provider_type {
            "claude" | "anthropic" => Protocol::Claude,
            "gemini" | "google" => Protocol::Gemini,
            "openai-resp" | "openai-responses" | "responses" => Protocol::OpenAIResp,
            "openai" | "" => Protocol::OpenAi,
            _ => Protocol::Passthrough,
        }
    }
}
