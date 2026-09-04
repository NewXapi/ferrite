//! 分组管理 — 单机版平表实现 (api_groups)。
//!
//! 替代原 store-trait 骨架。倍率 (ratio) 语义对齐 new-api: 用户组倍率 ×
//! 模型倍率 = 最终计费倍率 (MVP 只落组倍率)。
//! auth_users.group_id / api_tokens.group_id / api_channels.group_name
//! 按名字引用 (loose, 无 FK)；default 组不可删 (防孤儿)。

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS api_groups (
    key             UUID PRIMARY KEY,
    name            TEXT UNIQUE NOT NULL,
    ratio           DOUBLE PRECISION NOT NULL DEFAULT 1.0,
    model_whitelist JSONB NOT NULL DEFAULT '[]',
    remark          TEXT NOT NULL DEFAULT '',
    status          SMALLINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
INSERT INTO api_groups (key, name, ratio)
VALUES ('00000000-0000-0000-0000-0000000000d1', 'default', 1.0)
ON CONFLICT (name) DO NOTHING;
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupView {
    pub key: String,
    pub name: String,
    pub ratio: f64,
    pub model_whitelist: Value,
    pub remark: String,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, FromRow)]
struct GroupRow {
    key: Uuid,
    name: String,
    ratio: f64,
    model_whitelist: Value,
    remark: String,
    status: i16,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const COLS: &str =
    "key, name, ratio, model_whitelist, remark, status, created_at, updated_at";

pub struct GroupService {
    pool: PgPool,
}

impl GroupService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        name: &str,
        ratio: f64,
        model_whitelist: Value,
        remark: &str,
    ) -> Result<GroupView, AuthError> {
        validate(name, ratio, &model_whitelist)?;
        let key = Uuid::new_v4();
        let res = sqlx::query(
            "INSERT INTO api_groups (key, name, ratio, model_whitelist, remark) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(key)
        .bind(name.trim())
        .bind(ratio)
        .bind(model_whitelist)
        .bind(remark.trim())
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(db)) = &res
            && db.code().as_deref() == Some("23505")
        {
            return Err(AuthError::Conflict("group name taken".into()));
        }
        res?;
        Ok(self.fetch(key).await?.into())
    }

    pub async fn list(&self) -> Result<Vec<GroupView>, AuthError> {
        let sql = format!("SELECT {COLS} FROM api_groups ORDER BY name");
        let rows: Vec<GroupRow> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn update(
        &self,
        key: Uuid,
        ratio: Option<f64>,
        model_whitelist: Option<Value>,
        remark: Option<&str>,
        status: Option<i16>,
    ) -> Result<GroupView, AuthError> {
        let existing: GroupRow = sqlx::query_as(&format!("SELECT {COLS} FROM api_groups WHERE key = $1"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        if let Some(r) = ratio
            && !(r > 0.0 && r.is_finite())
        {
            return Err(AuthError::BadRequest("ratio must be > 0".into()));
        }
        if let Some(w) = &model_whitelist {
            validate_whitelist(w)?;
        }
        if let Some(s) = status {
            if ![1, 2].contains(&s) {
                return Err(AuthError::BadRequest("status must be 1|2".into()));
            }
            // default 组不可停用
            if existing.name == "default" && s == 2 {
                return Err(AuthError::BadRequest("default group cannot be disabled".into()));
            }
        }

        sqlx::query(
            "UPDATE api_groups SET ratio = COALESCE($2, ratio), \
             model_whitelist = COALESCE($3, model_whitelist), remark = COALESCE($4, remark), \
             status = COALESCE($5, status), updated_at = now() WHERE key = $1",
        )
        .bind(key)
        .bind(ratio)
        .bind(model_whitelist)
        .bind(remark.map(str::trim))
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(self.fetch(key).await?.into())
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), AuthError> {
        let existing: GroupRow = sqlx::query_as(&format!("SELECT {COLS} FROM api_groups WHERE key = $1"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::UserNotFound)?;
        if existing.name == "default" {
            return Err(AuthError::BadRequest("default group cannot be deleted".into()));
        }
        // 应用层引用检查 (loose FK)
        let refs: i64 = sqlx::query_scalar(
            "SELECT (SELECT count(*) FROM auth_users WHERE group_id = $1) \
           + (SELECT count(*) FROM api_tokens WHERE group_id = $1) \
           + (SELECT count(*) FROM api_channels WHERE group_name = $1)",
        )
        .bind(&existing.name)
        .fetch_one(&self.pool)
        .await?;
        if refs > 0 {
            return Err(AuthError::Conflict(format!(
                "group still referenced by {refs} object(s)"
            )));
        }
        sqlx::query("DELETE FROM api_groups WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn fetch(&self, key: Uuid) -> Result<GroupRow, AuthError> {
        sqlx::query_as(&format!("SELECT {COLS} FROM api_groups WHERE key = $1"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::UserNotFound)
    }
}

fn validate(name: &str, ratio: f64, whitelist: &Value) -> Result<(), AuthError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 32 {
        return Err(AuthError::BadRequest("name 1..=32 chars".into()));
    }
    if name == "default" {
        return Err(AuthError::BadRequest("default group is reserved".into()));
    }
    if !(ratio > 0.0 && ratio.is_finite()) {
        return Err(AuthError::BadRequest("ratio must be > 0".into()));
    }
    validate_whitelist(whitelist)
}

fn validate_whitelist(v: &Value) -> Result<(), AuthError> {
    if let Some(arr) = v.as_array() {
        for m in arr {
            if m.as_str().unwrap_or("").is_empty() {
                return Err(AuthError::BadRequest("whitelist entries must be non-empty strings".into()));
            }
        }
        Ok(())
    } else {
        Err(AuthError::BadRequest("model_whitelist must be an array".into()))
    }
}

impl From<GroupRow> for GroupView {
    fn from(r: GroupRow) -> Self {
        Self {
            key: r.key.to_string(),
            name: r.name,
            ratio: r.ratio,
            model_whitelist: r.model_whitelist,
            remark: r.remark,
            status: r.status,
            created_at: r.created_at,
            updated_at: r.updated_at,
        }
    }
}

// ---------- axum 路由 (admin) ----------

#[derive(Clone)]
pub struct GroupAppState {
    pub svc: std::sync::Arc<GroupService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: GroupAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/group", get(list).post(create))
        .route("/api/group/{key}", axum::routing::put(update).delete(remove))
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

async fn list(
    axum::extract::State(state): axum::extract::State<GroupAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    match state.svc.list().await {
        Ok(items) => Ok(axum::Json(serde_json::json!({"items": items}))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateGroupRequest {
    name: String,
    #[serde(default = "default_ratio")]
    ratio: f64,
    #[serde(default)]
    model_whitelist: Value,
    #[serde(default)]
    remark: String,
}

fn default_ratio() -> f64 {
    1.0
}

async fn create(
    axum::extract::State(state): axum::extract::State<GroupAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<CreateGroupRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    match state.svc.create(&req.name, req.ratio, req.model_whitelist, &req.remark).await {
        Ok(g) => Ok(axum::Json(serde_json::json!(g))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateGroupRequest {
    ratio: Option<f64>,
    model_whitelist: Option<Value>,
    remark: Option<String>,
    status: Option<i16>,
}

async fn update(
    axum::extract::State(state): axum::extract::State<GroupAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::Json(req): axum::Json<UpdateGroupRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| AuthError::BadRequest("invalid key".into())).map_err(err_json)?;
    match state
        .svc
        .update(key, req.ratio, req.model_whitelist, req.remark.as_deref(), req.status)
        .await
    {
        Ok(g) => Ok(axum::Json(serde_json::json!(g))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    axum::extract::State(state): axum::extract::State<GroupAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| AuthError::BadRequest("invalid key".into())).map_err(err_json)?;
    match state.svc.delete(key).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => Err(err_json(e)),
    }
}
