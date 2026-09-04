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
    #[allow(clippy::too_many_arguments)]
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
        validate(
            merged_name,
            merged_type,
            merged_url,
            &merged_keys,
            &merged_models,
        )?;

        let cur_keys =
            serde_json::to_value(&merged_keys).map_err(|e| AuthError::Crypto(e.to_string()))?;
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
        let affected =
            sqlx::query("UPDATE api_channels SET status = $2, updated_at = now() WHERE key = $1")
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

    /// 渠道可用模型列表（从 api_models 读）
    pub async fn channel_models(&self) -> Result<Vec<String>, AuthError> {
        let sql = "SELECT name FROM api_models WHERE status = 1 ORDER BY name";
        let rows: Vec<(String,)> = sqlx::query_as(sql).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|(name,)| name).collect())
    }

    /// 渠道余额查询 (stub)
    pub async fn update_balance(&self) -> Result<serde_json::Value, AuthError> {
        // TODO: 实现真实余额查询，从上游拉取 usage/credit
        Ok(serde_json::json!({"total_balance": 0.0, "channels": []}))
    }

    /// 按标签批量停用
    pub async fn batch_disable_by_tag(&self, tag: &str) -> Result<usize, AuthError> {
        let sql = "UPDATE api_channels SET status = 0 WHERE JSONB_EXISTS(tags, $1)";
        let affected = sqlx::query(sql)
            .bind(serde_json::json!(format!("/\"{}\"/", tag)))
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(affected as usize)
    }

    /// 按标签批量启用
    pub async fn batch_enable_by_tag(&self, tag: &str) -> Result<usize, AuthError> {
        let sql = "UPDATE api_channels SET status = 1 WHERE JSONB_EXISTS(tags, $1)";
        let affected = sqlx::query(sql)
            .bind(serde_json::json!(format!("/\"{}\"/", tag)))
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(affected as usize)
    }

    /// 编辑标签
    pub async fn update_tag(&self, key: Uuid, tag: &str) -> Result<String, AuthError> {
        let existing = self.fetch(key).await?;
        let mut tags: Vec<String> = serde_json::from_value(existing.tags.clone()).unwrap_or_default();
        if let Some(pos) = tags.iter().position(|t| t == tag) {
            tags.remove(pos);
        } else {
            tags.push(tag.to_string());
        }
        let new_tags = serde_json::to_value(&tags).map_err(|e| AuthError::Crypto(e.to_string()))?;
        sqlx::query(
            "UPDATE api_channels SET tags = $2, updated_at = now() WHERE key = $1"
        )
        .bind(key)
        .bind(new_tags)
        .execute(&self.pool)
        .await?;
        Ok(tag.to_string())
    }

    /// 批量删除
    pub async fn batch_delete(&self, keys: &[Uuid]) -> Result<usize, AuthError> {
        if keys.is_empty() {
            return Ok(0);
        }
        let mut query = "DELETE FROM api_channels WHERE key IN (".to_string();
        for _ in keys {
            query.push("$1,");
        }
        query.pop(); // remove trailing comma
        query.push_str(")");
        let mut q = sqlx::query(&query);
        for k in keys {
            q = q.bind(*k);
        }
        let affected = q.execute(&self.pool).await?.rows_affected();
        Ok(affected as usize)
    }

    /// 拉取上游模型 (stub)
    pub async fn fetch_models(&self, _payload: Value) -> Result<Value, AuthError> {
        // TODO: 实现从 forward crate 读取上游模型列表
        Ok(serde_json::json!([]))
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

// ---------- 渠道探活 ----------

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
    /// 对单个渠道发真实测试请求（chat/completions 短输出），结果落
    /// monitor_history 并返回。model 缺省取渠道 test_model，再缺省取
    /// models 里第一个 upstream。
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
            .map(str::to_string)
            .or_else(|| ch.test_model.clone())
            .or_else(|| {
                models
                    .first()
                    .and_then(|m| m.get("upstream"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .ok_or_else(|| AuthError::BadRequest("channel has no test_model or models".into()))?;

        let first_key = keys
            .first()
            .ok_or_else(|| AuthError::BadRequest("channel has no keys".into()))?;

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

    /// 全量探活：逐个渠道测试（串行，渠道多时调用方自行分批）。
    pub async fn test_all(
        &self,
        monitor: &observe::monitor::MonitorDeps,
    ) -> Result<Vec<ProbeResult>, AuthError> {
        let sql = format!(
            "SELECT {} FROM api_channels WHERE status = 1 ORDER BY priority DESC",
            SELECT_COLS
        );
        let rows: Vec<ChannelRow> = sqlx::query_as(&sql).fetch_all(&self.pool).await?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            match self.test_channel(monitor, r.key, None).await {
                Ok(p) => out.push(p),
                // 无 test_model/keys 的渠道记一条失败结果而不是中断全量
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

/// 探活底层 HTTP 调用 — POST {base_url}/chat/completions，max_tokens=1。
/// 超时 10s；HTTP 2xx 视为探活成功（body 不校验内容）。
async fn probe_chat_completions(base_url: &str, api_key: &str, model: &str) -> ProbeResult {
    let started = std::time::Instant::now();
    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    let client = match client {
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

    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 1
    });

    let resp = client
        .post(&url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await;

    let latency_ms = started.elapsed().as_millis() as i32;
    match resp {
        Ok(r) => {
            let status = r.status().as_u16() as i32;
            let ok = r.status().is_success();
            let message = if ok {
                String::new()
            } else {
                r.text()
                    .await
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect()
            };
            ProbeResult {
                channel_key: String::new(),
                channel_name: String::new(),
                model: model.into(),
                ok,
                status_code: Some(status),
                latency_ms,
                error_kind: if ok { String::new() } else { "http".into() },
                message,
            }
        }
        Err(e) => {
            let kind = if e.is_timeout() {
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
                error_kind: kind.into(),
                message: e.to_string(),
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct TestQuery {
    model: Option<String>,
}

/// POST /api/channel/{key}/test — 单渠道探活。
async fn test_one(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
    axum::extract::Query(q): axum::extract::Query<TestQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    match state
        .svc
        .test_channel(&state.monitor, key, q.model.as_deref())
        .await
    {
        Ok(p) => Ok(axum::Json(serde_json::json!(p))),
        Err(e) => Err(err_json(e)),
    }
}

/// POST /api/channel/test — 全量探活（启用渠道串行测试）。
async fn test_all(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state.svc.test_all(&state.monitor).await {
        Ok(items) => Ok(axum::Json(serde_json::json!({"items": items}))),
        Err(e) => Err(err_json(e)),
    }
}

// ---------- axum 路由 (admin) ----------

#[derive(Clone)]
pub struct ChannelAppState {
    pub svc: std::sync::Arc<ChannelService>,
    pub auth: std::sync::Arc<AuthService>,
    pub monitor: observe::monitor::MonitorDeps,
}

pub fn router(state: ChannelAppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/channel", get(list).post(create))
        .route("/api/channel/search", get(search))
        .route(
            "/api/channel/{key}",
            get(get_one).put(update).delete(remove),
        )
        .route("/api/channel/{key}/status", post(set_status))
        .route("/api/channel/{key}/test", post(test_one))
        .route("/api/channel/test", post(test_all))
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
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, headers)
        .await
        .map_err(err_json)?;
    match state.svc.list(search, page, size).await {
        Ok((items, total)) => Ok(axum::Json(
            serde_json::json!({"items": items, "total": total}),
        )),
        Err(e) => Err(err_json(e)),
    }
}

async fn list(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    handle_list(
        &state,
        &headers,
        q.search.as_deref(),
        q.page.unwrap_or(1),
        q.size.unwrap_or(20),
    )
    .await
}

async fn search(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Query(q): axum::extract::Query<ListQuery>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    let keyword = q.keyword.clone().or(q.search.clone()).unwrap_or_default();
    handle_list(
        &state,
        &headers,
        Some(&keyword),
        q.page.unwrap_or(1),
        q.size.unwrap_or(20),
    )
    .await
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
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    match state
        .svc
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
    {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

async fn get_one(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
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
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
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
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;
    let key = Uuid::parse_str(&key)
        .map_err(|_| AuthError::BadRequest("invalid key".into()))
        .map_err(err_json)?;
    match state.svc.set_status(key, req.status).await {
        Ok(c) => Ok(axum::Json(serde_json::json!(c))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    axum::extract::State(state): axum::extract::State<ChannelAppState>,
    headers: HeaderMap,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<axum::Json<serde_json::Value>, (axum::http::StatusCode, axum::Json<serde_json::Value>)>
{
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