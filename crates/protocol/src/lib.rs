//! # protocol — 协议模型与格式转换 (转发域的纯函数心脏)
//!
//! 参考: new-api modules/relaykit (注册表/组合路由/convmeta),
//! one-api relay/adaptor/interface.go (五方法契约),
//! wildtoken internal/proxy/sse.go (跨厂商 usage 提取)。
//!
//! ## 为什么独立成 crate (不塞进 gateway/forward)
//!
//! 转换是**纯函数** (bytes in → bytes out), 不需要 runtime; 放在 gateway 层
//! 会诱使实现者伸手拿 reqwest/tokio, 把协议转换和 IO 纠缠在一起。
//! 独立后: forward 依赖本 crate 做转换; console 复用模型列表/错误规范化;
//! 单测无需 mock 网络。
//!
//! ## 模块地图
//!
//! | 模块 | 内容 |
//! |------|------|
//! | [`error`] | NormalizedError + ProtocolError + 错误码归类 |
//! | [`sse`]   | SSE 帧扫描器 (边界/keepalive/终止原因) |
//! | [`codec`] | 各协议 Codec 实现 + 注册表 (openai/claude/gemini) |
//!
//! ## 设计铁律 (new-api 教训)
//!
//! 1. **单遍流式**: 禁止物化整个请求/响应再 parse (GC 压力 + 丢字段);
//! 2. **usage 保真**: 任何转换链路里 usage 必须无损传递到 metering;
//! 3. **规范化错误**: 上游错误 → NormalizedError 单一形状。

pub mod codec;
pub mod error;
pub mod sse;

pub use codec::claude::ClaudeCodec;
pub use codec::gemini::GeminiCodec;
pub use codec::openai::OpenAiCodec;
pub use codec::{Codec, Protocol, Registry};
pub use error::{ErrorCode, NormalizedError, ProtocolError};
pub use sse::{SseEnd, SseEvent, SseScanner};
