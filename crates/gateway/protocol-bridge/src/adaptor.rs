//! `adaptor` —— 厂商协议适配器（对标 new-api `relay/channel/*/adaptor.go`）
//!
//! 每个厂商一个适配器，负责 **客户端协议 ↔ 厂商协议** 的双向转换（接口兼容）。
//! 原 protocol crate 的 Codec trait 迁入本模块（保持 `(source, target)` 有向
//! 字节转换语义），注册表从单协议查找扩展为组合路由查到 `(source, target)`。
//!
//! 组合语义 (借鉴 relaykit composed routes): Chat→Claude 直转是"直接路由",
//! Chat→Responses→Claude 是"组合路由"。TODO(#503): 组合路由二期, 先覆盖直接路由。

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// 协议族 — 值域对应 contract ChannelRecord.provider_type。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    OpenAi,
    Claude,
    Gemini,
    /// 其它厂商先透传 (零转换), 转换器按需增补。
    Passthrough,
}

/// 请求/响应格式转换器 — 一次尝试内的 (源格式 → 目标格式) 有向转换。
pub trait Codec: Send + Sync {
    fn source(&self) -> Protocol;
    fn target(&self) -> Protocol;
    /// 请求体转换 (一次性; 请求体通常较小)。
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError>;
    /// 响应流转换 (逐块; chunk in → chunks out)。
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError>;
}

/// 适配器错误
#[derive(Debug, Error)]
pub enum AdaptorError {
    #[error("no adaptor for {from:?} -> {to:?}")]
    NotRegistered { from: Protocol, to: Protocol },
    #[error("encode failed: {0}")]
    EncodeFailed(String),
    #[error("decode failed: {0}")]
    DecodeFailed(String),
    #[error("unsupported conversion: {from:?} -> {to:?}")]
    Unsupported { from: Protocol, to: Protocol },
}

/// 适配器注册表 — `(source, target) → Codec` 查找。
/// forward 管道的唯一协议转换入口 — 业务代码不感知具体转换器实现。
pub struct AdaptorRegistry {
    codecs: HashMap<(Protocol, Protocol), Arc<dyn Codec>>,
}

impl AdaptorRegistry {
    pub fn new() -> Self {
        Self {
            codecs: HashMap::new(),
        }
    }

    /// 装载内置适配器 (openai 透传 + claude/gemini 转换骨架)。
    pub fn with_defaults() -> Self {
        let mut r = Self::new();
        r.register_openai();
        r.register_claude();
        r.register_gemini();
        r
    }

    fn register_openai(&mut self) {
        self.codecs
            .insert((Protocol::OpenAi, Protocol::OpenAi), Arc::new(OpenAiCodec));
    }

    fn register_claude(&mut self) {
        self.codecs.insert(
            (Protocol::OpenAi, Protocol::Claude),
            Arc::new(ClaudeCodec { to_claude: true }),
        );
        self.codecs.insert(
            (Protocol::Claude, Protocol::OpenAi),
            Arc::new(ClaudeCodec { to_claude: false }),
        );
    }

    fn register_gemini(&mut self) {
        self.codecs.insert(
            (Protocol::OpenAi, Protocol::Gemini),
            Arc::new(GeminiCodec { to_gemini: true }),
        );
        self.codecs.insert(
            (Protocol::Gemini, Protocol::OpenAi),
            Arc::new(GeminiCodec { to_gemini: false }),
        );
    }

    pub fn register(&mut self, codec: Arc<dyn Codec>) {
        self.codecs.insert((codec.source(), codec.target()), codec);
    }

    /// 查找 (source → target) 直接转换器; None = 不支持该组合。
    pub fn resolve(&self, source: Protocol, target: Protocol) -> Option<Arc<dyn Codec>> {
        if source == target {
            // 同格式透传兜底。
            return Some(Arc::new(PassthroughCodec));
        }
        self.codecs.get(&(source, target)).cloned()
    }
}

impl Default for AdaptorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// 同格式透传 — 零转换。
pub struct PassthroughCodec;

impl Codec for PassthroughCodec {
    fn source(&self) -> Protocol {
        Protocol::Passthrough
    }
    fn target(&self) -> Protocol {
        Protocol::Passthrough
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        Ok(body)
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(vec![chunk])
    }
}

/// OpenAI 兼容适配器 — 事实上的中心格式。
///
/// 设计立场: OpenAI Chat Completions 是**中枢格式**, X→OpenAI 与 OpenAI→Y
/// 各一个方向转换器, N 种协议互转从 O(N²) 降为 O(N)。new-api 的 channel
/// 适配器里 80% 厂商实际"路由到 openai adaptor"也验证了这点。
/// 转换点参考 new-api relay/channel/openai/adaptor.go。
pub struct OpenAiCodec;

impl Codec for OpenAiCodec {
    fn source(&self) -> Protocol {
        Protocol::OpenAi
    }
    fn target(&self) -> Protocol {
        Protocol::OpenAi
    }
    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, AdaptorError> {
        Ok(Bytes::new()) // TODO(#510): 透传; 注入 stream_options 则重写 body
    }
    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(Vec::new()) // TODO(#511): 透传; usage 事件交给 sse::SseScanner
    }
}

/// Claude (Anthropic Messages) 适配器。
/// 转换点参考 new-api relay/channel/claude_handler.go + relaykit 内建。
/// 请求: system 顶层化 / messages 重排 / tool 块映射 / max_tokens 必填默认;
/// 响应: message_start/content_block_delta/message_stop → OpenAI chunk 流。
pub struct ClaudeCodec {
    /// true: 目标是 Claude (入参为 OpenAI 格式); false: 反向。
    pub to_claude: bool,
}

impl Codec for ClaudeCodec {
    fn source(&self) -> Protocol {
        if self.to_claude {
            Protocol::OpenAi
        } else {
            Protocol::Claude
        }
    }
    fn target(&self) -> Protocol {
        if self.to_claude {
            Protocol::Claude
        } else {
            Protocol::OpenAi
        }
    }
    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, AdaptorError> {
        Ok(Bytes::new()) // TODO(#512): OpenAI→Claude 请求映射
    }
    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(Vec::new()) // TODO(#513): Claude SSE→OpenAI chunk 流
    }
}

/// Gemini 适配器。
/// 转换点参考 new-api relay/channel/gemini_handler.go + relaykit 内建。
/// 请求: contents/role 映射, systemInstruction 独立化;
/// 响应: candidates/usageMetadata → OpenAI chunk。
pub struct GeminiCodec {
    /// true: 目标是 Gemini。
    pub to_gemini: bool,
}

impl Codec for GeminiCodec {
    fn source(&self) -> Protocol {
        if self.to_gemini {
            Protocol::OpenAi
        } else {
            Protocol::Gemini
        }
    }
    fn target(&self) -> Protocol {
        if self.to_gemini {
            Protocol::Gemini
        } else {
            Protocol::OpenAi
        }
    }
    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, AdaptorError> {
        Ok(Bytes::new()) // TODO(#514): OpenAI→Gemini 请求映射
    }
    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(Vec::new()) // TODO(#515): Gemini SSE→OpenAI chunk 流
    }
}

impl std::fmt::Debug for dyn Codec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Codec")
            .field("source", &self.source())
            .field("target", &self.target())
            .finish()
    }
}
