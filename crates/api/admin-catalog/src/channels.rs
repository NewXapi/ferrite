//! 渠道管理 — 单机版平表实现 (api_channels)。
//!
//! 替代原 store-trait 骨架 (sync/outbox 设计推迟)。
//! 字段覆盖 gateway dispatch::ChannelConfig 所需，
//! apps/api 迁移读这张表后 kv_store JSON blob 可废弃。
//!
//! - keys: JSONB 字符串数组 (明文 key; 列表接口掩码，单查返回)
//! - models: JSONB 数组 [{alias, upstream}]
//! - tags: JSONB 字符串数组 (渠道分组/标签)

#![allow(clippy::too_many_arguments)] // ponytail: CRUD 参数逐一对应表列，包 struct 只是搬参数位置
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;
pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    // 迁移：旧表补 tags 列（必须在 CREATE INDEX 之前）
    sqlx::raw_sql(
        "ALTER TABLE api_channels ADD COLUMN IF NOT EXISTS tags JSONB NOT NULL DEFAULT '[]'",
    )
    .execute(pool)
    .await?;
    const DDL: &str = r#"
CREATE TABLE IF NOT EXISTS api_channels (
    key           UUID PRIMARY KEY,
    name          TEXT UNIQUE NOT NULL,
    channel_type  TEXT NOT NULL DEFAULT 'openai',
    base_url      TEXT NOT NULL DEFAULT '',
    keys          JSONB NOT NULL DEFAULT '[]',
    models        JSONB NOT NULL DEFAULT '[]',
    group_name    TEXT NOT NULL DEFAULT 'default',
    priority      INT  NOT NULL DEFAULT 0,
    weight        INT  NOT NULL DEFAULT 0,
    status        SMALLINT NOT NULL DEFAULT 1,
    tags          JSONB NOT NULL DEFAULT '[]',
    test_model    TEXT,
    remark        TEXT NOT NULL DEFAULT '',
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_api_channels_tags ON api_channels USING GIN (tags);
"#;
    sqlx::raw_sql(DDL).execute(pool).await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelView {
    pub key: String,
    pub name: String,
    pub channel_type: String,
    pub base_url: String,
    pub key_count: i64,
    pub keys: Option<Vec<String>>,
    pub models: Value,
    pub group_name: String,
    pub priority: i32,
    pub weight: i32,
    pub status: i16,
    pub tags: Value,
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
    tags: Value,
    test_model: Option<String>,
    remark: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

const SELECT_COLS: &str = "key, name, channel_type, base_url, keys, models, group_name, \
     priority, weight, status, tags, test_model, remark, created_at, updated_at";

fn row_to_view(r: ChannelRow, include_keys: bool) -> ChannelView {
    let keys: Vec<String> = serde_json::from_value(r.keys.clone()).unwrap_or_else(|e| {
        tracing::warn!(key = %r.key, error = %e, "corrupt keys JSONB — treating as empty");
        Vec::new()
    });
    let masked: Vec<String> = keys.iter().map(|k| mask_key(k)).collect();
    ChannelView {
        key: r.key.to_string(),
        key_count: keys.len() as i64,
        keys: if include_keys { Some(masked) } else { None },
        name: r.name,
        channel_type: r.channel_type,
        base_url: r.base_url,
        models: r.models,
        group_name: r.group_name,
        priority: r.priority,
        weight: r.weight,
        status: r.status,
        tags: r.tags,
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
            "SELECT {SELECT_COLS} FROM api_channels WHERE name ILIKE $1 OR base_url ILIKE $1 \
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
        let mn = name.unwrap_or(&existing.name);
        let mt = channel_type.unwrap_or(&existing.channel_type);
        let mu = base_url.unwrap_or(&existing.base_url);
        let mk: Vec<String> = match &keys {
            Some(k) => k.clone(),
            None => serde_json::from_value(existing.keys.clone()).unwrap_or_default(),
        };
        let mm = models.clone().unwrap_or_else(|| existing.models.clone());
        validate(mn, mt, mu, &mk, &mm)?;
        let cur_keys = serde_json::to_value(&mk).map_err(|e| AuthError::Crypto(e.to_string()))?;
        sqlx::query(
            r#"UPDATE api_channels SET
               name = COALESCE($2, name), channel_type = COALESCE($3, channel_type),
               base_url = COALESCE($4, base_url), keys = COALESCE($5, keys),
               models = COALESCE($6, models), group_name = COALESCE($7, group_name),
               priority = COALESCE($8, priority), weight = COALESCE($9, weight),
               test_model = $10, remark = COALESCE($11, remark),
               status = COALESCE($12, status), updated_at = now() WHERE key = $1"#,
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
        let n =
            sqlx::query("UPDATE api_channels SET status = $2, updated_at = now() WHERE key = $1")
                .bind(key)
                .bind(status)
                .execute(&self.pool)
                .await?
                .rows_affected();
        if n == 0 {
            return Err(AuthError::NotFound("channel not found".into()));
        }
        Ok(row_to_view(self.fetch(key).await?, true))
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), AuthError> {
        let n = sqlx::query("DELETE FROM api_channels WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AuthError::NotFound("channel not found".into()));
        }
        Ok(())
    }

    pub async fn channel_models(&self) -> Result<Vec<String>, AuthError> {
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM api_models WHERE status = 1 ORDER BY name")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows.into_iter().map(|(n,)| n).collect())
    }

    pub async fn update_balance(&self) -> Result<Value, AuthError> {
        Ok(json!({"total_balance": 0.0, "channels": []}))
    }

    pub async fn batch_disable_by_tag(&self, tag: &str) -> Result<usize, AuthError> {
        let n = sqlx::query("UPDATE api_channels SET status = 0 WHERE tags @> $1::jsonb")
            .bind(json!([tag]))
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(n as usize)
    }

    pub async fn batch_enable_by_tag(&self, tag: &str) -> Result<usize, AuthError> {
        let n = sqlx::query("UPDATE api_channels SET status = 1 WHERE tags @> $1::jsonb")
            .bind(json!([tag]))
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(n as usize)
    }

    pub async fn update_tag(&self, key: Uuid, tag: &str) -> Result<Value, AuthError> {
        let existing = self.fetch(key).await?;
        let mut tags: Vec<String> =
            serde_json::from_value(existing.tags.clone()).unwrap_or_default();
        if let Some(pos) = tags.iter().position(|t| t == tag) {
            tags.remove(pos);
        } else {
            tags.push(tag.to_string());
        }
        let new_tags = serde_json::to_value(&tags).map_err(|e| AuthError::Crypto(e.to_string()))?;
        let n = sqlx::query("UPDATE api_channels SET tags = $2, updated_at = now() WHERE key = $1")
            .bind(key)
            .bind(&new_tags)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AuthError::NotFound("channel not found".into()));
        }
        Ok(new_tags)
    }

    pub async fn batch_delete(&self, keys: &[Uuid]) -> Result<usize, AuthError> {
        if keys.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "DELETE FROM api_channels WHERE {}",
            (0..keys.len())
                .map(|i| format!("key = ${}", i + 1))
                .collect::<Vec<_>>()
                .join(" OR ")
        );
        let mut q = sqlx::query(&sql);
        for k in keys {
            q = q.bind(*k);
        }
        Ok(q.execute(&self.pool).await?.rows_affected() as usize)
    }

    pub async fn fetch_models(&self, _payload: Value) -> Result<Value, AuthError> {
        Ok(json!([]))
    }

    async fn fetch(&self, key: Uuid) -> Result<ChannelRow, AuthError> {
        let sql = format!("SELECT {SELECT_COLS} FROM api_channels WHERE key = $1");
        sqlx::query_as(&sql)
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::NotFound("channel not found".into()))
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

// ---------- 探活 ----------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProbeResult {
    pub channel_key: String,
    pub channel_name: String,
    pub model: String,
    pub ok: bool,
    pub status_code: Option<i32>,
    pub latency_ms: i32,
    pub error_kind: String,
    pub message: String,
}

impl ChannelService {
    pub async fn test_channel(
        &self,
        monitor: &observe::monitor::MonitorDeps,
        key: Uuid,
        model_override: Option<&str>,
    ) -> Result<ProbeResult, AuthError> {
        let ch = self.fetch(key).await?;
        let keys: Vec<String> = serde_json::from_value(ch.keys.clone()).unwrap_or_default();
        let models: Vec<Value> = serde_json::from_value(ch.models.clone()).unwrap_or_default();
        let model = model_override
            .map(String::from)
            .or_else(|| ch.test_model.clone())
            .or_else(|| {
                models
                    .first()
                    .and_then(|m| m.get("upstream"))
                    .and_then(|v| v.as_str().map(String::from))
            })
            .ok_or_else(|| AuthError::BadRequest("no test_model or models".into()))?;
        let first_key = keys
            .first()
            .ok_or_else(|| AuthError::BadRequest("no keys".into()))?;
        let result = probe_chat_completions(&ch.base_url, first_key, &model).await;
        let outcome = observe::monitor::ProbeOutcome {
            channel_key: ch.key,
            channel_name: ch.name.clone(),
            model: model.clone(),
            ok: result.ok,
            status_code: result.status_code,
            latency_ms: result.latency_ms,
            error_kind: result.error_kind.clone(),
            message: result.message.clone(),
        };
        monitor.record(&outcome).await?;
        Ok(ProbeResult {
            channel_key: ch.key.to_string(),
            channel_name: ch.name,
            model,
            ok: result.ok,
            status_code: result.status_code,
            latency_ms: result.latency_ms,
            error_kind: result.error_kind,
            message: result.message,
        })
    }

    pub async fn test_all(
        &self,
        monitor: &observe::monitor::MonitorDeps,
    ) -> Result<Vec<ProbeResult>, AuthError> {
        let sql = format!(
            "SELECT {SELECT_COLS} FROM api_channels WHERE status = 1 ORDER BY priority DESC"
        );
        let rows: Vec<ChannelRow> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            match self.test_channel(monitor, r.key, None).await {
                Ok(p) => out.push(p),
                Err(e) => out.push(ProbeResult {
                    channel_key: r.key.to_string(),
                    channel_name: r.name,
                    model: r.test_model.unwrap_or_default(),
                    ok: false,
                    status_code: None,
                    latency_ms: 0,
                    error_kind: "config".into(),
                    message: e.to_string(),
                }),
            }
        }
        Ok(out)
    }
}

async fn probe_chat_completions(base_url: &str, api_key: &str, model: &str) -> ProbeResult {
    let started = std::time::Instant::now();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProbeResult {
                channel_key: String::new(),
                channel_name: String::new(),
                model: model.into(),
                ok: false,
                status_code: None,
                latency_ms: 0,
                error_kind: "client".into(),
                message: e.to_string(),
            };
        }
    };
    let resp = client.post(&url).bearer_auth(api_key).json(&json!({ "model": model, "messages": [{"role":"user","content":"ping"}], "max_tokens": 1 })).send().await;
    let latency_ms = started.elapsed().as_millis() as i32;
    match resp {
        Ok(r) => {
            let s = r.status().as_u16() as i32;
            let ok = r.status().is_success();
            ProbeResult {
                channel_key: String::new(),
                channel_name: String::new(),
                model: model.into(),
                ok,
                status_code: Some(s),
                latency_ms,
                error_kind: if ok { String::new() } else { "http".into() },
                message: if ok {
                    String::new()
                } else {
                    r.text()
                        .await
                        .unwrap_or_default()
                        .chars()
                        .take(200)
                        .collect()
                },
            }
        }
        Err(e) => {
            let k = if e.is_timeout() {
                "timeout"
            } else if e.is_connect() {
                "connect"
            } else {
                "request"
            };
            ProbeResult {
                channel_key: String::new(),
                channel_name: String::new(),
                model: model.into(),
                ok: false,
                status_code: None,
                latency_ms,
                error_kind: k.into(),
                message: e.to_string(),
            }
        }
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct ChannelAppState {
    pub svc: std::sync::Arc<ChannelService>,
    pub auth: std::sync::Arc<AuthService>,
    pub monitor: observe::monitor::MonitorDeps,
}

pub fn router(state: ChannelAppState) -> axum::Router {
    use axum::routing::{get, post, put};
    axum::Router::new()
        .route("/api/channel", get(list).post(create))
        .route("/api/channel/search", get(search))
        .route(
            "/api/channel/{key}",
            get(get_one).put(update).delete(delete),
        )
        .route("/api/channel/{key}/status", post(set_status))
        .route("/api/channel/{key}/test", post(test_one))
        .route("/api/channel/test", post(test_all))
        .route("/api/channel/models", get(channel_models))
        .route("/api/channel/update_balance", get(update_balance))
        .route("/api/channel/tag/disabled", post(batch_disable_by_tag))
        .route("/api/channel/tag/enabled", post(batch_enable_by_tag))
        .route("/api/channel/tag", put(update_tag))
        .route("/api/channel/batch", post(batch_delete))
        .route("/api/channel/fetch_models", post(fetch_models))
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
    keyword: Option<String>,
    page: Option<i64>,
    size: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateChannelRequest {
    name: String,
    #[serde(default = "default_ct")]
    channel_type: String,
    #[serde(default)]
    base_url: String,
    keys: Vec<String>,
    #[serde(default)]
    models: Value,
    #[serde(default = "default_grp")]
    group_name: String,
    #[serde(default)]
    priority: i32,
    #[serde(default)]
    weight: i32,
    test_model: Option<String>,
    #[serde(default)]
    remark: String,
}
fn default_ct() -> String {
    "openai".into()
}
fn default_grp() -> String {
    "default".into()
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
#[derive(Debug, Deserialize)]
struct StatusRequest {
    status: i16,
}
#[derive(Debug, Deserialize)]
struct TestQuery {
    model: Option<String>,
}
#[derive(Debug, Deserialize)]
struct TagRequest {
    tag: String,
}
#[derive(Debug, Deserialize)]
struct BatchDeleteRequest {
    keys: Vec<String>,
}
#[derive(Debug, Deserialize)]
struct FetchModelsRequest {
    payload: Option<Value>,
}

async fn parse_key(key: &str) -> Result<Uuid, ErrResp> {
    Uuid::parse_str(key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))
}

async fn list(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let (items, total) = s
        .svc
        .list(
            q.search.as_deref(),
            q.page.unwrap_or(1),
            q.size.unwrap_or(20),
        )
        .await
        .map_err(err_json)?;
    ok_json(json!({"items": items, "total": total}))
}

async fn search(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let kw = q.keyword.clone().or(q.search.clone()).unwrap_or_default();
    let (items, total) = s
        .svc
        .list(Some(&kw), q.page.unwrap_or(1), q.size.unwrap_or(20))
        .await
        .map_err(err_json)?;
    ok_json(json!({"items": items, "total": total}))
}

async fn create(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Json(req): Json<CreateChannelRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .create(
            &req.name,
            &req.channel_type,
            &req.base_url,
            req.keys,
            req.models,
            &req.group_name,
            req.priority,
            req.weight,
            req.test_model,
            &req.remark,
        )
        .await
        .map(|c| Json(json!(c)))
        .map_err(err_json)
}

async fn get_one(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
        .get(key)
        .await
        .map(|c| Json(json!(c)))
        .map_err(err_json)
}

async fn update(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
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
        .map(|c| Json(json!(c)))
        .map_err(err_json)
}

async fn delete(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc.delete(key).await.map_err(err_json)?;
    ok_json(json!({"success": true}))
}

async fn set_status(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<StatusRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
        .set_status(key, req.status)
        .await
        .map(|c| Json(json!(c)))
        .map_err(err_json)
}

async fn test_one(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Query(q): Query<TestQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
        .test_channel(&s.monitor, key, q.model.as_deref())
        .await
        .map(|p| Json(json!(p)))
        .map_err(err_json)
}

async fn test_all(State(s): State<ChannelAppState>, h: HeaderMap) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .test_all(&s.monitor)
        .await
        .map(|items| Json(json!({"items": items})))
        .map_err(err_json)
}

async fn channel_models(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .channel_models()
        .await
        .map(|items| Json(json!({"items": items})))
        .map_err(err_json)
}

async fn update_balance(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc.update_balance().await.map(Json).map_err(err_json)
}

async fn batch_disable_by_tag(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Json(req): Json<TagRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .batch_disable_by_tag(&req.tag)
        .await
        .map(|n| Json(json!({"updated": n})))
        .map_err(err_json)
}

async fn batch_enable_by_tag(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Json(req): Json<TagRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .batch_enable_by_tag(&req.tag)
        .await
        .map(|n| Json(json!({"updated": n})))
        .map_err(err_json)
}

async fn update_tag(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<TagRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    s.svc
        .update_tag(key, &req.tag)
        .await
        .map(|tags| Json(json!({"tags": tags})))
        .map_err(err_json)
}

async fn batch_delete(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Json(req): Json<BatchDeleteRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let keys: Result<Vec<Uuid>, _> = req.keys.iter().map(|k| Uuid::parse_str(k)).collect();
    let keys = keys.map_err(|_| err_json(AuthError::BadRequest("invalid key in list".into())))?;
    s.svc
        .batch_delete(&keys)
        .await
        .map(|n| Json(json!({"deleted": n})))
        .map_err(err_json)
}

async fn fetch_models(
    State(s): State<ChannelAppState>,
    h: HeaderMap,
    Json(req): Json<FetchModelsRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    s.svc
        .fetch_models(req.payload.unwrap_or(json!({})))
        .await
        .map(Json)
        .map_err(err_json)
}

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
