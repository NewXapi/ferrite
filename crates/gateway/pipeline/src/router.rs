//! `router` —— axum 集成 + 错误响应转换
//!
//! 把 Pipeline 接到 axum `fallback`，统一处理 RequestCtx 构造 + 错误转换。

use std::convert::Infallible;
use std::sync::Arc;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use crate::stage::StageError;
use serde_json::json;
use crate::ctx::RequestCtx;
use crate::error_mapping::StageError;
use crate::pipeline::Pipeline;

/// 构造 axum Router，把 Pipeline 接到 fallback
pub fn build_router(pipeline: Arc<Pipeline>) -> axum::Router {
    async fn dispatch(
        State(pipeline): State<Arc<Pipeline>>,
        req: Request<Body>,
    ) -> Result<Response, Infallible> {
        let ctx = match RequestCtx::from_axum(req).await {
            Ok(c) => c,
            Err(e) => return Ok(error_to_response(StageError::Internal(e))),
        };
        match pipeline.run(ctx).await {
            Ok(resp) => Ok(resp),
            Err(e) => Ok(error_to_response(e)),
        }
    }

    axum::Router::new()
        .fallback(dispatch)
        .with_state(pipeline)
}

/// 错误 → HTTP 响应（OpenAI 错误形状）
pub fn error_to_response(e: StageError) -> Response {
    use StageError::*;
    let (status, code, message) = match e {
        Unauthenticated(msg) => (StatusCode::UNAUTHORIZED, "unauthenticated", msg),
        QuotaExhausted { remaining, required } => (
            StatusCode::PAYMENT_REQUIRED,
            "quota_exhausted",
            format!("remaining={} required={}", remaining, required),
        ),
        Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
        NoRoute => (StatusCode::NOT_FOUND, "no_route", "no available channel".to_string()),
        PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "payload_too_large", "request body too large".to_string()),
        Upstream(ue) => (StatusCode::BAD_GATEWAY, "upstream_error", ue.to_string()),
        Internal(err) => {
            tracing::error!(error = %err, "internal stage error");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal", "internal error".to_string())
        }
    };

    let body = json!({
        "error": {
            "code": code,
            "message": message,
        }
    });
    (status, axum::Json(body)).into_response()
}
