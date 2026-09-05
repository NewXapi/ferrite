//! 模型管理 — 单机版平表实现 (api_models)。

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
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
    status        SMALLINT NOT NULL DEFAULT 1,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_api_models_name ON api_models(name);
CREATE INDEX IF NOT EXISTS idx_api_models_owner ON api_models(owner);
CREATE INDEX IF NOT EXISTS idx_api_models_model_type ON api_models(model_type);
CREATE INDEX IF NOT EXISTS idx_api_models_status ON api_models(status);
CREATE INDEX IF NOT EXISTS idx_api_models_created_at ON api_models(created_at DESC);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelView {
    pub key: String,
    pub name: String,
    pub owner: String,
    pub model_type: String,
    pub base_url: String,
    pub masked_key: String,
    pub capabilities: Value,
    pub speed: i32,
    pub rating: Value,
    pub usage_count: i64,
    pub max_tokens: i32,
    pub is_vision: bool,
    pub is_tool: bool,
    pub status: i16,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
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
    status: i16,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

const COLS: &str = "key, name, owner, model_type, base_url, api_key, capabilities, \
     speed, rating, usage_count, max_tokens, is_vision, is_tool, status, created_at, updated_at";

fn row_to_view(r: ModelRow) -> ModelView {
    ModelView {
        key: r.key.to_string(),
        name: r.name,
        owner: r.owner,
        model_type: r.model_type,
        base_url: r.base_url,
        masked_key: mask_key(&r.api_key),
        capabilities: r.capabilities,
        speed: r.speed,
        rating: r.rating,
        usage_count: r.usage_count,
        max_tokens: r.max_tokens,
        is_vision: r.is_vision,
        is_tool: r.is_tool,
        status: r.status,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

fn mask_key(plaintext: &str) -> String {
    if plaintext.len() <= 8 {
        return "****".to_string();
    }
    let head = &plaintext[..4];
    let tail = &plaintext[plaintext.len() - 4..];
    format!("{head}****{tail}")
}

pub struct ModelService {
    pool: PgPool,
}

impl ModelService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

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
        max_tokens: i32,
        is_vision: bool,
        is_tool: bool,
    ) -> Result<ModelView, AuthError> {
        validate_model(name, owner, model_type, base_url, api_key)?;
        let key = Uuid::new_v4();
        let res = sqlx::query(
            r#"INSERT INTO api_models
               (key, name, owner, model_type, base_url, api_key, capabilities,
                speed, rating, max_tokens, is_vision, is_tool)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
        )
        .bind(key)
        .bind(name.trim())
        .bind(owner.trim())
        .bind(model_type.trim())
        .bind(base_url.trim())
        .bind(api_key.trim())
        .bind(&capabilities)
        .bind(speed)
        .bind(&rating)
        .bind(max_tokens)
        .bind(is_vision)
        .bind(is_tool)
        .execute(&self.pool)
        .await;
        if let Err(sqlx::Error::Database(db)) = &res
            && db.code().as_deref() == Some("23505")
        {
            return Err(AuthError::Conflict("model name taken".into()));
        }
        res?;
        Ok(row_to_view(self.fetch(key).await?))
    }

    pub async fn list(
        &self,
        search: Option<&str>,
        owner: Option<&str>,
        status: Option<i16>,
        model_type: Option<&str>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<ModelView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;

        let mut conds: Vec<String> = vec![];
        let mut binds: Vec<String> = vec![];

        if let Some(s) = search {
            if !s.trim().is_empty() {
                conds.push(format!("name ILIKE ${}", binds.len() + 1));
                binds.push(format!("%{}%", s.trim()));
            }
        }
        if let Some(o) = owner {
            if !o.trim().is_empty() {
                conds.push(format!("owner = ${}", binds.len() + 1));
                binds.push(o.trim().into());
            }
        }
        if let Some(st) = status {
            conds.push(format!("status = ${}", binds.len() + 1));
            binds.push(st.to_string());
        }
        if let Some(mt) = model_type {
            if !mt.trim().is_empty() {
                conds.push(format!("model_type = ${}", binds.len() + 1));
                binds.push(mt.trim().into());
            }
        }

        let where_clause = if conds.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conds.join(" AND "))
        };

        let count_sql = format!("SELECT count(*) FROM api_models WHERE 1=1{}", where_clause);
        let list_sql = format!(
            "SELECT {COLS} FROM api_models WHERE 1=1{} ORDER BY created_at DESC LIMIT ${} OFFSET ${}",
            where_clause,
            binds.len() + 1,
            binds.len() + 2
        );

        let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
        for a in &binds {
            count_q = count_q.bind(a);
        }
        let total: i64 = count_q.fetch_one(&self.pool).await?;

        let mut list_q = sqlx::query_as::<_, ModelRow>(&list_sql);
        for a in &binds {
            list_q = list_q.bind(a);
        }
        let rows: Vec<ModelRow> = list_q.bind(size).bind(offset).fetch_all(&self.pool).await?;

        Ok((rows.into_iter().map(row_to_view).collect(), total))
    }

    pub async fn get(&self, key: Uuid) -> Result<ModelView, AuthError> {
        Ok(row_to_view(self.fetch(key).await?))
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
        status: Option<i16>,
    ) -> Result<ModelView, AuthError> {
        let existing = self.fetch(key).await?;
        let mn = name.unwrap_or(&existing.name);
        let mo = owner.unwrap_or(&existing.owner);
        let mt = model_type.unwrap_or(&existing.model_type);
        let mu = base_url.unwrap_or(&existing.base_url);
        let mk = api_key.unwrap_or(&existing.api_key);
        validate_model(mn, mo, mt, mu, mk)?;

        sqlx::query(
            r#"UPDATE api_models SET
               name = COALESCE($2, name), owner = COALESCE($3, owner),
               model_type = COALESCE($4, model_type), base_url = COALESCE($5, base_url),
               api_key = COALESCE($6, api_key), capabilities = COALESCE($7, capabilities),
               speed = COALESCE($8, speed), rating = COALESCE($9, rating),
               max_tokens = COALESCE($10, max_tokens), is_vision = COALESCE($11, is_vision),
               is_tool = COALESCE($12, is_tool), status = COALESCE($13, status),
               updated_at = now() WHERE key = $1"#,
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
        .bind(status)
        .execute(&self.pool)
        .await?;

        Ok(row_to_view(self.fetch(key).await?))
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), AuthError> {
        let n = sqlx::query("DELETE FROM api_models WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AuthError::NotFound("model not found".into()));
        }
        Ok(())
    }

    pub async fn missing_models(&self) -> Result<Vec<String>, AuthError> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"WITH channel_models AS (
                   SELECT DISTINCT jsonb_array_elements(models)->>'alias' AS model_name
                   FROM api_channels WHERE status = 1
               )
               SELECT model_name FROM channel_models
               WHERE model_name <> '' AND model_name NOT IN (SELECT name FROM api_models WHERE status = 1)"#,
        )
        .fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    async fn fetch(&self, key: Uuid) -> Result<ModelRow, AuthError> {
        sqlx::query_as(&format!("SELECT {COLS} FROM api_models WHERE key = $1"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::NotFound("model not found".into()))
    }
}

fn validate_model(
    name: &str,
    owner: &str,
    model_type: &str,
    base_url: &str,
    api_key: &str,
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
        return Err(AuthError::BadRequest(
            "model_type must be one of chat/image/audio/video/text/embedding".into(),
        ));
    }
    let url = base_url.trim();
    if !url.is_empty() && !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(AuthError::BadRequest(
            "base_url must be http(s) URL if provided".into(),
        ));
    }
    if api_key.trim().is_empty() {
        return Err(AuthError::BadRequest("api_key required".into()));
    }
    Ok(())
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct ModelAppState {
    pub svc: std::sync::Arc<ModelService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: ModelAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/models", get(list).post(create))
        .route("/api/models/search", get(search))
        .route("/api/models/missing", get(missing))
        .route("/api/models/:key", get(get_one).put(update).delete(remove))
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

type ErrResp = (StatusCode, Json<Value>);
fn err_json(e: AuthError) -> ErrResp {
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}
fn ok_json(v: Value) -> Result<Json<Value>, ErrResp> {
    Ok(Json(v))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    search: Option<String>,
    owner: Option<String>,
    status: Option<i16>,
    model_type: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateModelRequest {
    name: String,
    owner: String,
    #[serde(default = "default_mt")]
    model_type: String,
    #[serde(default)]
    base_url: String,
    api_key: String,
    #[serde(default)]
    capabilities: Value,
    #[serde(default)]
    speed: i32,
    #[serde(default)]
    rating: Value,
    #[serde(default)]
    max_tokens: i32,
    #[serde(default)]
    is_vision: bool,
    #[serde(default)]
    is_tool: bool,
}
fn default_mt() -> String {
    "chat".into()
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
    status: Option<i16>,
}

async fn parse_key(key: &str) -> Result<Uuid, ErrResp> {
    Uuid::parse_str(key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))
}

async fn list(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let (items, total) = s
        .svc
        .list(
            q.search.as_deref(),
            q.owner.as_deref(),
            q.status,
            q.model_type.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
        .map_err(err_json)?;
    ok_json(json!({"items": items, "total": total}))
}

async fn search(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let (items, total) = s
        .svc
        .list(
            q.search.as_deref(),
            q.owner.as_deref(),
            q.status,
            q.model_type.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
        .map_err(err_json)?;
    ok_json(json!({"items": items, "total": total}))
}

async fn missing(State(s): State<ModelAppState>, h: HeaderMap) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .missing_models()
        .await
        .map(|items| Json(json!({"items": items})))
        .map_err(err_json)
}

async fn create(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Json(req): Json<CreateModelRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .create(
            &req.name,
            &req.owner,
            &req.model_type,
            &req.base_url,
            &req.api_key,
            req.capabilities.clone(),
            req.speed,
            req.rating.clone(),
            req.max_tokens,
            req.is_vision,
            req.is_tool,
        )
        .await
        .map(|m| Json(json!(m)))
        .map_err(err_json)
}

async fn get_one(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
        .get(key)
        .await
        .map(|m| Json(json!(m)))
        .map_err(err_json)
}

async fn update(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<UpdateModelRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
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
            req.status,
        )
        .await
        .map(|m| Json(json!(m)))
        .map_err(err_json)
}

async fn remove(
    State(s): State<ModelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc.delete(key).await.map_err(err_json)?;
    ok_json(json!({"success": true}))
}
