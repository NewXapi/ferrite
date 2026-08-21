//! HTTP 路由 + handler

use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};

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

/// 构造 OpenAI 标准错误响应
fn error_response(status: StatusCode, message: &str, error_type: &str) -> (StatusCode, String) {
    let body = serde_json::json!({
        "error": {
            "message": message,
            "type": error_type,
        }
    });
    (status, body.to_string())
}

/// GET /v1/models — 返回可用模型列表 (需要认证)
async fn list_models(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let token = crate::identity::extract_token(&headers)
        .map_err(|(s, m)| error_response(s, &m, "invalid_request_error"))?;
    crate::identity::authenticate(&state.pool, &token)
        .await
        .map_err(|(s, m)| error_response(s, &m, "invalid_request_error"))?;

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
    ))
}

/// POST /admin/reload — 从 PG 重新加载渠道配置 (需要认证)
async fn reload_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let token = crate::identity::extract_token(&headers)
        .map_err(|(s, m)| error_response(s, &m, "invalid_request_error"))?;
    crate::identity::authenticate(&state.pool, &token)
        .await
        .map_err(|(s, m)| error_response(s, &m, "invalid_request_error"))?;

    match load_channels(&state.pool).await {
        Ok(channels) => {
            let count = channels.len();
            state.route_index.build_from_channels(&channels);
            tracing::info!("reloaded {} channels", count);
            Ok((
                StatusCode::OK,
                serde_json::json!({"status": "ok", "channels": count}).to_string(),
            ))
        }
        Err(e) => {
            tracing::error!("failed to reload channels: {e}");
            Err(error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("reload failed: {e}"),
                "internal_error",
            ))
        }
    }
}

/// POST /v1/chat/completions
///
/// 认证 → 限流 → 路由 → 转发 (流式或非流式)
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. 认证
    let token = crate::identity::extract_token(&headers)
        .map_err(|(s, m)| error_response(s, &m, "invalid_request_error"))?;
    let pass = crate::identity::authenticate(&state.pool, &token)
        .await
        .map_err(|(s, m)| {
            error_response(
                s,
                &m,
                if s == StatusCode::UNAUTHORIZED {
                    "invalid_request_error"
                } else {
                    "insufficient_quota"
                },
            )
        })?;
    tracing::info!(user = %pass.username, "authenticated");

    // 2. 限流
    crate::ratelimit::check_and_increment(&state.pool, &pass)
        .await
        .map_err(|(s, m)| error_response(s, &m, "rate_limit_exceeded"))?;

    // 3. 解析请求体
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

    // 4. 查路由：model → channel
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

    // 5. 转发到上游
    if is_stream {
        let stream_resp = state
            .adapter
            .forward_stream(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "upstream_error")
            })?;

        Ok(stream_into_response(stream_resp))
    } else {
        let resp = state
            .adapter
            .forward(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "upstream_error")
            })?;

        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY);

        let mut response_headers = HeaderMap::new();
        if let Some(ct) = resp.content_type
            && let Ok(v) = ct.parse()
        {
            response_headers.insert("content-type", v);
        }

        Ok((status, response_headers, resp.body).into_response())
    }
}

/// 将 reqwest 流式响应转换为 axum Body 流
fn stream_into_response(resp: StreamResponse) -> axum::response::Response {
    let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::OK);

    // 上游非 200 时可能是 JSON 错误，透传 content-type
    let content_type = resp
        .stream
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "text/event-stream".to_string());

    let stream = resp.stream.bytes_stream();
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
