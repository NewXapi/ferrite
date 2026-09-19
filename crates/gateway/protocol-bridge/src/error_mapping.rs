//! `error_mapping` —— NormalizedError → 各协议错误形状
//!
//! 唯一内部错误出口 (`contract::error::NormalizedError`)，本模块
//! 负责把它映射成 OpenAI Chat / OpenAI Responses / Anthropic / Gemini
//! 各自规范的错误响应体。

use axum::body::Body;
use bytes::Bytes;
use contract::error::NormalizedError;
use gateway_pipeline::ctx::ProtocolKind;
use gateway_pipeline::error::StageError;
use http::Response;

/// OpenAI Chat Completions 兼容错误形状 (SDK 解析用)。
pub fn to_openai_shape(err: &NormalizedError) -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": err.code,
            "message": err.message,
            "type": classify_openai_error_type(err.code),
        }
    })
}

/// OpenAI Responses 错误形状。
///
/// Responses 与 Chat 同属 OpenAI 但错误体不同：客户端 SDK 读的是
/// `error.{code,message,type}` 且 **`code` 是可空字符串**（不是内部错误码），
/// `type` 才是分类。旧实现在 [`map_error`] 里把 `OpenAIResp` 与 `OpenAI` 归并到
/// 同一个分支，Responses 客户端会拿到 Chat 形状——这里拆开。
pub fn to_responses_shape(err: &NormalizedError) -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": serde_json::Value::Null,
            "message": err.message,
            "type": classify_openai_error_type(err.code),
        }
    })
}

/// Anthropic 错误形状。
pub fn to_anthropic_shape(err: &NormalizedError) -> serde_json::Value {
    serde_json::json!({
        "type": "error",
        "error": {
            "type": classify_anthropic_error_type(err.code),
            "message": err.message,
        }
    })
}

/// Gemini 错误形状。
pub fn to_gemini_shape(err: &NormalizedError) -> serde_json::Value {
    serde_json::json!({
        "error": {
            "code": err.code,
            "message": err.message,
            "status": gemini_status_name(err.status),
        }
    })
}

fn classify_openai_error_type(code: &str) -> &'static str {
    if code.contains("auth") {
        "invalid_api_key"
    } else if code.contains("quota") {
        "insufficient_quota"
    } else if code.contains("rate") {
        "rate_limit_error"
    } else {
        "api_error"
    }
}

fn classify_anthropic_error_type(code: &str) -> &'static str {
    if code.contains("auth") {
        "authentication_error"
    } else if code.contains("rate") {
        "rate_limit_error"
    } else if code.contains("quota") {
        "billing_error"
    } else {
        "api_error"
    }
}

fn gemini_status_name(status: u16) -> &'static str {
    match status {
        400 => "INVALID_ARGUMENT",
        401 => "UNAUTHENTICATED",
        403 => "PERMISSION_DENIED",
        404 => "NOT_FOUND",
        429 => "RESOURCE_EXHAUSTED",
        500..=599 => "INTERNAL",
        _ => "UNKNOWN",
    }
}

/// `StageError` → 各协议错误形状的 HTTP Response
pub fn map_error(e: StageError, target: ProtocolKind) -> Response<Body> {
    let normalized = NormalizedError {
        code: error_code(&e),
        message: e.to_string(),
        status: error_status(&e).as_u16(),
        retryable: false,
        channel_scoped: false,
    };

    let (status, body) = match target {
        ProtocolKind::OpenAI => (
            http::StatusCode::from_u16(normalized.status)
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
            to_openai_shape(&normalized),
        ),
        // Responses 与 Chat 是两种响应形状，错误体同样分开——旧实现两者归并，
        // Responses 客户端会拿到 Chat 形状。
        ProtocolKind::OpenAIResp => (
            http::StatusCode::from_u16(normalized.status)
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
            to_responses_shape(&normalized),
        ),
        ProtocolKind::Anthropic => (
            http::StatusCode::from_u16(normalized.status)
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
            to_anthropic_shape(&normalized),
        ),
        ProtocolKind::Gemini => (
            http::StatusCode::from_u16(normalized.status)
                .unwrap_or(http::StatusCode::INTERNAL_SERVER_ERROR),
            to_gemini_shape(&normalized),
        ),
    };

    let body_bytes = Bytes::from(serde_json::to_vec(&body).unwrap_or_default());
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(Body::from(body_bytes))
        .unwrap_or_else(|_| Response::new(Body::empty()))
}

fn error_code(e: &StageError) -> &'static str {
    use gateway_pipeline::error::StageError::*;
    match e {
        Unauthenticated(_) => "unauthenticated",
        QuotaExhausted { .. } => "quota_exhausted",
        Forbidden(_) => "forbidden",
        NoRoute => "no_route",
        RateLimited => "rate_limited",
        NotReady => "service_not_ready",
        PayloadTooLarge => "payload_too_large",
        Upstream(_) => "upstream_error",
        Internal(_) => "internal",
    }
}

fn error_status(e: &StageError) -> http::StatusCode {
    use gateway_pipeline::error::StageError::*;
    match e {
        Unauthenticated(_) => http::StatusCode::UNAUTHORIZED,
        QuotaExhausted { .. } => http::StatusCode::PAYMENT_REQUIRED,
        Forbidden(_) => http::StatusCode::FORBIDDEN,
        NoRoute => http::StatusCode::NOT_FOUND,
        // 与 gateway_pipeline::router::error_to_response 的 RateLimited 映射一致。
        RateLimited => http::StatusCode::TOO_MANY_REQUESTS,
        NotReady => http::StatusCode::SERVICE_UNAVAILABLE,
        PayloadTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
        Upstream(_) => http::StatusCode::BAD_GATEWAY,
        Internal(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
}
