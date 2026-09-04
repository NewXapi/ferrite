//! `error` —— 拒绝原因 + OpenAI 风格 4xx 响应体

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum Rejection {
    /// 401 — 找不到 key
    #[error("invalid api key")]
    InvalidApiKey,

    /// 401 — token 过期
    #[error("token expired")]
    TokenExpired,

    /// 401 — 找不到 user
    #[error("user not found")]
    UserNotFound,

    /// 403 — user 已被禁用
    #[error("user disabled")]
    UserDisabled,

    /// 401/403 — auth_version 不匹配（旧 token 立即失效）
    #[error("auth version mismatch")]
    TokenAuthVersionMismatch,

    /// 403 — IP 不在白名单
    #[error("ip not allowed")]
    IpNotAllowed,

    /// 500 — 内部错误：gate 顺序错
    #[error("auth skipped: previous gate did not fill token")]
    AuthSkipped,

    /// 402 — 余额不足
    #[error("insufficient quota: remaining={remaining} cost={cost}")]
    InsufficientQuota { remaining: i64, cost: i64 },

    /// 429 — 速率超限
    #[error("rate limited")]
    RateLimited,

    /// 400 — 请求体缺 model
    #[error("model not specified")]
    ModelNotSpecified,

    /// 403 — 模型在白名单外
    #[error("model forbidden: {model}")]
    ModelForbidden { model: String },

    /// 429 — 灰名单封禁中
    #[error("graylisted")]
    Graylisted,

    /// 429 — 并发槽满
    #[error("concurrency exhausted")]
    ConcurrencyExhausted,

    /// 413 — 请求体过大
    #[error("payload too large")]
    PayloadTooLarge,
}

/// 拒绝 → HTTP 响应
pub fn rejection_to_response(rej: Rejection) -> Response {
    use Rejection::*;
    let (status, code, message) = match rej {
        InvalidApiKey => (StatusCode::UNAUTHORIZED, "invalid_api_key", rej.to_string()),
        TokenExpired => (StatusCode::UNAUTHORIZED, "token_expired", rej.to_string()),
        UserNotFound => (StatusCode::UNAUTHORIZED, "user_not_found", rej.to_string()),
        UserDisabled => (StatusCode::FORBIDDEN, "user_disabled", rej.to_string()),
        TokenAuthVersionMismatch => (
            StatusCode::UNAUTHORIZED,
            "auth_version_mismatch",
            rej.to_string(),
        ),
        IpNotAllowed => (StatusCode::FORBIDDEN, "ip_not_allowed", rej.to_string()),
        AuthSkipped => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            rej.to_string(),
        ),
        InsufficientQuota { .. } => (
            StatusCode::PAYMENT_REQUIRED,
            "insufficient_quota",
            rej.to_string(),
        ),
        RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            rej.to_string(),
        ),
        ModelNotSpecified => (
            StatusCode::BAD_REQUEST,
            "model_not_specified",
            rej.to_string(),
        ),
        ModelForbidden { .. } => (StatusCode::FORBIDDEN, "model_forbidden", rej.to_string()),
        Graylisted => (StatusCode::TOO_MANY_REQUESTS, "graylisted", rej.to_string()),
        ConcurrencyExhausted => (
            StatusCode::TOO_MANY_REQUESTS,
            "concurrency_exhausted",
            rej.to_string(),
        ),
        PayloadTooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            rej.to_string(),
        ),
    };

    let body = json!({
        "error": {
            "code": code,
            "message": message,
        }
    });
    (status, axum::Json(body)).into_response()
}
