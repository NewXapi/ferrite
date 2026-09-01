//! # protocol — 协议模型与格式转换 (转发域的纯函数心脏)
//!
//! 参考来源 (调查结论, 详见 todo/spike/new-api-rust-rewrite/06-capability-matrix.md):
//! - new-api `modules/relaykit`: 规范化 DTO + 请求/响应注册表 + convmeta;
//! - one-api `relay/adaptor/interface.go`: 五方法适配器契约
//!   (GetRequestURL / SetupRequestHeader / ConvertRequest / Do / DoResponse);
//! - wildtoken `internal/proxy/sse.go`: 跨厂商 SSE usage 提取 + 首 token 检测。
//!
//! ## 为什么独立成 crate (不塞进 gateway/forward)
//!
//! 转换是**纯函数** (bytes in → bytes out), 不需要 runtime; 放在 gateway 层
//! 会诱使实现者伸手拿 reqwest/tokio, 把协议转换和 IO 纠缠在一起。
//! 独立后: forward 依赖本 crate 做转换; console 可以复用模型列表/错误
//! 规范化; 单测无需 mock 网络。
//!
//! ## 设计铁律 (来自 new-api 的教训)
//!
//! 1. **单遍流式**: 转换器逐块消费/产出, 禁止物化整个请求/响应再 parse
//!    (new-api 的 map[string]interface{} 全量反序列化 → GC 压力 + 丢字段);
//! 2. **usage 保真**: 任何转换链路里 usage 字段必须无损传递到 metering;
//! 3. **规范化错误**: 上游错误 → [`NormalizedError`] 单一形状, code/status/
//!    retryable/掩码 secret 在这里定, 所有调用方共享。

use bytes::Bytes;

/// 协议族 — forward 据此选择 adapter; contract 里 provider_type 字符串的白名单来源。
/// TODO(#500): 与 contract::records::ChannelRecord.provider_type 对齐 (字符串 or 枚举统一)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    OpenAi,
    Claude,
    Gemini,
    /// 其它厂商先透传, 转换器按需增补。
    Passthrough,
}

/// 规范化错误 — 所有上游/网关错误的统一出口 (对齐 relaykit types/error.go)。
#[derive(Debug, Clone)]
pub struct NormalizedError {
    /// 机器可读错误码 ("invalid_api_key", "rate_limited", "upstream_5xx", ...)。
    /// TODO(#501): 错误码清单 — 从 new-api channel_error.go 摘取全集后定表。
    pub code: String,
    /// 对客户端暴露的 HTTP 状态。
    pub status: u16,
    /// dispatch 状态机用它决定是否换候选重试 (对齐 FailureClass)。
    pub retryable: bool,
    /// 人类可读信息 (已掩码, 禁止包含上游 key/内部地址)。
    pub message: String,
}

/// SSE 扫描器: 逐块喂入, 产出帧边界 + 事件元数据。
///
/// 与 metering::StreamScanner 的分工: 本类型只管**帧** (event/data/id 边界、
/// keepalive、终止原因), token 计数是 metering 的事 — 二者串联, 各司其职。
/// 参考 new-api scan_sse.go 的保证: 逐字保真转发、不伪造 [DONE]、有界写缓冲。
/// TODO(#502): 状态机实现 — line buffer + event 聚合 + CRLF 容错。
#[derive(Debug, Default)]
pub struct SseScanner;

impl SseScanner {
    /// 喂入一块上游字节, 返回透传数据 (零拷贝切片) 与扫描到的事件。
    pub fn push(&mut self, chunk: &Bytes) -> (Bytes, Vec<SseEvent>) {
        let _ = chunk;
        (Bytes::new(), Vec::new())
    }
    /// 上游断开: 报告终止原因 (eof/truncated/panic), **不**伪造 [DONE]。
    pub fn finish(self) -> SseEnd { SseEnd::Truncated }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    /// 检测到首个可见 token (含 tool-call delta) — TTFT 计量点。
    FirstToken,
    /// usage 字段出现 (OpenAI stream_options / Claude message_delta)。
    Usage,
    /// 纯 keepalive 帧。
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseEnd {
    Clean,
    Truncated,
    Errored,
}

/// 请求/响应格式转换器 — 一次尝试内的 (源格式 → 目标格式) 有向转换。
///
/// 组合语义 (借鉴 relaykit 的 composed routes): Chat→Claude 直转是"直接路由",
/// Chat→Responses→Claude 是"组合路由"; 转换链与质量标志在链上传播,
/// usage 沿链保真。TODO(#503): 注册表实现 — 直接路由覆盖大部分流量,
/// 组合路由 (chat_completions_via_responses 等) 二期再做。
pub trait Codec: Send + Sync {
    fn source(&self) -> Protocol;
    fn target(&self) -> Protocol;
    /// 请求体转换 (一次性; 请求体通常小)。
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, ProtocolError>;
    /// 响应流转换 (逐块; 支持流式)。
    /// TODO(#504): 签名里流类型定型 — Bytes 迭代器还是 channel? 接线 forward 时定。
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, ProtocolError>;
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("malformed body: {0}")]
    Malformed(String),
    #[error("unsupported conversion: {from:?} -> {to:?}")]
    Unsupported { from: Protocol, to: Protocol },
}
