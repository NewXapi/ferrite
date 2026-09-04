//! 令牌生命周期 — 单机版平表实现 (api_tokens)。
//!
//! 替代原 store-trait 骨架 (sync/outbox 设计推迟)。
//! 参考: new-api controller/token.go + wildtoken api_tokens (hash + preview + 一次性明文)。
//!
//! - 明文 key = "sk-" + 64 hex (32B random), 只在创建响应出现一次
//! - 库存 sha256(明文) hex; 查表 (gateway identity) 用 key_hash
//! - preview = "sk-ab****ef" 列表展示用

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS api_tokens (
    key             UUID PRIMARY KEY,
    user_key        UUID NOT NULL,
    name            TEXT NOT NULL,
    key_hash        TEXT UNIQUE NOT NULL,
    key_preview     TEXT NOT NULL DEFAULT '',
    group_id        TEXT,
    quota           BIGINT NOT NULL DEFAULT 0,
    unlimited_quota BOOLEAN NOT NULL DEFAULT false,
    used_quota      BIGINT NOT NULL DEFAULT 0,
    expires_at      TIMESTAMPTZ,
    status          SMALLINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_api_tokens_user_key ON api_tokens(user_key);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenView {
    pub key: String,
    pub user_key: String,
    pub name: String,
    pub key_preview: String,
    pub group: Option<String>,
    pub quota: i64,
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub expires_at: Option<DateTime<Utc>>,
    pub status: i16,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTokenResult {
    /// 明文 key — 只在创建响应出现一次
    pub plaintext: String,
    pub token: TokenView,
}

pub struct TokenService {
    pool: PgPool,
}

impl TokenService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(
        &self,
        user_key: Uuid,
        name: &str,
        group: Option<String>,
        quota: i64,
        unlimited_quota: bool,
        expires_at: Option<DateTime<Utc>>,
    ) -> Result<CreateTokenResult, AuthError> {
        if name.trim().is_empty() {
            return Err(AuthError::BadRequest("token name required".into()));
        }
        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        let plaintext = format!("sk-{}", hex::encode(buf));
        let key_hash = sha256_hex(&plaintext);
        let key_preview = preview(&plaintext);
        let key = Uuid::new_v4();

        sqlx::query(
            r#"INSERT INTO api_tokens
               (key, user_key, name, key_hash, key_preview, group_id,
                quota, unlimited_quota, expires_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"#,
        )
        .bind(key)
        .bind(user_key)
        .bind(name.trim())
        .bind(&key_hash)
        .bind(key_preview)
        .bind(group)
        .bind(quota)
        .bind(unlimited_quota)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        Ok(CreateTokenResult {
            plaintext,
            token: self.get(user_key, key, true).await?,
        })
    }

    /// admin_all=true 时跨用户列出 (admin), 否则只看自己的。
    pub async fn list(&self, user_key: Uuid, admin_all: bool) -> Result<Vec<TokenView>, AuthError> {
        let rows: Vec<TokenRow> = if admin_all {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens ORDER BY created_at DESC"#,
            )
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens WHERE user_key = $1 ORDER BY created_at DESC"#,
            )
            .bind(user_key)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn search(
        &self,
        user_key: Uuid,
        admin_all: bool,
        keyword: &str,
    ) -> Result<Vec<TokenView>, AuthError> {
        let pattern = format!("%{}%", keyword.trim());
        let rows: Vec<TokenRow> = if admin_all {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens WHERE name ILIKE $1 ORDER BY created_at DESC"#,
            )
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens WHERE user_key = $1 AND name ILIKE $2
                   ORDER BY created_at DESC"#,
            )
            .bind(user_key)
            .bind(&pattern)
            .fetch_all(&self.pool)
            .await?
        };
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get(
        &self,
        user_key: Uuid,
        token_key: Uuid,
        admin_all: bool,
    ) -> Result<TokenView, AuthError> {
        let row: TokenRow = if admin_all {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens WHERE key = $1"#,
            )
            .bind(token_key)
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as(
                r#"SELECT key, user_key, name, key_preview, group_id, quota,
                          unlimited_quota, used_quota, expires_at, status, created_at
                   FROM api_tokens WHERE key = $1 AND user_key = $2"#,
            )
            .bind(token_key)
            .bind(user_key)
            .fetch_optional(&self.pool)
            .await?
        }
        .ok_or(AuthError::NotFound("token not found".into()))?;
        Ok(row.into())
    }
    /// 重取明文 key — 重新生成（旧 key 立即失效，新明文一次性返回）。
    /// 对齐 new-api GetTokenKey 语义：不做可逆加密存储。
    pub async fn regenerate_key(
        &self,
        user_key: Uuid,
        token_key: Uuid,
        admin_all: bool,
    ) -> Result<String, AuthError> {
        // 归属校验 (owner or admin)
        self.get(user_key, token_key, admin_all).await?;

        let mut buf = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut buf);
        let plaintext = format!("sk-{}", hex::encode(buf));
        let key_hash = sha256_hex(&plaintext);
        let key_preview = preview(&plaintext);

        let affected = sqlx::query(
            "UPDATE api_tokens SET key_hash = $2, key_preview = $3, updated_at = now() WHERE key = $1",
        )
        .bind(token_key)
        .bind(&key_hash)
        .bind(key_preview)
        .execute(&self.pool)
        .await?
        .rows_affected();
        if affected == 0 {
            return Err(AuthError::UserNotFound);
        }
        Ok(plaintext)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn update(
        &self,
        user_key: Uuid,
        token_key: Uuid,
        admin_all: bool,
        name: Option<&str>,
        group: Option<Option<String>>,
        quota: Option<i64>,
        unlimited_quota: Option<bool>,
        expires_at: Option<Option<DateTime<Utc>>>,
        status: Option<i16>,
    ) -> Result<TokenView, AuthError> {
        // 归属校验 (owner or admin)
        self.get(user_key, token_key, admin_all).await?;

        if let Some(name) = name {
            if name.trim().is_empty() {
                return Err(AuthError::BadRequest("token name required".into()));
            }
            sqlx::query("UPDATE api_tokens SET name = $2, updated_at = now() WHERE key = $1")
                .bind(token_key)
                .bind(name.trim())
                .execute(&self.pool)
                .await?;
        }
        if let Some(g) = group {
            sqlx::query("UPDATE api_tokens SET group_id = $2, updated_at = now() WHERE key = $1")
                .bind(token_key)
                .bind(g)
                .execute(&self.pool)
                .await?;
        }
        if let Some(q) = quota {
            sqlx::query("UPDATE api_tokens SET quota = $2, updated_at = now() WHERE key = $1")
                .bind(token_key)
                .bind(q)
                .execute(&self.pool)
                .await?;
        }
        if let Some(u) = unlimited_quota {
            sqlx::query(
                "UPDATE api_tokens SET unlimited_quota = $2, updated_at = now() WHERE key = $1",
            )
            .bind(token_key)
            .bind(u)
            .execute(&self.pool)
            .await?;
        }
        if let Some(e) = expires_at {
            sqlx::query("UPDATE api_tokens SET expires_at = $2, updated_at = now() WHERE key = $1")
                .bind(token_key)
                .bind(e)
                .execute(&self.pool)
                .await?;
        }
        if let Some(s) = status {
            if ![1, 2].contains(&s) {
                return Err(AuthError::BadRequest("status must be 1|2".into()));
            }
            sqlx::query("UPDATE api_tokens SET status = $2, updated_at = now() WHERE key = $1")
                .bind(token_key)
                .bind(s)
                .execute(&self.pool)
                .await?;
        }
        self.get(user_key, token_key, admin_all).await
    }

    pub async fn delete(
        &self,
        user_key: Uuid,
        token_key: Uuid,
        admin_all: bool,
    ) -> Result<(), AuthError> {
        // 归属校验 (owner or admin)
        self.get(user_key, token_key, admin_all).await?;
        sqlx::query("DELETE FROM api_tokens WHERE key = $1")
            .bind(token_key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// 可自动分配的分组（启用状态，按名称排序）。
    pub async fn auto_groups(&self) -> Result<Vec<String>, AuthError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM api_groups WHERE status = 1 ORDER BY name")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    /// 批量创建 token（逐个复用 create，任一失败整体报错）。
    pub async fn batch_create(
        &self,
        user_key: Uuid,
        items: Vec<NewToken>,
    ) -> Result<Vec<CreateTokenResult>, AuthError> {
        let mut out = Vec::with_capacity(items.len());
        for it in items {
            out.push(
                self.create(
                    user_key,
                    &it.name,
                    it.group,
                    it.quota,
                    it.unlimited_quota,
                    it.expires_at,
                )
                .await?,
            );
        }
        Ok(out)
    }

    /// 批量重取明文 key（逐个复用 regenerate_key；key 失效仅发生在调用时）。
    pub async fn batch_keys(
        &self,
        user_key: Uuid,
        token_keys: &[Uuid],
        admin_all: bool,
    ) -> Result<Vec<(Uuid, String)>, AuthError> {
        let mut out = Vec::with_capacity(token_keys.len());
        for &tk in token_keys {
            let plaintext = self.regenerate_key(user_key, tk, admin_all).await?;
            out.push((tk, plaintext));
        }
        Ok(out)
    }
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn preview(plaintext: &str) -> String {
    let body = plaintext.strip_prefix("sk-").unwrap_or(plaintext);
    let (head, tail) = (&body[..4], &body[body.len().saturating_sub(4)..]);
    format!("sk-{head}****{tail}")
}

#[derive(Debug, FromRow)]
struct TokenRow {
    key: Uuid,
    user_key: Uuid,
    name: String,
    key_preview: String,
    group_id: Option<String>,
    quota: i64,
    unlimited_quota: bool,
    used_quota: i64,
    expires_at: Option<DateTime<Utc>>,
    status: i16,
    created_at: DateTime<Utc>,
}

impl From<TokenRow> for TokenView {
    fn from(r: TokenRow) -> Self {
        Self {
            key: r.key.to_string(),
            user_key: r.user_key.to_string(),
            name: r.name,
            key_preview: r.key_preview,
            group: r.group_id,
            quota: r.quota,
            unlimited_quota: r.unlimited_quota,
            used_quota: r.used_quota,
            expires_at: r.expires_at,
            status: r.status,
            created_at: r.created_at,
        }
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct TokenAppState {
    pub svc: std::sync::Arc<TokenService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: TokenAppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/token", get(list).post(create))
        .route("/api/token/search", get(search))
        .route("/api/token/auto-groups", get(auto_groups))
        .route("/api/token/batch", post(batch_create))
        .route("/api/token/batch/keys", post(batch_keys))
        .route("/api/token/{key}", get(get_one).put(update).delete(remove))
        .route("/api/token/{key}/key", post(regenerate))
        .with_state(state)
}

async fn current_admin(auth: &AuthService, headers: &HeaderMap) -> Result<(Uuid, bool), AuthError> {
    let user = bearer_user(auth, headers).await?;
    let key = Uuid::parse_str(&user.key).map_err(|_| AuthError::InvalidToken)?;
    Ok((key, user.role >= 10))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateTokenRequest {
    name: String,
    group: Option<String>,
    #[serde(default)]
    quota: i64,
    #[serde(default)]
    unlimited_quota: bool,
    expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewToken {
    pub name: String,
    pub group: Option<String>,
    #[serde(default)]
    pub quota: i64,
    #[serde(default)]
    pub unlimited_quota: bool,
    pub expires_at: Option<DateTime<Utc>>,
}

async fn create(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<CreateTokenRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, _) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state
        .svc
        .create(
            user_key,
            &req.name,
            req.group,
            req.quota,
            req.unlimited_quota,
            req.expires_at,
        )
        .await
    {
        Ok(r) => Ok(axum::Json(serde_json::json!(r))),
        Err(e) => Err(err_json(e)),
    }
}

async fn get_one(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let token_key = Uuid::parse_str(&key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid token key".into())))?;
    match state.svc.get(user_key, token_key, is_admin).await {
        Ok(t) => Ok(axum::Json(serde_json::json!(t))),
        Err(e) => Err(err_json(e)),
    }
}

async fn auto_groups(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (_, _) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.auto_groups().await {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
struct BatchCreateRequest {
    #[serde(default)]
    items: Vec<NewToken>,
}

async fn batch_create(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<BatchCreateRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, _) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.batch_create(user_key, req.items).await {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
struct BatchKeysRequest {
    #[serde(default)]
    keys: Vec<String>,
}

async fn batch_keys(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::Json(req): axum::Json<BatchKeysRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let token_keys: Vec<Uuid> = req
        .keys
        .iter()
        .map(|k| {
            Uuid::parse_str(k).map_err(|_| AuthError::BadRequest("invalid token key".into()))
        })
        .collect::<Result<_, _>>()
        .map_err(err_json)?;
    match state.svc.batch_keys(user_key, &token_keys, is_admin).await {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    #[serde(default)]
    all: bool,
    keyword: Option<String>,
}

async fn list(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    // all=true 需要 admin
    if q.all && !is_admin {
        return Err(err_json(AuthError::Forbidden));
    }
    match state.svc.list(user_key, q.all && is_admin).await {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

async fn search(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let keyword = q.keyword.unwrap_or_default();
    if q.all && !is_admin {
        return Err(err_json(AuthError::Forbidden));
    }
    match state
        .svc
        .search(user_key, q.all && is_admin, &keyword)
        .await
    {
        Ok(items) => Ok(axum::Json(serde_json::json!({ "items": items }))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateTokenRequest {
    name: Option<String>,
    /// None = 不改; Some(inner) = 改 (inner None = 跟随用户组)
    group: Option<Option<String>>,
    quota: Option<i64>,
    unlimited_quota: Option<bool>,
    expires_at: Option<Option<DateTime<Utc>>>,
    status: Option<i16>,
}

async fn update(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::Json(req): axum::Json<UpdateTokenRequest>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let token_key = Uuid::parse_str(&key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid token key".into())))?;
    match state
        .svc
        .update(
            user_key,
            token_key,
            is_admin,
            req.name.as_deref(),
            req.group,
            req.quota,
            req.unlimited_quota,
            req.expires_at,
            req.status,
        )
        .await
    {
        Ok(t) => Ok(axum::Json(serde_json::json!(t))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let token_key = Uuid::parse_str(&key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid token key".into())))?;
    match state.svc.delete(user_key, token_key, is_admin).await {
        Ok(()) => Ok(axum::Json(serde_json::json!({"success": true}))),
        Err(e) => Err(err_json(e)),
    }
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, axum::Json<serde_json::Value>) {
    (
        e.status(),
        axum::Json(serde_json::json!({
            "code": e.code(),
            "message": e.to_string(),
        })),
    )
}

/// POST /api/token/{key}/key — 重新生成明文 key（旧 key 失效）。
async fn regenerate(
    axum::extract::State(state): axum::extract::State<TokenAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let (user_key, is_admin) = current_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let token_key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid token key".into()))
        .map_err(err_json)?;
    match state
        .svc
        .regenerate_key(user_key, token_key, is_admin)
        .await
    {
        Ok(plaintext) => Ok(axum::Json(serde_json::json!({ "key": plaintext }))),
        Err(e) => Err(err_json(e)),
    }
}
