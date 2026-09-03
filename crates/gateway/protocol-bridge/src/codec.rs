//! `codec` —— 协议 Codec 注册表
//!
//! 启动时 `CodecRegistry::with_defaults()` 装载 OpenAI / Anthropic / Gemini 三个 codec。
//! 具体 codec 实现来自 `crates/protocol`，本 crate 只做注册和分发。

use std::collections::HashMap;
use std::sync::Arc;
use bytes::Bytes;
use gateway_pipeline::ctx::ProtocolKind;
use gateway_pipeline::ctx::UpstreamResponse;
use thiserror::Error;

/// 协议 codec 错误
#[derive(Debug, Error)]
pub enum CodecError {
    #[error("codec not registered for {0:?}")]
    NotRegistered(ProtocolKind),
    #[error("encode failed: {0}")]
    EncodeFailed(String),
    #[error("decode failed: {0}")]
    DecodeFailed(String),
}

/// 上游请求（解码头）
#[derive(Debug)]
pub struct UpstreamRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Bytes,
}

/// 协议 Codec 抽象
pub trait Codec: Send + Sync {
    /// 上游响应 → 目标协议 HTTP 响应
    fn encode(&self, upstream: UpstreamResponse) -> Result<http::Response<axum::body::Body>, CodecError>;

    /// 客户端请求体 → 上游请求
    fn decode(&self, body: Bytes) -> Result<UpstreamRequest, CodecError>;

    /// 支持的协议
    fn protocol(&self) -> ProtocolKind;
}

/// Codec 注册表
///
/// 启动时 `with_defaults()` 装载内置 codec；可由 `apps/gateway` 扩展。
pub struct CodecRegistry {
    codecs: HashMap<ProtocolKind, Arc<dyn Codec>>,
}

impl CodecRegistry {
    pub fn with_defaults() -> Self {
        let mut codecs: HashMap<ProtocolKind, Arc<dyn Codec>> = HashMap::new();

        // 占位：实际 codec 在 crates/protocol 实现后通过 feature 装载
        // codecs.insert(ProtocolKind::OpenAI, Arc::new(OpenAICodec::new()));
        // codecs.insert(ProtocolKind::Anthropic, Arc::new(AnthropicCodec::new()));
        // codecs.insert(ProtocolKind::Gemini, Arc::new(GeminiCodec::new()));

        Self { codecs }
    }

    pub fn register(&mut self, codec: Arc<dyn Codec>) {
        self.codecs.insert(codec.protocol(), codec);
    }

    pub fn get(&self, kind: ProtocolKind) -> Option<Arc<dyn Codec>> {
        self.codecs.get(&kind).cloned()
    }
}
