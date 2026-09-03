//! Gemini Codec。
//!
//! 转换点参考 new-api relay/channel/gemini_handler.go + relaykit 内建:
//! - 请求: contents/role 映射, systemInstruction 独立化, generationConfig 字段搬移;
//! - 响应: candidates/usageMetadata → OpenAI chunk; finishReason 映射;
//! - 例外: Gemini 无 SSE 的 streamGenerateContent 用 SSE 语义包装 (alt=sse)。

use crate::error::ProtocolError;
use super::{Codec, Protocol};
use bytes::Bytes;

pub struct GeminiCodec {
    /// true: 目标是 Gemini。
    pub to_gemini: bool,
}

impl Codec for GeminiCodec {
    fn source(&self) -> Protocol {
        if self.to_gemini { Protocol::OpenAi } else { Protocol::Gemini }
    }
    fn target(&self) -> Protocol {
        if self.to_gemini { Protocol::Gemini } else { Protocol::OpenAi }
    }

    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, ProtocolError> {
        todo!("TODO(#514): OpenAI→Gemini 请求映射 (contents/systemInstruction/generationConfig)")
    }

    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, ProtocolError> {
        todo!("TODO(#515): Gemini SSE→OpenAI chunk 流 (usageMetadata 保真)")
    }
}
