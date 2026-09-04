//! 渠道管理 — 单机版平表实现 (api_channels)。
//!
//! 替代原 store-trait 骨架 (sync/outbox 设计推迟)。
//! 字段覆盖 gateway dispatch::ChannelConfig 所需 (name/base_url/channel_type/keys/models)，
//! apps/api 迁移读这张表后 kv_store 的 JSON blob 可废弃。
//!
//! - keys: JSONB 字符串数组 (明文 key; 列表接口掩码，单查返回)
//! - models: JSONB 数组 [{alias, upstream}] (模型路由映射)

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
CREATE TABLE IF NOT EXISTS api_channels (
    key           UUID PRIMARY KEY,
    name          TEXT UNIQUE NOT NULL,
    channel_type  TEXT NOT NULL DEFAULT 'openai',
    base_url      TEXT NOT NULL DEFAULT '',
    keys          JSONB NOT NULL DEFAULT '[]',
    models        JSONB NOT NULL DEFAULT '[]',
    group_name    TEXT NOT NULL DEFAULT 'default',
    priority      INT NOT NULL DEFAULT 0,
    weight        INT NOT NULL DEFAULT 0,
    status        SMALLINT NOT NULL DEFAULT 1,
    test_model    TEXT,
    remark        TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelView {
    pub key: String,
    pub name: String,
    pub channel_type: String,
    pub base_url: String,
    /// keys 数量 (列表掩码；单查返回完整 keys)
    pub key_count: i64,
    pub keys: Option<Vec<String>>,
    pub models: Value,
    pub group_name: String,
    pub priority: i32,
    pub weight: i32,
    pub status: i16,
    pub test_model: Option<String>,
    pub remark: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct ChannelRow {
    key: Uuid,
    name: String,
    channel_type: String,
    base_url: String,
    keys: Value,
    models: Value,
    group_name: String,
    priority: i32,
    weight: i32,
    status: i16,
    test_model: Option<String>,
    remark: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const SELECT_COLS: &str = "key, name, channel_type, base_url, keys, models, group_name, \
     priority, weight, status, test_model, remark, created_at, updated_at";
fn row_to_view(r: ChannelRow, include_keys: bool) -> ChannelView {
    let keys: Vec<String> = serde_json::from_value(r.keys.clone()).unwrap_or_else(|e| {
        tracing::warn!(key = %r.key, error = %e, "corrupt keys JSONB — treating as empty");
        Vec::new()
    });
    ChannelView {
        key: r.key.to_string(),
        key_count: keys.len() as i64,
        keys: if include_keys { Some(keys) } else { None },
        name: r.name,
        channel_type: r.channel_type,
        base_url: r.base_url,
        models: r.models,
        group_name: r.group_name,
        priority: r.priority,
        weight: r.weight,
        status: r.status,
        test_model: r.test_model,
        remark: r.remark,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct ChannelService {
    pool: PgPool,
}

impl ChannelService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    #[allow(clippy::too_many_arguments)] // 显式字段 = 部分更新语义
    pub async fn create(
        &self,
        name: &str,
        channel_type: &str,
        base_url: &str,
        keys: Vec<String>,
        models: Value,
        group_name: &str,
        priority: i32,
        weight: i32,
        test_model: Option<String>,
        remark: &str,
    ) -> Result<ChannelView, AuthError> {
        validate(name, channel_type, base_url, &keys, &models)?;
        let key = Uuid::new_v4();
        let res = sqlx::query(
            r#"INSERT INTO api_channels
               (key, name, channel_type, base_url, keys, models, group_name,
                priority, weight, test_model, remark)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)"#,
        )
        .bind(key)
        .bind(name.trim())
        .bind(channel_type.trim())
        .bind(base_url.trim())
        .bind(serde_json::to_value(&keys).map_err(|e| AuthError::Crypto(e.to_string()))?)
        .bind(models)
        .bind(group_name.trim())
        .bind(priority)
        .bind(weight)
        .bind(test_model)
        .bind(remark.trim())
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(db)) = &res
            && db.code().as_deref() == Some("23505")
        {
            return Err(AuthError::Conflict("channel name taken".into()));
        }
        res?;

        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn list(
        &self,
        search: Option<&str>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<ChannelView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;
        let pattern = search
            .map(|s| format!("%{}%", s.trim()))
            .unwrap_or_else(|| "%".into());

        let total: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM api_channels WHERE name ILIKE $1 OR base_url ILIKE $1",
        )
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let sql = format!(
            "SELECT {SELECT_COLS} FROM api_channels \
             WHERE name ILIKE $1 OR base_url ILIKE $1 \
             ORDER BY priority DESC, created_at DESC LIMIT $2 OFFSET $3"
        );
        let rows: Vec<ChannelRow> = sqlx::query_as(&sql)
            .bind(&pattern)
            .bind(size)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        Ok((
            rows.into_iter().map(|r| row_to_view(r, false)).collect(),
            total,
        ))
    }

    pub async fn get(&self, key: Uuid) -> Result<ChannelView, AuthError> {
        Ok(row_to_view(self.fetch(key).await?, true))
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        key: Uuid,
        name: Option<&str>,
        channel_type: Option<&str>,
        base_url: Option<&str>,
        keys: Option<Vec<String>>,
        models: Option<Value>,
        group_name: Option<&str>,
        priority: Option<i32>,
        weight: Option<i32>,
        test_model: Option<Option<String>>,
        remark: Option<&str>,
        status: Option<i16>,
    ) -> Result<ChannelView, AuthError> {
        let existing = self.fetch(key).await?;
        // 合并后整体校验
        let merged_name = name.unwrap_or(&existing.name);
        let merged_type = channel_type.unwrap_or(&existing.channel_type);
        let merged_url = base_url.unwrap_or(&existing.base_url);
        let merged_keys: Vec<String> = match &keys {
            Some(k) => k.clone(),
            None => serde_json::from_value(existing.keys.clone()).unwrap_or_default(),
        };
        let merged_models = match &models {
            Some(v) => v.clone(),
            None => existing.models.clone(),
        };
        validate(merged_name, merged_type, merged_url, &merged_keys, &merged_models)?;

        let cur_keys = serde_json::to_value(&merged_keys).map_err(|e| AuthError::Crypto(e.to_string()))?;
        sqlx::query(
            r#"UPDATE api_channels SET
               name = COALESCE($2, name), channel_type = COALESCE($3, channel_type),
               base_url = COALESCE($4, base_url), keys = COALESCE($5, keys),
               models = COALESCE($6, models), group_name = COALESCE($7, group_name),
               priority = COALESCE($8, priority), weight = COALESCE($9, weight),
               test_model = $10, remark = COALESCE($11, remark),
               status = COALESCE($12, status), updated_at = now()
               WHERE key = $1"#,
        )
        .bind(key)
        .bind(name.map(str::trim))
        .bind(channel_type.map(str::trim))
        .bind(base_url.map(str::trim))
        .bind(keys.map(|_| cur_keys))
        .bind(models)
        .bind(group_name.map(str::trim))
        .bind(priority)
        .bind(weight)
        .bind(test_model)
        .bind(remark.map(str::trim))
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn set_status(&self, key: Uuid, status: i16) -> Result<ChannelView, AuthError> {
        if ![1, 2].contains(&status) {
            return Err(AuthError::BadRequest("status must be 1|2".into()));
        }
        let affected = sqlx::query("UPDATE api_channels SET status = $2, updated_at = now() WHERE key = $1")
            .bind(key)
            .bind(status)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AuthError::UserNotFound);
        }
        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), AuthError> {
        let affected = sqlx::query("DELETE FROM api_channels WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AuthError::UserNotFound);
        }
        Ok(())
    }

    async fn fetch(&self, key: Uuid) -> Result<ChannelRow, AuthError> {
        let sql = format!("SELECT {SELECT_COLS} FROM api_channels WHERE key = $1");
        sqlx::query_as(&sql)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::UserNotFound)
    }
}

/// 校验 (写前): name 非空≤64; type 非空≤32; base_url 合法 http(s); keys 非空;
/// models 每项 {alias, upstream} 均非空字符串。
fn validate(
    name: &str,
    channel_type: &str,
    base_url: &str,
    keys: &[String],
    models: &Value,
) -> Result<(), AuthError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(AuthError::BadRequest("name 1..=64 chars".into()));
    }
    let ct = channel_type.trim();
    if ct.is_empty() || ct.chars().count() > 32 {
        return Err(AuthError::BadRequest("channel_type 1..=32 chars".into()));
    }
    let url = base_url.trim();
    if url.is_empty() {
        return Err(AuthError::BadRequest("base_url required".into()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AuthError::BadRequest("base_url must be http(s) URL".into()));
    }
    if keys.is_empty() {
        return Err(AuthError::BadRequest("at least one key required".into()));
    }
    if let Some(arr) = models.as_array() {
        for m in arr {
            let alias = m.get("alias").and_then(|v| v.as_str()).unwrap_or("");
            let upstream = m.get("upstream").and_then(|v| v.as_str()).unwrap_or("");
            if alias.is_empty() || upstream.is_empty() {
                return Err(AuthError::BadRequest(
                    "models entries need non-empty alias+upstream".into(),
                ));
            }
        }
    } else if !models.is_null() {
        return Err(AuthError::BadRequest("models must be an array".into()));
    }
    Ok(())
}

// ---------- axum 路由 (admin) ----------

#[derive(Clone)]
pub struct ChannelAppState {
    pub svc: std::sync::Arc<ChannelService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: ChannelAppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/channel", get(list).post(create))
        .route("/api/channel/search", get(search))
        .route("/api/channel/{key}", get(get_one).put(update).delete(remove))
        .route("/api/channel/{key}/status", post(set_status))
        .with_state(state)
}

async fn require_admin(
    auth: &AuthService,
    headers: &HeaderMap,
) -> Result<(), AuthError> {
    let user = bearer_user(auth, headers).await?;
    if user.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        e.status(),
        axum::Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    search: Option<String>,
    keyword: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

async fn handle_list(
    state: &ChannelAppState,
    headers: &HeaderMap,
    search: Option<&str>,
    page: i64,
    size: i64,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, headers).await.map_err(err_json)?;
    match state.svc.list(search, page, size).await {
        Ok((items, total)) => Ok(axum::Json(serde_json::json!({"items": items, "total": total}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn list(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    handle_list(&state, &headers, q.search.as_deref(), q.page.unwrap_or(1), q.size.unwrap_or(20)).await
}

async fn search(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    let keyword = q.keyword.clone().or(q.search.clone()).unwrap_or_default();
    handle_list(&state, &headers, Some(&keyword), q.page.unwrap_or(1), q.size.unwrap_or(20)).await
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateChannelRequest {
    name: String,
    #[serde(default = "default_channel_type")]
    channel_type: String,
    #[serde(default)]
    base_url: String,
    keys: Vec<String>,
    #[serde(default)]
    models: Value,
    #[serde(default = "default_group")]
    group_name: String,
    #[serde(default)]
    priority: i32,
    #[serde(default)]
    weight: i32,
    test_model: Option<String>,
    #[serde(default)]
    remark: String,
}

fn default_channel_type() -> String {
    "openai".into()
}

fn default_group() -> String {
    "default".into()
}

async fn create(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<CreateChannelRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    match state
        .svc
        .create(
            &req.name, &req.channel_type, &req.base_url, req.keys, req.models,
            &req.group_name, req.priority, req.weight, req.test_model, &req.remark,
        )
        .await
    {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

async fn get_one(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| AuthError::BadRequest("invalid key".into())).map_err(err_json)?;
    match state.svc.get(key).await {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateChannelRequest {
    name: Option<String>,
    channel_type: Option<String>,
    base_url: Option<String>,
    keys: Option<Vec<String>>,
    models: Option<Value>,
    group_name: Option<String>,
    priority: Option<i32>,
    weight: Option<i32>,
    test_model: Option<Option<String>>,
    remark: Option<String>,
    status: Option<i16>,
}

async fn update(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::Json(req): axum::Json<UpdateChannelRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| AuthError::BadRequest("invalid key".into())).map_err(err_json)?;
    match state
        .svc
        .update(
            key,
            req.name.as_deref(),
            req.channel_type.as_deref(),
            req.base_url.as_deref(),
            req.keys,
            req.models,
            req.group_name.as_deref(),
            req.priority,
            req.weight,
            req.test_model,
            req.remark.as_deref(),
            req.status,
        )
        .await
    {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
struct StatusRequest {
    status: i16,
}

async fn set_status(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::Json(req): axum::Json<StatusRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| AuthError::BadRequest("invalid key".into())).map_err(err_json)?;
    match state.svc.set_status(key, req.status).await {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
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
