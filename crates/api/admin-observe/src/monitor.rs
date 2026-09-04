//! 渠道探活历史 — 单机版平表实现 (monitor_history)。
//!
//! 探活调用方（catalog::channels::test_channel / ops::probe）写一行结果；
//! 面板按 (channel, days) 查可用率。日聚合/保留策略推迟（数据量小先全存）。
//!
//! 平表无 FK：channel_key 按名字引用 api_channels.key，渠道删除后历史保留
//! （可用率查询对已删渠道仍可读，面板自行过滤）。

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
CREATE TABLE IF NOT EXISTS monitor_history (
    id            BIGSERIAL PRIMARY KEY,
    channel_key   UUID NOT NULL,
    channel_name  TEXT NOT NULL DEFAULT '',
    model         TEXT NOT NULL DEFAULT '',
    ok            BOOLEAN NOT NULL,
    status_code   INT,
    latency_ms    INT NOT NULL DEFAULT 0,
    error_kind    TEXT NOT NULL DEFAULT '',
    message       TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_monitor_history_channel
    ON monitor_history(channel_key, created_at);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

/// 一次探活结果 — 探活执行方构造，落一行历史。
#[derive(Debug, Clone)]
pub struct ProbeOutcome {
    pub channel_key: Uuid,
    pub channel_name: String,
    pub model: String,
    pub ok: bool,
    pub status_code: Option<i32>,
    pub latency_ms: i32,
    /// 错误分类: timeout / connect / http / decode — ok=true 时为空
    pub error_kind: String,
    pub message: String,
}

/// 记录一次探活结果，返回行 id。
pub async fn record_probe(pool: &PgPool, o: &ProbeOutcome) -> Result<i64, AuthError> {
    let id: (i64,) = sqlx::query_as(
        r#"INSERT INTO monitor_history
           (channel_key, channel_name, model, ok, status_code, latency_ms, error_kind, message)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id"#,
    )
    .bind(o.channel_key)
    .bind(&o.channel_name)
    .bind(&o.model)
    .bind(o.ok)
    .bind(o.status_code)
    .bind(o.latency_ms)
    .bind(&o.error_kind)
    .bind(&o.message)
    .fetch_one(pool)
    .await?;
    Ok(id.0)
}

#[derive(Debug, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorRow {
    pub id: i64,
    pub channel_key: Uuid,
    pub channel_name: String,
    pub model: String,
    pub ok: bool,
    pub status_code: Option<i32>,
    pub latency_ms: i32,
    pub error_kind: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

/// 某渠道最近 N 条探活历史（新→旧）。
pub async fn history(
    pool: &PgPool,
    channel_key: Uuid,
    limit: i64,
) -> Result<Vec<MonitorRow>, AuthError> {
    let rows: Vec<MonitorRow> = sqlx::query_as(
        r#"SELECT id, channel_key, channel_name, model, ok, status_code,
                  latency_ms, error_kind, message, created_at
           FROM monitor_history WHERE channel_key = $1
           ORDER BY id DESC LIMIT $2"#,
    )
    .bind(channel_key)
    .bind(limit.clamp(1, 200))
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Availability {
    pub channel_key: String,
    pub days: u32,
    pub total: i64,
    pub ok_count: i64,
    /// 0.0..=1.0；窗口内无记录时为 None
    pub availability: Option<f64>,
    /// 窗口内成功探活的平均延迟（成功样本）
    pub avg_latency_ms: Option<f64>,
}

/// 某渠道近 N 天可用率。
pub async fn availability(
    pool: &PgPool,
    channel_key: Uuid,
    days: u32,
) -> Result<Availability, AuthError> {
    let days = days.clamp(1, 90);
    let (total, ok_count, avg): (i64, i64, Option<f64>) = sqlx::query_as(
        r#"SELECT count(*),
                  count(*) FILTER (WHERE ok),
                  (avg(latency_ms) FILTER (WHERE ok))::float8
           FROM monitor_history
           WHERE channel_key = $1 AND created_at >= now() - make_interval(days => $2)"#,
    )
    .bind(channel_key)
    .bind(days as i32)
    .fetch_one(pool)
    .await?;

    Ok(Availability {
        channel_key: channel_key.to_string(),
        days,
        total,
        ok_count,
        availability: if total > 0 {
            Some(ok_count as f64 / total as f64)
        } else {
            None
        },
        avg_latency_ms: avg,
    })
}

/// 全渠道可用率一览（渠道健康页）。
pub async fn availability_all(pool: &PgPool, days: u32) -> Result<Vec<Availability>, AuthError> {
    let days = days.clamp(1, 90);
    let rows: Vec<(Uuid, i64, i64, Option<f64>)> = sqlx::query_as(
        r#"SELECT channel_key, count(*), count(*) FILTER (WHERE ok),
                  (avg(latency_ms) FILTER (WHERE ok))::float8
           FROM monitor_history
           WHERE created_at >= now() - make_interval(days => $1)
           GROUP BY channel_key"#,
    )
    .bind(days as i32)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|(key, total, ok_count, avg)| Availability {
            channel_key: key.to_string(),
            days,
            total,
            ok_count,
            availability: if total > 0 {
                Some(ok_count as f64 / total as f64)
            } else {
                None
            },
            avg_latency_ms: avg,
        })
        .collect())
}

/// 探活落库依赖 — catalog 探活调用方持有（包一层避免 observe 内部直接暴露 pool）。
#[derive(Clone)]
pub struct MonitorDeps {
    pool: PgPool,
}

impl MonitorDeps {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 落一行探活历史。
    pub async fn record(&self, o: &ProbeOutcome) -> Result<(), AuthError> {
        record_probe(&self.pool, o).await.map(|_| ())
    }

    /// 某渠道历史。
    pub async fn history(
        &self,
        channel_key: Uuid,
        limit: i64,
    ) -> Result<Vec<MonitorRow>, AuthError> {
        history(&self.pool, channel_key, limit).await
    }

    /// 某渠道可用率。
    pub async fn availability(
        &self,
        channel_key: Uuid,
        days: u32,
    ) -> Result<Availability, AuthError> {
        availability(&self.pool, channel_key, days).await
    }

    /// 全渠道可用率。
    pub async fn availability_all(&self, days: u32) -> Result<Vec<Availability>, AuthError> {
        availability_all(&self.pool, days).await
    }
}

// ---------- axum 路由 (admin) ----------

#[derive(Clone)]
pub struct MonitorAppState {
    pub deps: MonitorDeps,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: MonitorAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/monitor/{key}", get(monitor_one))
        .route("/api/monitor", get(monitor_all))
        .with_state(state)
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        e.status(),
        axum::Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}

async fn require_admin(auth: &AuthService, headers: &HeaderMap) -> Result<(), AuthError> {
    let user = bearer_user(auth, headers).await?;
    if user.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

#[derive(Debug, Deserialize)]
struct MonitorQuery {
    days: Option<u32>,
    limit: Option<i64>,
}

/// GET /api/monitor/{key}?days=7&limit=50 — 某渠道历史 + 可用率。
async fn monitor_one(
    axum::extract::State(state): axum::extract::State<MonitorAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<MonitorQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    let days = q.days.unwrap_or(7);
    let history = state
        .deps
        .history(key, q.limit.unwrap_or(50))
        .await
        .map_err(err_json)?;
    let availability = state.deps.availability(key, days).await.map_err(err_json)?;
    Ok(axum::Json(
        serde_json::json!({ "history": history, "availability": availability }),
    ))
}

/// GET /api/monitor?days=7 — 全渠道可用率一览。
async fn monitor_all(
    axum::extract::State(state): axum::extract::State<MonitorAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<MonitorQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.deps.availability_all(q.days.unwrap_or(7)).await {
        Ok(items) => Ok(axum::Json(serde_json::json!({"items": items}))),
        Err(e) => Err(err_json(e)),
    }
}
