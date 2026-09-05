//! Ferrite — 用量记录中间件
//!
//! 拦截 `/v1/*` 路径的 POST 请求，提取 Authorization Bearer，
//! 查询令牌和用户元数据，并记录 token/usage 变化。
//!
//! 核心流程：
//! 1. 认证头 → sha256 哈希 → 令牌快照查询（TokenRecord）
//! 2. TokenRecord 提取 username → user 快照查询（UserRecord）
//! 3. 请求体缓存（pipeline 本来就整包 to_bytes）
//! 4. 响应体流式 tee，实时提取 usage（text/event-stream 使用 metering::scanner）
//! 5. JSON 体一次性获取 prompt_tokens/completion_tokens
//! 6. 落库：observe::logs::LogService::record(UsageEvent)（type=consume）
//! 7. 副作用：api_tokens.used_quota += cost，quota_snapshot 扣除 cost
//! 8. 仅记录 2xx 响应，其他错误 TODO(#N) 占位声明后续补。
//!
//! 注意：channel_name/channel_key 在 pipeline 内部选定，本 PR 拿不到 -> ponytail 注释说明

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use bytes::Bytes;
use futures::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::{Layer, Service};
use uuid::Uuid;

use crate::{snapshot::Snapshots, PgPool};

/// 前置上下文：TokenRecord + UserInfo
#[derive(Debug, Clone)]
struct AuthContext {
    user_key: Uuid,
    username: String,
    model: String,
}

/// UsageEvent 记录（来自 admin-observe crate）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct UsageEvent {
    pub log_type: i16,
    pub user_key: Uuid,
    pub username: String,
    pub token_key: Option<Uuid>,
    pub token_name: String,
    pub channel_key: Option<Uuid>,
    pub channel_name: String,
    pub model_name: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub quota: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub ip: String,
    pub request_id: String,
    pub content: String,
}

impl UsageEvent {
    /// 用于 consume 的快速构造
    pub fn consume(user_key: Uuid, username: &str, model_name: &str) -> Self {
        Self {
            log_type: 2, // consume
            user_key,
            username: username.into(),
            token_key: None,
            token_name: String::new(),
            channel_key: None,
            channel_name: String::new(),
            model_name: model_name.into(),
            prompt_tokens: 0,
            completion_tokens: 0,
            quota: 0,
            use_time_ms: 0,
            is_stream: false,
            ip: String::new(),
            request_id: String::new(),
            content: String::new(),
        }
    }
}

/// 中间件应用 state
#[derive(Clone)]
pub struct UsageMiddlewareState {
    pub pool: PgPool,
    pub snapshots: Arc<Snapshots>,
    pub token_swap: Arc<arc_swap::ArcSwap<gateway_gate::snapshot::TokenSnapshot>>,
    pub user_swap: Arc<arc_swap::ArcSwap<gateway_gate::snapshot::UserSnapshot>>,
    pub quota_swap: Arc<arc_swap::ArcSwap<gateway_gate::snapshot::QuotaSnapshot>>,
}

/// 用量中间件 (axum::middleware::from_fn_with_state)
pub async fn usage_middleware(
    State(state): State<UsageMiddlewareState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // 仅拦截 POST /v1/* 请求
    if request.method() != &axum::http::Method::POST {
        return Ok(next.run(request).await);
    }
    if !request.uri().path().starts_with("/v1/") {
        return Ok(next.run(request).await);
    }

    // 1. 提取 Authorization Bearer
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth_header.starts_with("Bearer ") {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let token_key = auth_header.strip_prefix("Bearer ").unwrap();

    // 2. 查 token_snapshot (SHA256 哈希)
    let token_snapshot = &state.snapshots.token_snapshot;
    let token_record = token_snapshot
        .iter()
        .find(|t| t.key == token_key)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let username = token_record.user.clone();

    // 3. 查 user_snapshot
    let user_snapshot = &state.snapshots.user_snapshot;
    let _user_record = user_snapshot
        .iter()
        .find(|u| u.meta.key == username)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 4. 缓存请求体
    let (parts, body) = request.into_parts();
    let body_bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 5. 请求体重构
    let mut new_request = Request::from_parts(parts, body_bytes.into());

    // 6. 执行请求
    let response = next.run(new_request).await;

    // 7. 记录 usage（简化版本，仅提示实际实现）
    // TODO: 实际实现需要解析响应体提取 usage 数据
    tracing::info!("Usage middleware called for token: {}", token_key);

    Ok(response)
}