//! protocol crate 模块树。
//!
//! codec/ 目录: mod.rs (trait + Registry) + openai.rs + claude.rs + gemini.rs

pub mod claude;
pub mod gemini;
pub mod openai;

use crate::error::ProtocolError;
use bytes::Bytes;

/// 协议族 — forward 据此选择 Codec; 值域对应 contract ChannelRecord.provider_type。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Protocol {
    OpenAi,
    Claude,
    Gemini,
    /// 其它厂商先透传 (零转换), 转换器按需增补。
    Passthrough,
}

/// 请求/响应格式转换器 — 一次尝试内的 (源格式 → 目标格式) 有向转换。
///
/// 组合语义 (借鉴 relaykit composed routes): Chat→Claude 直转是"直接路由",
/// Chat→Responses→Claude 是"组合路由"。TODO(#503): 组合路由二期, 先覆盖直接路由。
pub trait Codec: Send + Sync {
    fn source(&self) -> Protocol;
    fn target(&self) -> Protocol;
    /// 请求体转换 (一次性; 请求体通常较小)。
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, ProtocolError>;
    /// 响应流转换 (逐块; chunk in → chunks out)。
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, ProtocolError>;
}

/// 转换器注册表: (source, target) → Codec 查找。
/// forward 管道的唯一入口 — 业务代码不感知具体转换器实现。
pub trait Registry: Send + Sync {
    /// 查找 (source → target) 直接转换器; None = 不支持该组合。
    /// TODO(#503): 实现持 HashMap<(Protocol, Protocol), Arc<dyn Codec>>;
    /// Passthrough 对永远返回透传 Codec。
    fn resolve(&self, source: Protocol, target: Protocol) -> Option<std::sync::Arc<dyn Codec>>;
}
