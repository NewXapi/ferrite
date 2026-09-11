//! `router` —— axum 集成 + 错误响应转换
//!
//! 把 Pipeline 接到 axum `fallback`，统一处理 RequestCtx 构造 + 错误转换。

use crate::ctx::RequestCtx;
use crate::pipeline::Pipeline;
use crate::stage::StageError;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;

/// 构造 axum Router：`/healthz` 独立成路由，其余全部落到 pipeline 的 fallback。
///
/// `/healthz` 必须绕开 pipeline：链首是 gate，任何没带合法 key 的请求都会被
/// `AuthGate` 判 401，健康检查也不例外。它只报进程活着，不体现上游可用性。
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

    async fn healthz() -> Response {
        axum::Json(json!({ "status": "ok" })).into_response()
    }

    axum::Router::new()
        .route("/healthz", axum::routing::get(healthz))
        .fallback(dispatch)
        .with_state(pipeline)
}

/// 错误 → HTTP 响应（OpenAI 错误形状）
pub fn error_to_response(e: StageError) -> Response {
    use StageError::*;
    let (status, code, message) = match e {
        Unauthenticated(msg) => (StatusCode::UNAUTHORIZED, "unauthenticated", msg),
        QuotaExhausted {
            remaining,
            required,
        } => (
            StatusCode::PAYMENT_REQUIRED,
            "quota_exhausted",
            format!("remaining={} required={}", remaining, required),
        ),
        Forbidden(msg) => (StatusCode::FORBIDDEN, "forbidden", msg),
        NoRoute => (
            StatusCode::NOT_FOUND,
            "no_route",
            "no available channel".to_string(),
        ),
        RateLimited => (
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "all candidates rate limited".to_string(),
        ),
        NotReady => (
            StatusCode::SERVICE_UNAVAILABLE,
            "service_not_ready",
            "gateway not ready".to_string(),
        ),
        PayloadTooLarge => (
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "request body too large".to_string(),
        ),
        Upstream(ue) => (StatusCode::BAD_GATEWAY, "upstream_error", ue.to_string()),
        Internal(err) => {
            tracing::error!(error = %err, "internal stage error");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal",
                "internal error".to_string(),
            )
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
