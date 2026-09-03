//! OpenAI 兼容 Codec — 事实上的中心格式。
//!
//! 设计立场 (调查结论): OpenAI Chat Completions 是**中枢格式**。
//! X→OpenAI 与 OpenAI→Y 各一个方向转换器, N 种协议互转从 O(N²) 降为 O(N)。
//! new-api 的 channel 适配器里 80% 厂商实际"路由到 openai adaptor"也验证了这点。
//!
//! 职责:
//! - 请求: stream_options.include_usage 注入 (wildtoken 同款兼容规则);
//! - 响应: SSE 里 usage 抽取标记 (喂给 SseEvent::Usage);
//! - 兼容: text-only content-part 数组拍平, reasoning_content 回填。

use crate::error::ProtocolError;
use super::{Codec, Protocol};
use bytes::Bytes;

pub struct OpenAiCodec;

impl Codec for OpenAiCodec {
    fn source(&self) -> Protocol { Protocol::OpenAi }
    fn target(&self) -> Protocol { Protocol::OpenAi }

    fn adapt_request(&self, _body: Bytes) -> Result<Bytes, ProtocolError> {
        todo!("TODO(#510): 同格式走透传; 若注入 stream_options 则重写 body")
    }

    fn adapt_response(&self, _chunk: Bytes) -> Result<Vec<Bytes>, ProtocolError> {
        todo!("TODO(#511): 同格式透传; usage 事件检测交给 SseScanner")
    }
}
