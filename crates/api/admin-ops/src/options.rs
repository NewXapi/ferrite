//! 运行时选项 — one-api Option 表 + new-api 类型化校验的合并（平表直连 sqlx）。
//!
//! 每个选项 key 注册默认值 + validator；写入校验后落 `options` 表。
//! routing_visible 选项变更需下发 edge（单机版暂只标记，供 gateway 快照刷新）。

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        r#"CREATE TABLE IF NOT EXISTS options (
    key        TEXT PRIMARY KEY,
    value      JSONB NOT NULL,
    updated_by UUID,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO options (key, value) VALUES ('site.registration_enabled', 'true')
    ON CONFLICT (key) DO NOTHING;
INSERT INTO options (key, value) VALUES ('site.quota_new_user', '0')
    ON CONFLICT (key) DO NOTHING;"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// 选项定义（注册表条目）。
pub struct OptionSpec {
    pub key: &'static str,
    pub default: serde_json::Value,
    /// 值域校验（类型/范围/枚举）。
    pub validate: fn(&serde_json::Value) -> Result<(), String>,
}

impl OptionSpec {
    fn validate_value(&self, v: &serde_json::Value) -> Result<(), AuthError> {
        (self.validate)(v).map_err(AuthError::BadRequest)
    }
}

/// 首批选项（酒馆中转单机版所需）。
pub fn registry() -> Vec<OptionSpec> {
    vec![
        OptionSpec {
            key: "site.registration_enabled",
            default: serde_json::json!(true),
            validate: |v| v.is_boolean().then_some(()).ok_or("must be boolean".into()),
        },
        OptionSpec {
            key: "site.quota_new_user",
            default: serde_json::json!(0),
            validate: |v| {
                v.as_i64().filter(|q| *q >= 0).map(|_| ()).ok_or("must be non-negative integer".into())
            },
        },
        OptionSpec {
            key: "gateway.retry.max_attempts",
            default: serde_json::json!(3),
            validate: |v| {
                v.as_i64().filter(|n| (1..=10).contains(n)).map(|_| ()).ok_or("must be 1..=10".into())
            },
        },
        OptionSpec {
            key: "gateway.timeout.first_byte_ms",
            default: serde_json::json!(30000),
            validate: |v| {
                v.as_i64().filter(|n| *n >= 1000).map(|_| ()).ok_or("must be >= 1000 ms".into())
            },
        },
        OptionSpec {
            key: "observe.retention.usage_days",
            default: serde_json::json!(90),
            validate: |v| {
                v.as_i64().filter(|n| (7..=365).contains(n)).map(|_| ()).ok_or("must be 7..=365".into())
            },
        },
    ]
}

fn spec_of(key: &str) -> Option<OptionSpec> {
    registry().into_iter().find(|s| s.key == key)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OptionView {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

pub struct OptionsService {
    pool: PgPool,
}

impl OptionsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 全部选项（库值回退注册表默认值）。
    pub async fn list(&self) -> Result<Vec<OptionView>, AuthError> {
        let stored: Vec<(String, serde_json::Value, sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>)> =
            sqlx::query_as("SELECT key, value, updated_at FROM options ORDER BY key")
                .fetch_all(&self.pool)
                .await?;
        let mut out: Vec<OptionView> = stored
            .into_iter()
            .map(|(key, value, updated_at)| OptionView { key, value, updated_at })
            .collect();
        // 注册表里有但库中没有的，补默认值视图
        for spec in registry() {
            if !out.iter().any(|o| o.key == spec.key) {
                out.push(OptionView {
                    key: spec.key.to_string(),
                    value: spec.default.clone(),
                    updated_at: sqlx::types::chrono::DateTime::<sqlx::types::chrono::Utc>::UNIX_EPOCH,
                });
            }
        }
        out.sort_by(|a, b| a.key.cmp(&b.key));
        Ok(out)
    }

    /// 读取（带默认回退）。
    pub async fn get(&self, key: &str) -> Result<serde_json::Value, AuthError> {
        let row: Option<(serde_json::Value,)> =
            sqlx::query_as("SELECT value FROM options WHERE key = $1")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?;
        if let Some((v,)) = row {
            return Ok(v);
        }
        spec_of(key)
            .map(|s| s.default.clone())
            .ok_or(AuthError::NotFound("option not found".into()))
    }

    /// 写入（校验 → 落库；未知 key 拒绝）。
    pub async fn set(
        &self,
        actor: Uuid,
        key: &str,
        value: serde_json::Value,
    ) -> Result<serde_json::Value, AuthError> {
        let spec = spec_of(key)
            .ok_or_else(|| AuthError::BadRequest(format!("unknown option key: {key}")))?;
        spec.validate_value(&value)?;
        sqlx::query(
            "INSERT INTO options (key, value, updated_by) VALUES ($1, $2, $3)
             ON CONFLICT (key) DO UPDATE SET value = $2, updated_by = $3, updated_at = now()",
        )
        .bind(key)
        .bind(&value)
        .bind(actor)
        .execute(&self.pool)
        .await?;
        Ok(value)
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct OptionsAppState {
    pub svc: std::sync::Arc<OptionsService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: OptionsAppState) -> axum::Router {
    use axum::routing::{get, put};
    axum::Router::new()
        .route("/api/option", get(list).put(update))
        .route("/api/option/{key}", get(get_one))
        .with_state(state)
}

async fn require_admin(auth: &AuthService, h: &HeaderMap) -> Result<Uuid, AuthError> {
    let u = bearer_user(auth, h).await?;
    if u.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Uuid::parse_str(&u.key).map_err(|_| AuthError::InvalidToken)
    } else {
        Err(AuthError::Forbidden)
    }
}

type ErrResp = (StatusCode, Json<serde_json::Value>);
fn err_json(e: AuthError) -> ErrResp {
    (e.status(), Json(json!({ "code": e.code(), "message": e.to_string() })))
}

async fn list(
    State(s): State<OptionsAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let items = s.svc.list().await.map_err(err_json)?;
    Ok(Json(json!({ "items": items })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRequest {
    key: String,
    value: serde_json::Value,
}

async fn update(
    State(s): State<OptionsAppState>,
    h: HeaderMap,
    Json(req): Json<UpdateRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let actor = require_admin(&s.auth, &h).await.map_err(err_json)?;
    let value = s.svc.set(actor, &req.key, req.value).await.map_err(err_json)?;
    Ok(Json(json!({ "key": req.key, "value": value })))
}

async fn get_one(
    State(s): State<OptionsAppState>,
    h: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let value = s.svc.get(&key).await.map_err(err_json)?;
    Ok(Json(json!({ "key": key, "value": value })))
}
