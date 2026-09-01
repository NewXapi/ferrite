//! Claude (Anthropic Messages) Codec。
//!
//! 转换点参考 new-api relay/channel/claude_handler.go + relaykit 内建:
//! - 请求: system 顶层化 / messages 重排 / tool 块映射 / max_tokens 必填默认;
//! - 响应: message_start/content_block_delta/message_stop → OpenAI chunk 流;
//!   usage 在 message_delta 里 (input_tokens 开头给, output_tokens 结尾给)。

use crate::error::ProtocolError;
use super::{Codec, Protocol};
use bytes::Bytes;

/// 方向由构造参数决定 (X→Claude 或 Claude→X 共享本结构, trait 实现两个方向)。
pub struct ClaudeCodec {
    /// true: 目标是 Claude (入参为 OpenAI 格式); false: 反向。
    pub to_claude: bool,
}

impl Codec for ClaudeCodec {
    fn source(&self) -> Protocol {
        if self.to_claude { Protocol::OpenAi } else { Protocol::Claude }
    }
    fn target(&self) -> Protocol {
        if self.to_claude { Protocol::Claude } else { Protocol::OpenAi }
    }

    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, ProtocolError> {
        todo!("TODO(#512): OpenAI→Claude 请求映射 (system 顶层化/tool 映射/max_tokens)")
    }

    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, ProtocolError> {
        todo!("TODO(#513): Claude SSE→OpenAI chunk 流 (message_delta usage 保真)")
    }
}
