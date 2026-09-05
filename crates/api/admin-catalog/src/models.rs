//! 模型管理 — 单机版平表实现 (api_models)。
//!
//! 替代原 store-trait 骨架 (sync/outbox 设计推迟)。
//!
//! 参考 new-api controller/model.go + wildtoken models (能力 + 标签支持)。
//!
//! - name: 唯一标识，用于路由识别
//! - owner: 归属用户/团队，用于权限隔离
//! - model_type: "chat"|"image"|"audio"|"video"|"text"|"embedding" 等
//! - base_url: 后端服务地址 (OpenAI 兼容格式)
//! - api_key: 请求密钥，列表接口掩码，单查返回明文
//! - capabilities: 功能集合 {"vision", "tool_call", "stream"}
//! - speed/rating/usage_count: 统计
//! - tags: 用于分组和管理（JSONB数组字符串）
//! - status: 0=禁用, 1=启用, 2=维护中

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
CREATE TABLE IF NOT EXISTS api_models (
    key           UUID PRIMARY KEY,
    name          TEXT UNIQUE NOT NULL,
    owner         TEXT NOT NULL DEFAULT '',
    model_type    TEXT NOT NULL DEFAULT 'chat',
    base_url      TEXT NOT NULL DEFAULT '',
    api_key       TEXT NOT NULL DEFAULT '',
    capabilities  JSONB NOT NULL DEFAULT '[]',
    speed         INT NOT NULL DEFAULT 0,
    rating        JSONB NOT NULL DEFAULT '{}',
    usage_count   BIGINT NOT NULL DEFAULT 0,
    max_tokens    INT NOT NULL DEFAULT 0,
    is_vision     BOOLEAN NOT NULL DEFAULT false,
    is_tool       BOOLEAN NOT NULL DEFAULT false,
    tags          JSONB NOT NULL DEFAULT '[]',
    status        SMALLINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_api_models_name ON api_models(name);
CREATE INDEX idx_api_models_owner ON api_models(owner);
CREATE INDEX idx_api_models_status ON api_models(status);
CREATE INDEX idx_api_models_type ON api_models(model_type);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub key: String,
    pub name: String,
    pub owner: String,
    pub model_type: String,
    pub base_url: String,
    /// api_key 列表掩码 (sk-****)；单查返回完整明文
    pub api_key_preview: String,
    pub capabilities: Value,
    pub speed: i32,
    pub rating: Value,
    pub usage_count: i64,
    pub max_tokens: i32,
    pub is_vision: bool,
    pub is_tool: bool,
    pub tags: Value,
    pub status: i16,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct ModelRow {
    key: Uuid,
    name: String,
    owner: String,
    model_type: String,
    base_url: String,
    api_key: String,
    capabilities: Value,
    speed: i32,
    rating: Value,
    usage_count: i64,
    max_tokens: i32,
    is_vision: bool,
    is_tool: bool,
    tags: Value,
    status: i16,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

fn row_to_view(r: ModelRow, include_api_key: bool) -> ModelView {
    ModelView {
        key: r.key.to_string(),
        name: r.name,
        owner: r.owner,
        model_type: r.model_type,
        base_url: r.base_url,
        api_key_preview: if include_api_key { r.api_key.clone() } else { preview_api_key(&r.api_key) },
        capabilities: r.capabilities,
        speed: r.speed,
        rating: r.rating,
        usage_count: r.usage_count,
        max_tokens: r.max_tokens,
        is_vision: r.is_vision,
        is_tool: r.is_tool,
        tags: r.tags,
        status: r.status,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn preview_api_key(key: &str) -> String {
    if key.is_empty() {
        return String::new();
    }
    if key.starts_with("sk-") {
        format!("sk-****")
    } else {
        format!("****")
    }
}

pub struct ModelService {
    pool: PgPool,
}

impl ModelService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create(
        &self,
        name: &str,
        owner: &str,
        model_type: &str,
        base_url: &str,
        api_key: &str,
        capabilities: Value,
        speed: i32,
        rating: Value,
        usage_count: i64,
        max_tokens: i32,
        is_vision: bool,
        is_tool: bool,
        tags: Value,
        status: i16,
    ) -> Result<ModelView, AuthError> {
        validate(
            name,
            owner,
            model_type,
            base_url,
            api_key,
            &capabilities,
            &tags,
            status,
        )?;
        let key = Uuid::new_v4();
        let res = sqlx::query(
            r#"INSERT INTO api_models
               (key, name, owner, model_type, base_url, api_key, capabilities,
                speed, rating, usage_count, max_tokens, is_vision, is_tool, tags, status)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)"#,
        )
        .bind(key)
        .bind(name.trim())
        .bind(owner.trim())
        .bind(model_type.trim())
        .bind(base_url.trim())
        .bind(api_key.trim())
        .bind(capabilities)
        .bind(speed)
        .bind(rating)
        .bind(usage_count)
        .bind(max_tokens)
        .bind(is_vision)
        .bind(is_tool)
        .bind(tags)
        .bind(status)
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(db)) = &res
            && db.code().as_deref() == Some("23505")
        {
            return Err(AuthError::Conflict("model name taken".into()));
        }
        res?;
        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn list(
        &self,
        search: Option<&str>,
        page: i64,
        size: i64,
        owner: Option<&str>,
        status: Option<i16>,
        model_type: Option<&str>,
    ) -> Result<(Vec<ModelView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;
        let pattern = search
            .map(|s| format!("%{}%", s.trim()))
            .unwrap_or_else(|| "%".into());

        let mut where_clauses = vec!["1=1".to_string()];
        let mut param_idx = 1;

        if let Some(owner) = owner {
            where_clauses.push(format!("owner = ${}", param_idx));
            param_idx += 1;
        }
        if let Some(status) = status {
            where_clauses.push(format!("status = ${}", param_idx));
            param_idx += 1;
        }
        if let Some(model_type) = model_type {
            where_clauses.push(format!("model_type = ${}", param_idx));
            param_idx += 1;
        }
        where_clauses.push(format!("name ILIKE ${}", param_idx));
        param_idx += 1;

        let where_sql = where_clauses.join(" AND ");

        // Build count query with dynamic params
        let count_sql = format!("SELECT count(*) FROM api_models WHERE {}", where_sql);
        let mut count_query = sqlx::query_scalar(&count_sql);

        if let Some(owner) = owner {
            count_query = count_query.bind(owner);
        }
        if let Some(status) = status {
            count_query = count_query.bind(status);
        }
        if let Some(model_type) = model_type {
            count_query = count_query.bind(model_type);
        }
        count_query = count_query.bind(&pattern);

        let total: i64 = count_query.fetch_one(&self.pool).await?;

        // Build select query
        let select_sql = format!(
            "SELECT key, name, owner, model_type, base_url, api_key, capabilities,
             speed, rating, usage_count, max_tokens, is_vision, is_tool, tags, status, created_at, updated_at
             FROM api_models WHERE {} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_sql, param_idx, param_idx + 1
        );
        let mut select_query = sqlx::query_as::<_, ModelRow>(&select_sql);

        if let Some(owner) = owner {
            select_query = select_query.bind(owner);
        }
        if let Some(status) = status {
            select_query = select_query.bind(status);
        }
        if let Some(model_type) = model_type {
            select_query = select_query.bind(model_type);
        }
        select_query = select_query.bind(&pattern).bind(size).bind(offset);

        let rows: Vec<ModelRow> = select_query.fetch_all(&self.pool).await?;

        Ok((
            rows.into_iter().map(|r| row_to_view(r, false)).collect(),
            total,
        ))
    }

    pub async fn get(&self, key: Uuid) -> Result<ModelView, AuthError> {
        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn update(
        &self,
        key: Uuid,
        name: Option<&str>,
        owner: Option<&str>,
        model_type: Option<&str>,
        base_url: Option<&str>,
        api_key: Option<&str>,
        capabilities: Option<Value>,
        speed: Option<i32>,
        rating: Option<Value>,
        max_tokens: Option<i32>,
        is_vision: Option<bool>,
        is_tool: Option<bool>,
        tags: Option<Value>,
        status: Option<i16>,
    ) -> Result<ModelView, AuthError> {
        let existing = self.fetch(key).await?;

        let merged_name = name.unwrap_or(&existing.name);
        let merged_owner = owner.unwrap_or(&existing.owner);
        let merged_type = model_type.unwrap_or(&existing.model_type);
        let merged_url = base_url.unwrap_or(&existing.base_url);
        let merged_api_key = api_key.unwrap_or(&existing.api_key);
        let merged_capabilities = capabilities.unwrap_or(existing.capabilities.clone());
        let merged_speed = speed.unwrap_or(existing.speed);
        let merged_rating = rating.unwrap_or(existing.rating.clone());
        let merged_max_tokens = max_tokens.unwrap_or(existing.max_tokens);
        let merged_is_vision = is_vision.unwrap_or(existing.is_vision);
        let merged_is_tool = is_tool.unwrap_or(existing.is_tool);
        let merged_tags = tags.unwrap_or(existing.tags.clone());
        let merged_status = status.unwrap_or(existing.status);

        validate(
            merged_name,
            merged_owner,
            merged_type,
            merged_url,
            merged_api_key,
            &merged_capabilities,
            &merged_tags,
            merged_status,
        )?;

        sqlx::query(
            r#"UPDATE api_models SET
               name = COALESCE($2, name), owner = COALESCE($3, owner),
               model_type = COALESCE($4, model_type), base_url = COALESCE($5, base_url),
               api_key = COALESCE($6, api_key), capabilities = COALESCE($7, capabilities),
               speed = COALESCE($8, speed), rating = COALESCE($9, rating),
               max_tokens = COALESCE($10, max_tokens), is_vision = COALESCE($11, is_vision),
               is_tool = COALESCE($12, is_tool), tags = COALESCE($13, tags),
               status = COALESCE($14, status), updated_at = now()
               WHERE key = $1"#,
        )
        .bind(key)
        .bind(name.map(str::trim))
        .bind(owner.map(str::trim))
        .bind(model_type.map(str::trim))
        .bind(base_url.map(str::trim))
        .bind(api_key.map(str::trim))
        .bind(capabilities)
        .bind(speed)
        .bind(rating)
        .bind(max_tokens)
        .bind(is_vision)
        .bind(is_tool)
        .bind(tags)
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn set_status(&self, key: Uuid, status: i16) -> Result<ModelView, AuthError> {
        if ![0, 1, 2].contains(&status) {
            return Err(AuthError::BadRequest("status must be 0|1|2".into()));
        }
        let affected = sqlx::query(
            "UPDATE api_models SET status = $2, updated_at = now() WHERE key = $1"
        )
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
        let affected = sqlx::query("DELETE FROM api_models WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if affected == 0 {
            return Err(AuthError::UserNotFound);
        }
        Ok(())
    }

    /// 缺失模型检测：对比 api_channels 的 models 引用，找出在 api_models 中不存在的模型名
    pub async fn missing(&self) -> Result<Vec<String>, AuthError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT jsonb_array_elements_text(models->>'upstream') as model_name
            FROM api_channels
            WHERE models IS NOT NULL
            "#
        ).fetch_all(&self.pool).await?;

        let mut missing_models = Vec::new();
        for (name,) in rows {
            let exists: Option<(String,)> = sqlx::query_as(
                "SELECT name FROM api_models WHERE name = $1"
            ).bind(&name).fetch_optional(&self.pool).await?;
            if exists.is_none() {
                missing_models.push(name);
            }
        }
        Ok(missing_models)
    }

    /// 同步预览：读取上游模型列表但不落库
    pub async fn sync_preview(&self, _payload: Value) -> Result<Value, AuthError> {
        // TODO: 实现从 forward crate 读取上游模型列表
        // 当前返回 stub
        Ok(serde_json::json!({
            "models": [],
            "message": "sync_preview not yet implemented - requires forward crate"
        }))
    }

    /// 从上游拉取落库
    pub async fn sync_upstream(&self, _payload: Value) -> Result<Value, AuthError> {
        // TODO: 实现从 forward crate 同步上游模型
        // 当前返回 stub
        Ok(serde_json::json!({
            "synced": 0,
            "message": "sync_upstream not yet implemented - requires forward crate"
        }))
    }

    async fn fetch(&self, key: Uuid) -> Result<ModelRow, AuthError> {
        let sql = "SELECT key, name, owner, model_type, base_url, api_key, capabilities, speed, rating, usage_count, max_tokens, is_vision, is_tool, tags, status, created_at, updated_at FROM api_models WHERE key = $1";
        sqlx::query_as::<_, ModelRow>(&sql)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::UserNotFound)
    }
}

fn validate(
    name: &str,
    owner: &str,
    model_type: &str,
    base_url: &str,
    api_key: &str,
    capabilities: &Value,
    tags: &Value,
    status: i16,
) -> Result<(), AuthError> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 128 {
        return Err(AuthError::BadRequest("name 1..=128 chars".into()));
    }
    let owner = owner.trim();
    if owner.is_empty() || owner.chars().count() > 64 {
        return Err(AuthError::BadRequest("owner 1..=64 chars".into()));
    }
    let mt = model_type.trim();
    let allowed = ["chat", "image", "audio", "video", "text", "embedding"];
    if mt.is_empty() || !allowed.contains(&mt) {
        return Err(AuthError::BadRequest("model_type must be one of chat/image/audio/video/text/embedding".into()));
    }
    let url = base_url.trim();
    if url.is_empty() {
        return Err(AuthError::BadRequest("base_url required".into()));
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AuthError::BadRequest("base_url must be http(s) URL".into()));
    }
    let key = api_key.trim();
    if key.is_empty() || key.len() > 256 {
        return Err(AuthError::BadRequest("api_key 1..=256 chars".into()));
    }
    if let Some(arr) = capabilities.as_array() {
        for cap in arr {
            if cap.as_str().is_none() {
                return Err(AuthError::BadRequest("capabilities array must be strings".into()));
            }
        }
    } else if !capabilities.is_null() {
        return Err(AuthError::BadRequest("capabilities must be an array or null".into()));
    }
    if let Some(arr) = tags.as_array() {
        for t in arr {
            if t.as_str().is_none() {
                return Err(AuthError::BadRequest("tags array must be strings".into()));
            }
        }
    } else if !tags.is_null() {
        return Err(AuthError::BadRequest("tags must be an array or null".into()));
    }
    if ![0, 1, 2].contains(&status) {
        return Err(AuthError::BadRequest("status must be 0|1|2".into()));
    }
    Ok(())
}

// ---------- axum 路由 (admin) ----------

#[derive(Clone)]
pub struct ModelAppState {
    pub svc: std::sync::Arc<ModelService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: ModelAppState) -> axum::Router {
    use axum::routing::{get, post, put, delete};
    axum::Router::new()
        .route("/api/models", get(list).post(create))
        .route("/api/models/search", get(search))
        .route("/api/models/{key}", get(get_one).put(update).delete(remove))
        .route("/api/models/missing", get(missing))
        .route("/api/models/sync_upstream/preview", post(sync_preview))
        .route("/api/models/sync_upstream", post(sync_upstream))
        .with_state(state)
}

async fn require_admin(auth: &AuthService, headers: &HeaderMap) -> Result<(), AuthError> {
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
#[serde(rename_all = "camelCase")]
struct CreateModelRequest {
    name: String,
    owner: String,
    #[serde(default = "default_model_type")]
    model_type: String,
    #[serde(default)]
    base_url: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    capabilities: Value,
    #[serde(default)]
    speed: i32,
    #[serde(default)]
    rating: Value,
    #[serde(default)]
    usage_count: i64,
    #[serde(default)]
    max_tokens: i32,
    #[serde(default)]
    is_vision: bool,
    #[serde(default)]
    is_tool: bool,
    #[serde(default)]
    tags: Value,
    #[serde(default)]
    status: i16,
}

fn default_model_type() -> String {
    "chat".into()
}

async fn create(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<CreateModelRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state
        .svc
        .create(
            &req.name,
            &req.owner,
            &req.model_type,
            &req.base_url,
            &req.api_key,
            req.capabilities,
            req.speed,
            req.rating,
            req.usage_count,
            req.max_tokens,
            req.is_vision,
            req.is_tool,
            req.tags,
            req.status,
        )
        .await
    {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    search: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
    owner: Option<String>,
    status: Option<i16>,
    model_type: Option<String>,
}

async fn list(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state
        .svc
        .list(
            q.search.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
            q.owner.as_deref(),
            q.status,
            q.model_type.as_deref(),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(serde_json::json!({"items": items, "total": total}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn search(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state
        .svc
        .list(
            q.search.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
            q.owner.as_deref(),
            q.status,
            q.model_type.as_deref(),
        )
        .await
    {
        Ok((items, total)) => Ok(axum::Json(serde_json::json!({"items": items, "total": total}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn get_one(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    match state.svc.get(key).await {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateModelRequest {
    name: Option<String>,
    owner: Option<String>,
    model_type: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    capabilities: Option<Value>,
    speed: Option<i32>,
    rating: Option<Value>,
    max_tokens: Option<i32>,
    is_vision: Option<bool>,
    is_tool: Option<bool>,
    tags: Option<Value>,
    status: Option<i16>,
}

async fn update(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::Json(req): axum::Json<UpdateModelRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    match state
        .svc
        .update(
            key,
            req.name.as_deref(),
            req.owner.as_deref(),
            req.model_type.as_deref(),
            req.base_url.as_deref(),
            req.api_key.as_deref(),
            req.capabilities,
            req.speed,
            req.rating,
            req.max_tokens,
            req.is_vision,
            req.is_tool,
            req.tags,
            req.status,
        )
        .await
    {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    match state.svc.delete(key).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => Err(err_json(e)),
    }
}

async fn missing(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.missing().await {
        Ok(missing) => Ok(axum::Json(serde_json::json!(missing))),
        Err(e) => Err(err_json(e)),
    }
}

async fn sync_preview(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.sync_preview(payload).await {
        Ok(preview) => Ok(axum::Json(serde_json::json!(preview))),
        Err(e) => Err(err_json(e)),
    }
}

async fn sync_upstream(
    axum::extract::State(state): axum::extract::State<ModelAppState>,
    headers: HeaderMap,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.sync_upstream(payload).await {
        Ok(result) => Ok(axum::Json(serde_json::json!(result))),
        Err(e) => Err(err_json(e)),
    }
}