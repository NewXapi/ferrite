//! HTTP 路由 + handler

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use futures_util::StreamExt;

use crate::adapter::{OpenAIAdapter, StreamResponse};
use crate::config::PgPool;
use crate::dispatch::RouteIndex;

pub struct AppState {
    pub pool: PgPool,
    pub route_index: RouteIndex,
    pub adapter: OpenAIAdapter,
}

pub struct Gateway {
    state: Arc<AppState>,
}

impl Gateway {
    pub fn new(pool: PgPool, route_index: RouteIndex) -> Self {
        Self {
            state: Arc::new(AppState {
                pool,
                route_index,
                adapter: OpenAIAdapter::new(),
            }),
        }
    }

    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(health))
            .route("/v1/models", get(list_models))
            .route("/v1/chat/completions", post(chat_completions))
            .route("/admin/reload", post(reload_channels))
            .with_state(self.state.clone())
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// 构造 OpenAI 标准错误响应（application/json）
fn error_response(status: StatusCode, message: &str, error_type: &str) -> axum::response::Response {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": error_type,
        }
    });
    (
        status,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body.to_string(),
    )
        .into_response()
}

/// 认证失败状态码 → OpenAI error type
fn auth_error_type(status: StatusCode) -> &'static str {
    match status {
        StatusCode::UNAUTHORIZED => "invalid_api_key",
        StatusCode::FORBIDDEN => "permission_denied",
        StatusCode::PAYMENT_REQUIRED => "insufficient_quota",
        _ => "server_error",
    }
}

/// 统一认证：提取 token → PG 查询，错误已映射为 OpenAI 格式
async fn authenticate_request(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<crate::identity::Pass, axum::response::Response> {
    let (status, message) = match crate::identity::extract_token(headers) {
        Ok(token) => match crate::identity::authenticate(&state.pool, &token).await {
            Ok(pass) => return Ok(pass),
            Err(e) => e,
        },
        Err(e) => e,
    };
    Err(error_response(status, &message, auth_error_type(status)))
}

/// GET /v1/models — 返回可用模型列表 (需要认证)
async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, axum::response::Response> {
    authenticate_request(&state, &headers).await?;

    let models = state.route_index.list_models();
    let data: Vec<serde_json::Value> = models
        .into_iter()
        .map(|id| serde_json::json!({"id": id, "object": "model"}))
        .collect();
    let body = serde_json::json!({"object": "list", "data": data});
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// POST /admin/reload — 从 PG 重新加载渠道配置 (仅 admin 组)
async fn reload_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    if pass.group != "admin" {
        return Err(error_response(
            StatusCode::FORBIDDEN,
            "admin group required",
            "permission_denied",
        ));
    }

    match load_channels(&state.pool).await {
        Ok(channels) => {
            let count = channels.len();
            state.route_index.build_from_channels(&channels);
            tracing::info!("reloaded {} channels", count);
            Ok((
                StatusCode::OK,
                [(
                    axum::http::header::CONTENT_TYPE,
                    "application/json".to_string(),
                )],
                serde_json::json!({"status": "ok", "channels": count}).to_string(),
            )
                .into_response())
        }
        Err(e) => {
            tracing::error!("failed to reload channels: {e}");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("reload failed: {e}"),
                "server_error",
            ))
        }
    }
}

/// POST /v1/chat/completions
///
/// 认证 → 解析 → 路由 → 限流 → 转发 (流式或非流式)
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    // 1. 认证
    let pass = authenticate_request(&state, &headers).await?;
    tracing::info!(user = %pass.username, "authenticated");

    // 2. 解析请求体
    let req: serde_json::Value = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;

    let model = req.get("model").and_then(|v| v.as_str()).ok_or_else(|| {
        error_response(
            StatusCode::BAD_REQUEST,
            "missing model field",
            "invalid_request_error",
        )
    })?;

    let is_stream = req.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);

    // 3. 查路由：model → channel
    let route = state.route_index.resolve(model).map_err(|e| {
        error_response(
            StatusCode::NOT_FOUND,
            &e.to_string(),
            "invalid_request_error",
        )
    })?;

    tracing::info!(
        channel = %route.channel_name,
        upstream = %route.upstream_model,
        stream = is_stream,
        "resolved route"
    );

    // 4. 限流（在路由解析之后，无效请求不消耗配额）
    crate::ratelimit::check_and_increment(&state.pool, &pass)
        .await
        .map_err(|(s, m)| error_response(s, &m, "rate_limit_reached"))?;

    // 5. 转发到上游
    if is_stream {
        let stream_resp = state
            .adapter
            .forward_stream(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        Ok(stream_into_response(stream_resp))
    } else {
        let resp = state
            .adapter
            .forward(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY);

        let mut response_headers = HeaderMap::new();
        if let Some(ct) = resp.content_type
            && let Ok(v) = ct.parse()
        {
            response_headers.insert(axum::http::header::CONTENT_TYPE, v);
        }

        Ok((status, response_headers, resp.body).into_response())
    }
}

/// 将 reqwest 流式响应转换为 axum Body 流
///
/// - 2xx 默认 text/event-stream；非 2xx 默认 application/json（上游错误体）
/// - 流中断时注入 SSE error frame，避免静默截断
fn stream_into_response(resp: StreamResponse) -> axum::response::Response {
    let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);

    let default_ct = if status.is_success() {
        "text/event-stream"
    } else {
        "application/json"
    };
    let content_type = resp
        .stream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| default_ct.to_string());

    // 流错误 → 注入 SSE error frame 而非静默断开
    let stream = resp.stream.bytes_stream().map(|result| -> Result<bytes::Bytes, std::io::Error> {
        match result {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                tracing::warn!("upstream stream error: {e}");
                const FRAME: &str = "data: {\"error\":{\"message\":\"upstream stream interrupted\",\"type\":\"server_error\"}}\n\n";
                Ok(bytes::Bytes::from_static(FRAME.as_bytes()))
            }
        }
    });

    let body = Body::from_stream(stream);
    (
        status,
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    )
        .into_response()
}

/// 从 PG kv_store 加载渠道配置
pub async fn load_channels(
    pool: &sqlx::PgPool,
) -> Result<Vec<crate::dispatch::ChannelConfig>, sqlx::Error> {
    let rows: Vec<(String, serde_json::Value)> =
        sqlx::query_as("SELECT key, value FROM kv_store WHERE key LIKE 'channel:%'")
            .fetch_all(pool)
            .await?;

    Ok(rows
        .into_iter()
        .filter_map(|(_, value)| serde_json::from_value(value).ok())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auth_error_type_maps_openai_types() {
        assert_eq!(auth_error_type(StatusCode::UNAUTHORIZED), "invalid_api_key");
        assert_eq!(auth_error_type(StatusCode::FORBIDDEN), "permission_denied");
        assert_eq!(
            auth_error_type(StatusCode::PAYMENT_REQUIRED),
            "insufficient_quota"
        );
        assert_eq!(
            auth_error_type(StatusCode::INTERNAL_SERVER_ERROR),
            "server_error"
        );
    }

    #[tokio::test]
    async fn error_response_is_json_with_openai_shape() {
        let resp = error_response(StatusCode::BAD_REQUEST, "bad", "invalid_request_error");
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            resp.headers().get(axum::http::header::CONTENT_TYPE).unwrap(),
            "application/json"
        );
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["error"]["message"], "bad");
        assert_eq!(v["error"]["type"], "invalid_request_error");
    }
}
