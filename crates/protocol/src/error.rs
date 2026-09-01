//! 规范化错误 — 所有上游/网关错误的统一出口。
//!
//! 参考: new-api relaykit types/error.go + channel_error.go (错误归类与重试判定),
//! contract::error::code 常量 (单一来源)。

/// 机器可读错误码 (值域 = contract::error::code 常量)。
pub type ErrorCode = &'static str;

/// 规范化错误 — forward 管道与 dispatch 状态机之间的错误协议。
#[derive(Debug, Clone)]
pub struct NormalizedError {
    /// contract::error::code 常量之一。
    pub code: ErrorCode,
    /// 对客户端暴露的 HTTP 状态。
    pub status: u16,
    /// dispatch 状态机据此决定是否换候选重试。
    pub retryable: bool,
    /// 人类可读信息 (已掩码, 禁止包含上游 key/内部地址)。
    pub message: String,
}

impl NormalizedError {
    /// 从原始上游状态码归类。TODO(#501): 归类表 — 401/403→auth, 429→rate,
    /// 5xx→server, 超时→timeout; 特定上游错误文本 (new-api 的归类规则) 二期。
    pub fn from_status(status: u16, raw_message: impl Into<String>) -> Self {
        let _ = status;
        Self {
            code: contract::error::code::UPSTREAM_ERROR,
            status: 502,
            retryable: false,
            message: raw_message.into(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("malformed body: {0}")]
    Malformed(String),
    #[error("unsupported conversion: {from:?} -> {to:?}")]
    Unsupported { from: crate::codec::Protocol, to: crate::codec::Protocol },
}
