//! HTTP 路由 + handler
//!
//! ponytail: `Err = axum::response::Response`（128 bytes）是 axum 的惯用错误载荷 —— handler 里
//! `Err(...)` 直接就是要返回给客户端的响应。按 clippy 建议 box 掉会破坏 `IntoResponse` 契约，
//! 且要改 14 处签名和全部 `?` 传播点，换不到任何实际收益。
#![allow(clippy::result_large_err)] // ponytail: axum Response 就是错误载荷，box 掉会破坏 IntoResponse

use std::sync::Arc;

type UsageRow = (
    String,
    i64,
    String,
    i64,
    i64,
    String,
    bool,
    chrono::DateTime<chrono::Utc>,
    bool,
);

use axum::Router;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post, put};
use futures_util::StreamExt;

use crate::adapter::{OpenAIAdapter, StreamResponse};
use crate::config::PgPool;
use crate::dispatch::RouteIndex;

/// 日志文件目录（main.rs tracing_appender 与 list_logs 共用）
pub const LOG_DIR: &str = "logs";

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
        use tower_http::trace::{DefaultOnResponse, TraceLayer};

        Router::new()
            .route("/health", get(health))
            .route("/v1/models", get(list_models))
            .route("/v1/chat/completions", post(chat_completions))
            .route("/admin/logs", get(list_logs))
            .route("/admin/reload", post(reload_channels))
            .route("/admin/tokens", get(list_tokens).post(create_token))
            .route("/admin/tokens/{key}", delete(delete_token))
            .route("/admin/channels", get(list_channels).post(create_channel))
            .route(
                "/admin/channels/{id}",
                put(update_channel).delete(delete_channel),
            )
            .route("/admin/recharge", post(recharge))
            .layer(
                // 统一请求日志：span 声明业务字段，handler 内 record 填充，
                // TraceLayer 的 on_response 事件自动携带全部字段写入 JSON 日志文件
                TraceLayer::new_for_http()
                    .make_span_with(|req: &axum::http::Request<axum::body::Body>| {
                        tracing::info_span!(
                            "http_request",
                            method = %req.method(),
                            path = %req.uri().path(),
                            request_id = tracing::field::Empty,
                            user = tracing::field::Empty,
                            model = tracing::field::Empty,
                            channel = tracing::field::Empty,
                            upstream_model = tracing::field::Empty,
                            stream = tracing::field::Empty,
                            prompt_tokens = tracing::field::Empty,
                            completion_tokens = tracing::field::Empty,
                           reserved_quota = tracing::field::Empty,
                           actual_quota = tracing::field::Empty,
                        )
                    })
                    .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
            )
            .with_state(self.state.clone())
    }
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

/// 构造 OpenAI 标准错误响应（application/json）
pub fn error_response(
    status: StatusCode,
    message: &str,
    error_type: &str,
) -> axum::response::Response {
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
pub fn auth_error_type(status: StatusCode) -> &'static str {
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
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

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
        Err(e) => Err(error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("reload failed: {e}"),
            "server_error",
        )),
    }
}

/// F6.1 — POST /admin/tokens 创建请求
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateTokenReq {
    pub user_id: Option<i64>,
    pub username: String,
    pub quota: Option<i64>,
    pub group: Option<String>,
    pub is_admin: Option<bool>,
}

/// 生成随机 token key：`sk-` + 32 hex 字符
pub fn gen_token_key() -> String {
    let bytes: [u8; 16] = rand::random();
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("sk-{hex}")
}

/// POST /admin/tokens — 创建 token (仅 admin)
async fn create_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let req: CreateTokenReq = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;
    if req.username.trim().is_empty() {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "username must not be empty",
            "invalid_request_error",
        ));
    }

    // user_id 缺省时取 max+1（admin 低频操作，不做并发防护）
    // ponytail: max+1 有竞态，token 创建频率低到可忽略；真要并发就换 sequence
    let user_id = match req.user_id {
        Some(id) => id,
        None => {
            let (next,): (i64,) =
                sqlx::query_as("SELECT COALESCE(MAX(user_id), 0) + 1 FROM tokens")
                    .fetch_one(&state.pool)
                    .await
                    .map_err(|e| {
                        error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            &format!("db error: {e}"),
                            "server_error",
                        )
                    })?;
            next
        }
    };

    let key = gen_token_key();
    let quota = req.quota.unwrap_or(500_000);
    let group = req.group.unwrap_or_else(|| "default".into());
    let is_admin = req.is_admin.unwrap_or(false);

    let row: (i64, String, bool) = sqlx::query_as(
        r#"INSERT INTO tokens (key, user_id, username, quota, "group", enabled, is_admin)
           VALUES ($1, $2, $3, $4, $5, true, $6)
           RETURNING user_id, "group", is_admin"#,
    )
    .bind(&key)
    .bind(user_id)
    .bind(req.username.trim())
    .bind(quota)
    .bind(&group)
    .bind(is_admin)
    .fetch_one(&state.pool)
    .await
    .map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("db error: {e}"),
            "server_error",
        )
    })?;

    let body = serde_json::json!({
        "object": "token",
        "key": key,
        "user_id": row.0,
        "username": req.username.trim(),
        "quota": quota,
        "used_quota": 0,
        "group": row.1,
        "enabled": true,
        "is_admin": row.2,
    });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// F7.1 — POST /admin/channels 创建请求
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateChannelReq {
    pub name: String,
    pub base_url: String,
    pub channel_type: String,
    pub keys: Vec<String>,
    pub models: Vec<crate::dispatch::ModelRoute>,
}

/// 允许的 channel_type 取值
pub const CHANNEL_TYPES: &[&str] = &["openai", "openai-compat", "claude", "gemini"];

/// 纯校验（可单测）：字段非空 + channel_type 合法 + base_url 可解析
pub fn validate_channel(req: &CreateChannelReq) -> Result<(), String> {
    if req.name.trim().is_empty() {
        return Err("name must not be empty".into());
    }
    if !CHANNEL_TYPES.contains(&req.channel_type.as_str()) {
        return Err(format!(
            "invalid channel_type {:?}; allowed one of {:?}",
            req.channel_type, CHANNEL_TYPES
        ));
    }
    if reqwest::Url::parse(&req.base_url).is_err() {
        return Err(format!("invalid base_url: {}", req.base_url));
    }
    if req.keys.is_empty() || req.keys.iter().any(|k| k.trim().is_empty()) {
        return Err("keys must be a non-empty array of non-empty strings".into());
    }
    if req.models.is_empty() {
        return Err("models must not be empty".into());
    }
    for m in &req.models {
        if m.alias.trim().is_empty() || m.upstream.trim().is_empty() {
            return Err("models[].alias/upstream must not be empty".into());
        }
    }
    Ok(())
}

/// POST /admin/channels — 创建渠道 (仅 admin)，存 kv_store `channel:{id}`
async fn create_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let req: CreateChannelReq = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;
    validate_channel(&req)
        .map_err(|m| error_response(StatusCode::BAD_REQUEST, &m, "invalid_request_error"))?;

    // name 唯一性：与现有渠道比对
    let existing = load_channels(&state.pool).await.map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("db error: {e}"),
            "server_error",
        )
    })?;
    if existing.iter().any(|c| c.name == req.name) {
        return Err(error_response(
            StatusCode::CONFLICT,
            &format!("channel name {:?} already exists", req.name),
            "invalid_request_error",
        ));
    }

    // id 用毫秒时间戳（kv_store key 是 text，无自增序列）
    let id = chrono::Utc::now().timestamp_millis();
    let cfg = crate::dispatch::ChannelConfig {
        id,
        name: req.name.clone(),
        base_url: req.base_url.clone(),
        channel_type: req.channel_type.clone(),
        keys: req.keys.clone(),
        models: req.models.clone(),
    };
    let value = serde_json::to_value(&cfg).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("serialize error: {e}"),
            "server_error",
        )
    })?;
    sqlx::query("INSERT INTO kv_store (key, value) VALUES ($1, $2)")
        .bind(format!("channel:{id}"))
        .bind(value)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("db error: {e}"),
                "server_error",
            )
        })?;

    let body = serde_json::json!({
        "status": "ok",
        "id": id,
        "name": req.name,
        "hint": "POST /admin/reload to apply",
    });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}
// ─── F6.2 + F6.3: Token 管理 ──────────────────────────────────────────────

/// F6.2 — GET /admin/tokens（列表）
#[derive(serde::Deserialize)]
pub struct TokenListQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub user_id: Option<i64>,
    pub enabled: Option<bool>,
}

/// key 掩码：前 8 位 + ...
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        // 短 key 不完整显示，防泄露上游短密钥
        "***".into()
    } else {
        format!("{}...", &key[..8])
    }
}

/// GET /admin/tokens — 列出 token (仅 admin)
async fn list_tokens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<TokenListQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let limit = q.limit.unwrap_or(50).clamp(1, 500) as i64;
    let offset = q.offset.unwrap_or(0) as i64;

    // 用「超集 → 内存过滤 → 切页」代替动态 SQL，简化参数绑定。token 表 <1000 行，内存过滤完全够用
    let rows: Vec<UsageRow> = sqlx::query_as(
        "SELECT key, user_id, username, quota, used_quota, \"group\", enabled, created_at, is_admin FROM tokens ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|e| error_response(StatusCode::INTERNAL_SERVER_ERROR, &format!("db error: {e}"), "server_error"))?;

    let filtered: Vec<_> = rows
        .into_iter()
        .filter(|r| q.user_id.is_none_or(|u| r.1 == u))
        .filter(|r| q.enabled.is_none_or(|e| r.6 == e))
        .collect();

    let total = filtered.len();
    let data: Vec<serde_json::Value> = filtered
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .map(
            |(key, user_id, username, quota, used_quota, group, enabled, created_at, is_admin)| {
                serde_json::json!({
                    "key": mask_key(&key),
                    "user_id": user_id,
                    "username": username,
                    "quota": quota,
                    "used_quota": used_quota,
                    "group": group,
                    "enabled": enabled,
                    "created_at": created_at,
                    "is_admin": is_admin,
                })
            },
        )
        .collect();

    let body = serde_json::json!({ "object": "list", "total": total, "data": data });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// F6.3 — DELETE /admin/tokens/:key（软删除）
async fn delete_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let result = sqlx::query("UPDATE tokens SET enabled = false WHERE key = $1")
        .bind(&key)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("db error: {e}"),
                "server_error",
            )
        })?;

    if result.rows_affected() == 0 {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "token not found",
            "invalid_request_error",
        ));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
}

/// F10.4 — POST /admin/recharge（充电，仅 admin）
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RechargeReq {
    pub token_key: String,
    pub amount: i64,
}

async fn recharge(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let req: RechargeReq = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;

    if req.amount <= 0 {
        return Err(error_response(
            StatusCode::BAD_REQUEST,
            "amount must be positive",
            "invalid_request_error",
        ));
    }

    let row: Option<(i64, i64, String)> = sqlx::query_as(
        "UPDATE tokens SET quota = quota + $1 WHERE key = $2 RETURNING quota, user_id, username",
    )
    .bind(req.amount)
    .bind(&req.token_key)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("db error: {e}"),
            "server_error",
        )
    })?;

    let (new_quota, user_id, username) = row.ok_or_else(|| {
        error_response(
            StatusCode::NOT_FOUND,
            "token not found",
            "invalid_request_error",
        )
    })?;

    let body = serde_json::json!({
        "status": "ok",
        "token_key": mask_key(&req.token_key),
        "new_quota": new_quota,
        "user_id": user_id,
        "username": username,
    });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

// ─── F7.2 + F7.3: Channel 管理 ─────────────────────────────────────────────

/// F7.2 — GET /admin/channels（列表）
#[derive(serde::Deserialize)]
pub struct ChannelListQuery {
    pub channel_type: Option<String>,
}

/// 渠道 key 掩码（每个 key 都掩码）
pub fn mask_channel_keys(keys: &[String]) -> Vec<String> {
    keys.iter().map(|k| mask_key(k)).collect()
}

/// GET /admin/channels — 列出所有渠道 (仅 admin)
async fn list_channels(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ChannelListQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let rows = load_channels(&state.pool).await.map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("db error: {e}"),
            "server_error",
        )
    })?;

    let filtered: Vec<&crate::dispatch::ChannelConfig> = rows
        .iter()
        .filter(|c| {
            q.channel_type
                .as_deref()
                .is_none_or(|t| c.channel_type == t)
        })
        .collect();

    let data: Vec<serde_json::Value> = filtered
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.id,
                "name": c.name,
                "base_url": c.base_url,
                "channel_type": c.channel_type,
                "keys": mask_channel_keys(&c.keys),
                "models": c.models,
            })
        })
        .collect();

    let body = serde_json::json!({ "object": "list", "total": filtered.len(), "data": data });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// F7.3 — PUT /admin/channels/:id 更新请求（字段全 Option，传什么改什么）
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateChannelReq {
    pub name: Option<String>,
    pub base_url: Option<String>,
    pub channel_type: Option<String>,
    pub keys: Option<Vec<String>>,
    pub models: Option<Vec<crate::dispatch::ModelRoute>>,
}

/// 纯函数：将部分更新合并进现有 config，返回新版本 + 是否变化
pub fn merge_channel_config(
    existing: &crate::dispatch::ChannelConfig,
    req: &UpdateChannelReq,
) -> (crate::dispatch::ChannelConfig, bool) {
    let mut cfg = existing.clone();
    let mut changed = false;
    if let Some(v) = &req.name
        && *v != cfg.name
    {
        cfg.name = v.clone();
        changed = true;
    }
    if let Some(v) = &req.base_url
        && *v != cfg.base_url
    {
        cfg.base_url = v.clone();
        changed = true;
    }
    if let Some(v) = &req.channel_type
        && *v != cfg.channel_type
    {
        cfg.channel_type = v.clone();
        changed = true;
    }
    if let Some(v) = &req.keys
        && *v != cfg.keys
    {
        cfg.keys = v.clone();
        changed = true;
    }
    if let Some(v) = &req.models
        && *v != cfg.models
    {
        cfg.models = v.clone();
        changed = true;
    }
    (cfg, changed)
}

/// 把 ChannelConfig 合成 CreateChannelReq 以便复用 validate_channel
fn channel_as_create_req(cfg: &crate::dispatch::ChannelConfig) -> CreateChannelReq {
    CreateChannelReq {
        name: cfg.name.clone(),
        base_url: cfg.base_url.clone(),
        channel_type: cfg.channel_type.clone(),
        keys: cfg.keys.clone(),
        models: cfg.models.clone(),
    }
}

/// F7.3 — PUT /admin/channels/:id（更新）
async fn update_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
    body: axum::body::Bytes,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let req: UpdateChannelReq = serde_json::from_slice(&body).map_err(|e| {
        error_response(
            StatusCode::BAD_REQUEST,
            &format!("invalid JSON: {e}"),
            "invalid_request_error",
        )
    })?;

    let kv_key = format!("channel:{id}");
    let row: Option<(serde_json::Value,)> =
        sqlx::query_as("SELECT value FROM kv_store WHERE key = $1")
            .bind(&kv_key)
            .fetch_optional(&state.pool)
            .await
            .map_err(|e| {
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("db error: {e}"),
                    "server_error",
                )
            })?;

    let value = row
        .ok_or_else(|| {
            error_response(
                StatusCode::NOT_FOUND,
                "channel not found",
                "invalid_request_error",
            )
        })?
        .0;

    let existing: crate::dispatch::ChannelConfig = serde_json::from_value(value).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("corrupt channel config: {e}"),
            "server_error",
        )
    })?;

    let (merged, _changed) = merge_channel_config(&existing, &req);

    // 更新后 name 可能变了，需要重新查重（排除自己）
    if merged.name != existing.name {
        let all = load_channels(&state.pool).await.map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("db error: {e}"),
                "server_error",
            )
        })?;
        if all.iter().any(|c| c.id != id && c.name == merged.name) {
            return Err(error_response(
                StatusCode::CONFLICT,
                &format!("channel name {:?} already exists", merged.name),
                "invalid_request_error",
            ));
        }
    }

    // 重新校验合并后的整体（哪怕 only name 改了也要验证 base_url/models 一致性）
    validate_channel(&channel_as_create_req(&merged))
        .map_err(|m| error_response(StatusCode::BAD_REQUEST, &m, "invalid_request_error"))?;

    let new_value = serde_json::to_value(&merged).map_err(|e| {
        error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("serialize error: {e}"),
            "server_error",
        )
    })?;

    sqlx::query("UPDATE kv_store SET value = $2 WHERE key = $1")
        .bind(&kv_key)
        .bind(new_value)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("db error: {e}"),
                "server_error",
            )
        })?;

    let body = serde_json::json!({
        "status": "ok",
        "id": id,
        "name": merged.name,
        "hint": "POST /admin/reload to apply",
    });
    Ok((
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response())
}

/// F7.3 — DELETE /admin/channels/:id（删除）
async fn delete_channel(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    let kv_key = format!("channel:{id}");
    let result = sqlx::query("DELETE FROM kv_store WHERE key = $1")
        .bind(&kv_key)
        .execute(&state.pool)
        .await
        .map_err(|e| {
            error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("db error: {e}"),
                "server_error",
            )
        })?;

    if result.rows_affected() == 0 {
        return Err(error_response(
            StatusCode::NOT_FOUND,
            "channel not found",
            "invalid_request_error",
        ));
    }

    Ok(StatusCode::NO_CONTENT.into_response())
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
    let span = tracing::Span::current();
    span.record(
        "request_id",
        chrono::Utc::now()
            .timestamp_nanos_opt()
            .unwrap_or(0)
            .to_string(),
    );
    span.record("user", pass.username.as_str());

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
    let route = state.route_index.resolve(model).map_err(|e| {
        error_response(
            StatusCode::NOT_FOUND,
            &e.to_string(),
            "invalid_request_error",
        )
    })?;

    span.record("model", model);
    span.record("channel", route.channel_name.as_ref());
    span.record("upstream_model", route.upstream_model.as_ref());
    span.record("stream", is_stream);

    // 4. 限流（在路由解析之后，无效请求不消耗配额）
    crate::ratelimit::check_and_increment(&state.pool, &pass)
        .await
        .map_err(|(s, m)| error_response(s, &m, "rate_limit_reached"))?;

    // 4b. F10.2 预扣配额
    let pricing = crate::billing::read_pricing(&state.pool, model)
        .await
        .unwrap_or_else(|e| {
            tracing::warn!(error = %e, model, "read_pricing failed, using 1:1");
            None
        });
    let reserve = crate::billing::RESERVE_QUOTA;
    let mut reserved = false;
    match crate::billing::reserve_quota(&state.pool, &pass.token_key, reserve).await {
        Ok(Some(used)) => {
            reserved = true;
            span.record("reserved_quota", reserve);
            tracing::debug!(used_quota = used, "quota reserved");
        }
        Ok(None) => {
            return Err(error_response(
                StatusCode::PAYMENT_REQUIRED,
                "insufficient quota",
                "insufficient_quota",
            ));
        }
        Err(e) => {
            tracing::warn!(error = %e, "reserve_quota failed, letting request through");
        }
    }

    // 5. 转发到上游
    if is_stream {
        // ponytail: 流式无 usage，结算随 F8 单独处理
        let stream_resp = state
            .adapter
            .forward_stream(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                tracing::warn!(event = "billing_reserve_consumed", reserve, error = %e, "upstream error, reserve not rolled back");
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        Ok(stream_into_response(stream_resp).await?)
    } else {
        let resp = state
            .adapter
            .forward(body, &route.base_url, &route.api_key, &route.upstream_model)
            .await
            .map_err(|e| {
                tracing::warn!(event = "billing_reserve_consumed", reserve, error = %e, "upstream error, reserve not rolled back");
                error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "server_error")
            })?;

        let status = StatusCode::from_u16(resp.status).unwrap_or(StatusCode::BAD_GATEWAY);

        // F10.3 结算：仅在上游 2xx 且预扣成功时执行；非 2xx / reserve 失败都不退款（决策 C）
        if status.is_success()
            && reserved
            && let Ok(v) = serde_json::from_slice::<serde_json::Value>(&resp.body)
            && let Some(u) = v.get("usage")
<<<<<<< Updated upstream
        {
            let prompt_tokens = u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
            let completion_tokens = u
                .get("completion_tokens")
                .and_then(|x| x.as_u64())
                .unwrap_or(0);
            span.record("prompt_tokens", prompt_tokens as i64);
            span.record("completion_tokens", completion_tokens as i64);
            if let Some(n) = u.get("total_tokens").and_then(|x| x.as_i64()) {
                span.record("total_tokens", n);
            }
=======
            {
                let prompt_tokens = u.get("prompt_tokens").and_then(|x| x.as_u64()).unwrap_or(0);
                let completion_tokens = u
                    .get("completion_tokens")
                    .and_then(|x| x.as_u64())
                    .unwrap_or(0);
                span.record("prompt_tokens", prompt_tokens as i64);
                span.record("completion_tokens", completion_tokens as i64);
                if let Some(n) = u.get("total_tokens").and_then(|x| x.as_i64()) {
                    span.record("total_tokens", n);
                }
>>>>>>> Stashed changes

            let actual =
                crate::billing::tokens_to_quota(prompt_tokens, completion_tokens, pricing.as_ref());
            span.record("actual_quota", actual);
            if let Err(e) =
                crate::billing::settle_quota(&state.pool, &pass.token_key, reserve, actual).await
            {
                tracing::warn!(event = "billing_settle_failed", error = %e, actual, reserve, "settle_quota failed");
            }
        }

        // 上游非 2xx → 预扣消耗，不回滚（决策 C）
        if !status.is_success() {
            tracing::warn!(
                event = "billing_reserve_consumed",
                reserve,
                status = status.as_u16(),
                "upstream non-2xx, reserve not rolled back"
            );
        }

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
async fn stream_into_response(
    resp: StreamResponse,
) -> Result<axum::response::Response, axum::response::Response> {
    let resp = crate::adapter::ensure_stream_ok(resp)
        .await
        .map_err(|e| error_response(StatusCode::BAD_GATEWAY, &e.to_string(), "upstream_error"))?;

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
    Ok((
        status,
        [
            (axum::http::header::CONTENT_TYPE, content_type),
            (axum::http::header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    )
        .into_response())
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

/// F5.3 — GET /admin/logs 查询参数
#[derive(serde::Deserialize)]
pub struct LogQuery {
    pub user: Option<String>,
    pub model: Option<String>,
    pub channel: Option<String>,
    pub path: Option<String>,
    pub status: Option<u16>,
    pub since: Option<String>,
    pub until: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// 展平 JSONL 日志行：合并 fields > span > spans[0] 为一个 flat map。
/// 实现按优先级从低到高依次插入（spans[0] → span → fields），后者覆盖前者。
fn flatten(v: &serde_json::Value) -> Option<serde_json::Value> {
    let obj = v.as_object()?;
    let mut merged = serde_json::Map::new();
    if let Some(arr) = obj.get("spans").and_then(|s| s.as_array())
        && let Some(first) = arr.first().and_then(|s| s.as_object())
    {
        for (k, val) in first {
            merged.insert(k.clone(), val.clone());
        }
    }
    if let Some(span) = obj.get("span").and_then(|s| s.as_object()) {
        for (k, val) in span {
            merged.insert(k.clone(), val.clone());
        }
    }
    if let Some(fields) = obj.get("fields").and_then(|s| s.as_object()) {
        for (k, val) in fields {
            merged.insert(k.clone(), val.clone());
        }
    }
    Some(serde_json::Value::Object(merged))
}

/// 纯函数：过滤 JSONL 行并分页
///
/// - 只保留"请求完成事件"行（顶层有 fields.status 的行）
/// - 展平 fields > span > spans[0]
/// - 字符串字段精确匹配；status 数值匹配；since/until 与 timestamp 做前缀比较
/// - 保持传入顺序；返回 (当前页数据, 匹配总数)
pub fn filter_log_lines(lines: &[&str], q: &LogQuery) -> (Vec<serde_json::Value>, usize) {
    let limit = q.limit.unwrap_or(50).clamp(1, 500);
    let offset = q.offset.unwrap_or(0);

    let mut matched: Vec<serde_json::Value> = Vec::new();
    for line in lines {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue; // 损坏行静默跳过
        };
        // 只保留请求完成事件行（顶层有 fields.status）
        if v.get("fields").and_then(|f| f.get("status")).is_none() {
            continue;
        }
        let Some(flat) = flatten(&v) else { continue };

        // 字符串字段精确匹配
        let str_match = |field: &str, val: &Option<String>| -> bool {
            val.as_ref().is_none_or(|want| {
                flat.get(field)
                    .and_then(|f| f.as_str())
                    .is_some_and(|s| s == want)
            })
        };
        if !str_match("user", &q.user)
            || !str_match("model", &q.model)
            || !str_match("channel", &q.channel)
            || !str_match("path", &q.path)
        {
            continue;
        }

        // status 数值匹配
        if let Some(status) = q.status
            && flat.get("status").and_then(|s| s.as_u64()) != Some(status as u64)
        {
            continue;
        }

        // since/until 与顶层 timestamp 比较（RFC3339 字典序 = 时间序，要求同精度格式）。
        // until 用"相等或前缀包含"语义：until="2025-01-01" 表示含当天全天。
        if let Some(ts) = v.get("timestamp").and_then(|t| t.as_str()) {
            if let Some(since) = &q.since
                && ts < since.as_str()
            {
                continue;
            }
            if let Some(until) = &q.until
<<<<<<< Updated upstream
                && ts > until.as_str()
                && !ts.starts_with(until.as_str())
=======
                && ts > until.as_str() && !ts.starts_with(until.as_str())
>>>>>>> Stashed changes
            {
                continue;
            }
        }

        matched.push(flat);
    }

    let total = matched.len();
    let page = matched.into_iter().skip(offset).take(limit).collect();
    (page, total)
}

/// GET /admin/logs — 查询 JSONL 日志（仅 admin）
async fn list_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<LogQuery>,
) -> Result<axum::response::Response, axum::response::Response> {
    let pass = authenticate_request(&state, &headers).await?;
    crate::identity::require_admin(&pass)
        .map_err(|(s, m)| error_response(s, &m, "permission_denied"))?;

    // 收集日志行：文件名降序（新→旧），逐文件按行收集。
    // 内存上限：最多读 MAX_SCAN_LINES 行（新文件优先），防止长期运行后 OOM。
    // since/until 可解析出日期前缀时，跳过窗口外的旧文件（文件名含 YYYY-MM-DD）。
    const MAX_SCAN_LINES: usize = 100_000;
    let since_date = q.since.as_deref().map(|s| s.get(..10).unwrap_or(s));
    let until_date = q.until.as_deref().map(|s| s.get(..10).unwrap_or(s));

    let mut all_lines: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(LOG_DIR) {
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| name.starts_with("ferrite.log."))
            .collect();
        names.sort_unstable_by(|a, b| b.cmp(a)); // 降序
        'files: for name in names {
            // 文件名日期在 since 窗口之前 → 后面只会更旧，直接停
            if let Some(d) = name.get(12..22) {
                if let Some(sd) = since_date
                    && d < sd
                {
                    break;
                }
                if let Some(ud) = until_date
                    && d > ud
                {
                    continue;
                }
            }
            let Ok(content) = std::fs::read_to_string(std::path::Path::new(LOG_DIR).join(&name))
            else {
                continue;
            };
            for line in content.lines() {
                all_lines.push(line.to_string());
                if all_lines.len() >= MAX_SCAN_LINES {
                    break 'files;
                }
            }
        }
    }
    // 目录不存在或不可读 → 空结果，不报 500
    let refs: Vec<&str> = all_lines.iter().map(|s| s.as_str()).collect();
    let (data, total) = filter_log_lines(&refs, &q);

    let body = serde_json::json!({"object": "list", "total": total, "data": data});
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "application/json".to_string(),
        )],
        body.to_string(),
    )
        .into_response())
}
