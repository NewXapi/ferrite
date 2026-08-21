//! HTTP 路由 + handler

use std::sync::Arc;

use axum::Router;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};

use crate::adapter::OpenAIAdapter;
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
            .route("/v1/chat/completions", post(chat_completions))
            .with_state(self.state.clone())
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// POST /v1/chat/completions
///
/// 认证 → 限流 → 路由 → 转发
async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // 1. 认证
    let token = crate::identity::extract_token(&headers)?;
    let pass = crate::identity::authenticate(&state.pool, &token).await?;
    tracing::info!(user = %pass.username, "authenticated");

    // 2. 限流
    crate::ratelimit::check_and_increment(&state.pool, &pass).await?;

    // 3. 解析 model
    let req: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;

    let model = req
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or((StatusCode::BAD_REQUEST, "missing model field".to_string()))?;

    // 4. 查路由：model → channel
    let route = state
        .route_index
        .resolve(model)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    tracing::info!(
        channel = %route.channel_name,
        upstream = %route.upstream_model,
        "resolved route"
    );

    // 5. 转发到上游
    let resp = state
        .adapter
        .forward(body, &route.base_url, &route.api_key, &route.upstream_model)
        .await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY);

    let mut response_headers = HeaderMap::new();
    if let Some(ct) = resp.content_type
        && let Ok(v) = ct.parse()
    {
        response_headers.insert("content-type", v);
    }

    Ok((status, response_headers, resp.body))
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
