//! 用量日志 + dashboard — 单机版平表实现 (usage_logs)。
//!
//! 替代原 hourly/perf/rankings 骨架 (聚合优化推迟，先落原始表 + 查询)。
//! 网关侧后续调 `record()` 写消费记录; admin 面板走查询路由。
//!
//! log_type: 1=topup 2=consume 3=manage 4=system (对齐 new-api)。

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS usage_logs (
    id                BIGSERIAL PRIMARY KEY,
    log_type          SMALLINT NOT NULL DEFAULT 2,
    user_key          UUID NOT NULL,
    username          TEXT NOT NULL DEFAULT '',
    token_key         UUID,
    token_name        TEXT NOT NULL DEFAULT '',
    channel_key       UUID,
    channel_name      TEXT NOT NULL DEFAULT '',
    model_name        TEXT NOT NULL DEFAULT '',
    prompt_tokens     INT NOT NULL DEFAULT 0,
    completion_tokens INT NOT NULL DEFAULT 0,
    quota             BIGINT NOT NULL DEFAULT 0,
    use_time_ms       INT NOT NULL DEFAULT 0,
    is_stream         BOOLEAN NOT NULL DEFAULT false,
    ip                TEXT NOT NULL DEFAULT '',
    request_id        TEXT NOT NULL DEFAULT '',
    content           TEXT NOT NULL DEFAULT '',
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_usage_logs_created ON usage_logs(created_at);
CREATE INDEX IF NOT EXISTS idx_usage_logs_user ON usage_logs(user_key, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_logs_token_name ON usage_logs(token_name);
CREATE INDEX IF NOT EXISTS idx_usage_logs_model_name ON usage_logs(model_name);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageEvent {
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
    pub fn consume(user_key: Uuid, username: &str, model_name: &str) -> Self {
        Self {
            log_type: 2,
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

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogView {
    pub id: i64,
    pub log_type: i16,
    pub user_key: Uuid,
    pub username: String,
    pub token_name: String,
    pub channel_name: String,
    pub model_name: String,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
    pub quota: i64,
    pub use_time_ms: i32,
    pub is_stream: bool,
    pub ip: String,
    pub request_id: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UsageStat {
    pub quota: i64,
    pub requests: i64,
    /// 近 60s 请求数
    pub rpm: i64,
    /// 近 60s token 数
    pub tpm: i64,
}

pub struct LogService {
    pool: PgPool,
}

const COLS: &str = "id, log_type, user_key, username, token_name, channel_name, model_name, \
     prompt_tokens, completion_tokens, quota, use_time_ms, is_stream, ip, request_id, \
     created_at";

impl LogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 网关写入一条消费记录。
    pub async fn record(&self, e: &UsageEvent) -> Result<i64, AuthError> {
        let id: (i64,) = sqlx::query_as(
            r#"INSERT INTO usage_logs
               (log_type, user_key, username, token_key, token_name, channel_key,
                channel_name, model_name, prompt_tokens, completion_tokens, quota,
                use_time_ms, is_stream, ip, request_id, content)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16)
               RETURNING id"#,
        )
        .bind(e.log_type)
        .bind(e.user_key)
        .bind(&e.username)
        .bind(e.token_key)
        .bind(&e.token_name)
        .bind(e.channel_key)
        .bind(&e.channel_name)
        .bind(&e.model_name)
        .bind(e.prompt_tokens)
        .bind(e.completion_tokens)
        .bind(e.quota)
        .bind(e.use_time_ms)
        .bind(e.is_stream)
        .bind(&e.ip)
        .bind(&e.request_id)
        .bind(&e.content)
        .fetch_one(&self.pool)
        .await?;
        Ok(id.0)
    }

    #[allow(clippy::too_many_arguments)]
    async fn query(
        &self,
        user_key: Option<Uuid>,
        log_type: Option<i16>,
        username: Option<&str>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;

        // 固定形状 SQL + NULL 传参 — 条件集固定, 不做动态拼接。
        let rows: Vec<LogView> = sqlx::query_as(&format!(
            r#"SELECT {COLS} FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND ($2::smallint IS NULL OR log_type = $2)
                 AND ($3::text IS NULL OR username = $3)
                 AND ($4::text IS NULL OR token_name = $4)
                 AND ($5::text IS NULL OR model_name = $5)
                 AND ($6::timestamptz IS NULL OR created_at >= $6)
                 AND ($7::timestamptz IS NULL OR created_at < $7)
               ORDER BY id DESC
               LIMIT $8 OFFSET $9"#
        ))
        .bind(user_key)
        .bind(log_type)
        .bind(username.filter(|s| !s.trim().is_empty()))
        .bind(token_name.filter(|s| !s.trim().is_empty()))
        .bind(model_name.filter(|s| !s.trim().is_empty()))
        .bind(start)
        .bind(end)
        .bind(size)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let total: i64 = sqlx::query_scalar(
            r#"SELECT count(*) FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND ($2::smallint IS NULL OR log_type = $2)
                 AND ($3::text IS NULL OR username = $3)
                 AND ($4::text IS NULL OR token_name = $4)
                 AND ($5::text IS NULL OR model_name = $5)
                 AND ($6::timestamptz IS NULL OR created_at >= $6)
                 AND ($7::timestamptz IS NULL OR created_at < $7)"#,
        )
        .bind(user_key)
        .bind(log_type)
        .bind(username.filter(|s| !s.trim().is_empty()))
        .bind(token_name.filter(|s| !s.trim().is_empty()))
        .bind(model_name.filter(|s| !s.trim().is_empty()))
        .bind(start)
        .bind(end)
        .fetch_one(&self.pool)
        .await?;

        Ok((rows, total))
    }

    /// admin 全量查询。
    #[allow(clippy::too_many_arguments)]
    pub async fn list_logs(
        &self,
        log_type: Option<i16>,
        username: Option<&str>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        self.query(None, log_type, username, token_name, model_name, start, end, page, size)
            .await
    }

    /// 用户自查。
    #[allow(clippy::too_many_arguments)]
    pub async fn list_self_logs(
        &self,
        user_key: Uuid,
        log_type: Option<i16>,
        token_name: Option<&str>,
        model_name: Option<&str>,
        start: Option<DateTime<Utc>>,
        end: Option<DateTime<Utc>>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<LogView>, i64), AuthError> {
        self.query(Some(user_key), log_type, None, token_name, model_name, start, end, page, size)
            .await
    }

    async fn stat_inner(&self, user_key: Option<Uuid>) -> Result<UsageStat, AuthError> {
        let stat = sqlx::query_as::<_, (i64, i64)>(
            r#"SELECT COALESCE(sum(quota),0)::bigint, count(*) FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND created_at >= date_trunc('day', now())"#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        let (rpm, tpm): (i64, i64) = sqlx::query_as(
            r#"SELECT count(*), COALESCE(sum(prompt_tokens + completion_tokens),0)::bigint FROM usage_logs
               WHERE ($1::uuid IS NULL OR user_key = $1)
                 AND created_at >= now() - interval '60 seconds'"#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(UsageStat {
            quota: stat.0,
            requests: stat.1,
            rpm,
            tpm,
        })
    }

    pub async fn stat(&self) -> Result<UsageStat, AuthError> {
        self.stat_inner(None).await
    }

    pub async fn self_stat(&self, user_key: Uuid) -> Result<UsageStat, AuthError> {
        self.stat_inner(Some(user_key)).await
    }

    /// dashboard 汇总 — 一次查全。
    pub async fn dashboard(&self) -> Result<serde_json::Value, AuthError> {
        let (users, tokens, channels, channels_enabled, groups): (i64, i64, i64, i64, i64) =
            sqlx::query_as(
                r#"SELECT
                   (SELECT count(*) FROM auth_users),
                   (SELECT count(*) FROM api_tokens),
                   (SELECT count(*) FROM api_channels),
                   (SELECT count(*) FROM api_channels WHERE status = 1),
                   (SELECT count(*) FROM api_groups)"#,
            )
            .fetch_one(&self.pool)
            .await?;
        let stat = self.stat().await?;
        Ok(serde_json::json!({
            "users": users,
            "tokens": tokens,
            "channels": channels,
            "channelsEnabled": channels_enabled,
            "groups": groups,
            "quotaToday": stat.quota,
            "requestsToday": stat.requests,
            "rpm": stat.rpm,
            "tpm": stat.tpm,
        }))
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct LogAppState {
    pub svc: std::sync::Arc<LogService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: LogAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/log", get(list))
        .route("/api/log/stat", get(stat))
        .route("/api/log/self", get(list_self))
        .route("/api/log/self/stat", get(self_stat))
        .route("/api/dashboard", get(dashboard))
        .with_state(state)
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        e.status(),
        axum::Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LogQuery {
    log_type: Option<i16>,
    username: Option<String>,
    token_name: Option<String>,
    model_name: Option<String>,
    start: Option<DateTime<Utc>>,
    end: Option<DateTime<Utc>>,
    page: Option<i64>,
    size: Option<i64>,
}

async fn list(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<LogQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state
        .svc
        .list_logs(
            q.log_type, q.username.as_deref(), q.token_name.as_deref(),
            q.model_name.as_deref(), q.start, q.end, q.page.unwrap_or(1), q.size.unwrap_or(20),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(serde_json::json!({"items": items, "total": total}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn stat(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state.svc.stat().await {
        Ok(s) => Ok(axum::Json(serde_json::json!(s))),
        Err(e) => Err(err_json(e)),
    }
}

async fn list_self(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<LogQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&user.key).map_err(|_| AuthError::InvalidToken).map_err(err_json)?;
    match state
        .svc
        .list_self_logs(
            key, q.log_type, q.token_name.as_deref(), q.model_name.as_deref(),
            q.start, q.end, q.page.unwrap_or(1), q.size.unwrap_or(20),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(serde_json::json!({"items": items, "total": total}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn self_stat(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&user.key).map_err(|_| AuthError::InvalidToken).map_err(err_json)?;
    match state.svc.self_stat(key).await {
        Ok(s) => Ok(axum::Json(serde_json::json!(s))),
        Err(e) => Err(err_json(e)),
    }
}

async fn dashboard(
    axum::extract::State(state): axum::extract::State<LogAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let user = bearer_user(&state.auth, &headers).await.map_err(err_json)?;
    if user.role < auth::routes::ADMIN_ROLE_THRESHOLD {
        return Err(err_json(AuthError::Forbidden));
    }
    match state.svc.dashboard().await {
        Ok(d) => Ok(axum::Json(d)),
        Err(e) => Err(err_json(e)),
    }
}
