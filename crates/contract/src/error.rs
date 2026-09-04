//! # error — 跨端共享错误码表
//!
//! 单一错误码词汇表: gateway 错误体、console 信封 message、web 前端 i18n
//! 三方共用。对齐 new-api relaykit types/channel_error.go 的归类思路。
//!
//! ## 错误码 → HTTP 状态 → 客户端行为
//!
//! | code                    | HTTP | 客户端行为 |
//! |-------------------------|------|-----------|
//! | INVALID_API_KEY         | 401  | 停止重试, 重新鉴权 |
//! | TOKEN_EXPIRED           | 401  | 刷新后重试一次 |
//! | INSUFFICIENT_QUOTA      | 402  | 引导充值 |
//! | MODEL_FORBIDDEN         | 403  | 停止重试 |
//! | RATE_LIMITED            | 429  | 指数退避重试 |
//! | NO_CANDIDATE            | 404  | 模型/分组无可用路由 |
//! | UPSTREAM_ERROR          | 502  | 网关已重试, 上游仍失败 |
//! | CATALOG_NOT_READY       | 503  | 稍后重试 (启动期) |
//! | INTERNAL                | 500  | 联系管理员 |

/// 错误码常量 — serde 序列化为字符串。
pub mod code {
    pub const INVALID_API_KEY: &str = "invalid_api_key";
    pub const TOKEN_EXPIRED: &str = "token_expired";
    pub const INSUFFICIENT_QUOTA: &str = "insufficient_quota";
    pub const MODEL_FORBIDDEN: &str = "model_forbidden";
    pub const RATE_LIMITED: &str = "rate_limited";
    pub const NO_CANDIDATE: &str = "no_candidate";
    pub const UPSTREAM_ERROR: &str = "upstream_error";
    pub const CATALOG_NOT_READY: &str = "catalog_not_ready";
    pub const INTERNAL: &str = "internal";
}

/// gateway /v1 错误体 (OpenAI 风格, 兼容主流 SDK 解析)。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayErrorBody {
    pub error: GatewayErrorDetail,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayErrorDetail {
    /// contract::error::code 常量之一。
    pub code: String,
    /// 已掩码的人类可读信息 (禁止包含上游 key/内部地址)。
    pub message: String,
    /// OpenAI 兼容字段 (SDK 读取)。
    pub r#type: String,
}

impl GatewayErrorBody {
    /// 从 NormalizedError 构造 OpenAI 风格错误体 (SDK 兼容形状, TODO(#214) 落定)。
    pub fn from_normalized(err: &NormalizedError) -> Self {
        Self {
            error: GatewayErrorDetail {
                code: err.code.to_string(),
                message: err.message.clone(),
                r#type: match err.status {
                    401 => "authentication_error".into(),
                    402 => "insufficient_quota".into(),
                    403 => "permission_error".into(),
                    404 => "not_found".into(),
                    429 => "rate_limit_error".into(),
                    500..=599 => "api_error".into(),
                    _ => "invalid_request_error".into(),
                },
            },
        }
    }
}

/// 机器可读错误码 (值域 = [`code`] 常量)。
pub type ErrorCode = &'static str;

/// 规范化错误 — 网关内部流转与对外暴露的单一错误协议。
///
/// 由 forward 的上游归类产生, dispatch 状态机据此决定重试/换候选,
/// protocol-bridge 把它映射成各协议错误形状。跨 crate 共享, 放 contract。
#[derive(Debug, Clone, PartialEq)]
pub struct NormalizedError {
    pub code: ErrorCode,
    /// 对客户端暴露的 HTTP 状态。
    pub status: u16,
    /// dispatch 状态机据此决定是否换候选重试。
    pub retryable: bool,
    /// 人类可读信息 (已掩码, 禁止包含上游 key/内部地址)。
    pub message: String,
}

impl NormalizedError {
    /// 按上游状态码归类 — TODO(#501) 规则表雏形:
    /// 401/403 → auth (不重试), 429 → 限流 (可重试), 5xx → 上游 (可重试),
    /// 其它 4xx → 请求问题 (不重试), 其它 → 502 兜底。
    pub fn from_status(status: u16, raw_message: impl Into<String>) -> Self {
        let raw = raw_message.into();
        let (code, http_status, retryable, message) = match status {
            401 | 403 => (code::INVALID_API_KEY, status, false, raw),
            429 => (code::RATE_LIMITED, status, true, raw),
            500..=599 => (code::UPSTREAM_ERROR, status, true, raw),
            400..=499 => (code::INVALID_API_KEY, status, false, raw),
            _ => (code::UPSTREAM_ERROR, 502, false, raw),
        };
        Self {
            code,
            status: http_status,
            retryable,
            message,
        }
    }
}
