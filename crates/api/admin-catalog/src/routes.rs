//! 路由单元管理 — one-api abilities 表语义的写侧（平表直连 sqlx）。
//!
//! route_units = (group, public_model) → (channel, key_index, upstream_model)
//! 的路由映射。dispatch 快照按此表建 (group, model) 候选集。
//!
//! 写前引用完整性（原 TODO(#425) 已实现）：
//! - channel_key 存在且 status=启用
//! - key_index < channel.keys.len()
//! - 渠道删除 → 该渠道的 route_units 级联失效（平表无 FK，写侧手工级联：
//!   channels::delete 后调用 `invalidate_by_channel`）

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        r#"CREATE TABLE IF NOT EXISTS route_units (
    key            UUID PRIMARY KEY,
    group_id       TEXT NOT NULL,
    public_model   TEXT NOT NULL,
    channel_key    UUID NOT NULL,
    key_index      INT  NOT NULL DEFAULT 0,
    upstream_model TEXT NOT NULL,
    priority       INT  NOT NULL DEFAULT 0,
    weight         INT  NOT NULL DEFAULT 10,
    status         SMALLINT NOT NULL DEFAULT 1,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_route_units_lookup ON route_units (group_id, public_model) WHERE status = 1;
CREATE INDEX IF NOT EXISTS idx_route_units_channel ON route_units (channel_key);"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RouteUnitView {
    pub key: String,
    pub group_id: String,
    pub public_model: String,
    pub channel_key: String,
    pub key_index: i32,
    pub upstream_model: String,
    pub priority: i32,
    pub weight: i32,
    pub status: i16,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct RouteUnitRow {
    key: Uuid,
    group_id: String,
    public_model: String,
    channel_key: Uuid,
    key_index: i32,
    upstream_model: String,
    priority: i32,
    weight: i32,
    status: i16,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    updated_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

const COLS: &str = "key, group_id, public_model, channel_key, key_index, upstream_model, priority, weight, status, created_at, updated_at";

fn row_to_view(r: RouteUnitRow) -> RouteUnitView {
    RouteUnitView {
        key: r.key.to_string(),
        group_id: r.group_id,
        public_model: r.public_model,
        channel_key: r.channel_key.to_string(),
        key_index: r.key_index,
        upstream_model: r.upstream_model,
        priority: r.priority,
        weight: r.weight,
        status: r.status,
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

pub struct RouteUnitService {
    pool: PgPool,
}

impl RouteUnitService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 写前引用完整性校验（原 TODO(#425)）：
    /// 渠道存在且启用、key_index 在 keys 范围内。
    async fn validate(&self, group: &str, public_model: &str, channel_key: Uuid, key_index: i32) -> Result<(), AuthError> {
        if group.trim().is_empty() || public_model.trim().is_empty() {
            return Err(AuthError::BadRequest("group and public_model required".into()));
        }
        let (channel_status, keys): (i16, Value) = sqlx::query_as(
            "SELECT status, keys FROM api_channels WHERE key = $1",
        )
        .bind(channel_key)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(AuthError::NotFound("channel not found".into()))?;
        if channel_status != 1 {
            return Err(AuthError::BadRequest("channel is disabled".into()));
        }
        let key_count = keys.as_array().map(|a| a.len()).unwrap_or(0) as i32;
        if key_index < 0 || key_index >= key_count {
            return Err(AuthError::BadRequest(format!(
                "key_index {key_index} out of range (channel has {key_count} keys)"
            )));
        }
        Ok(())
    }

    pub async fn create(
        &self,
        group_id: &str,
        public_model: &str,
        channel_key: Uuid,
        key_index: i32,
        upstream_model: &str,
        priority: i32,
        weight: i32,
    ) -> Result<RouteUnitView, AuthError> {
        self.validate(group_id, public_model, channel_key, key_index).await?;
        let key = Uuid::new_v4();
        sqlx::query(
            r#"INSERT INTO route_units
               (key, group_id, public_model, channel_key, key_index, upstream_model, priority, weight)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(key)
        .bind(group_id.trim())
        .bind(public_model.trim())
        .bind(channel_key)
        .bind(key_index)
        .bind(upstream_model.trim())
        .bind(priority)
        .bind(weight)
        .execute(&self.pool)
        .await?;
        Ok(row_to_view(self.fetch(key).await?))
    }

    pub async fn list(
        &self,
        group: Option<&str>,
        public_model: Option<&str>,
        page: i64,
        size: i64,
    ) -> Result<(Vec<RouteUnitView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;
        let mut conds: Vec<String> = vec![];
        let mut binds: Vec<String> = vec![];
        if let Some(g) = group {
            if !g.trim().is_empty() {
                conds.push(format!("group_id = ${}", binds.len() + 1));
                binds.push(g.trim().into());
            }
        }
        if let Some(m) = public_model {
            if !m.trim().is_empty() {
                conds.push(format!("public_model = ${}", binds.len() + 1));
                binds.push(m.trim().into());
            }
        }
        let w = if conds.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conds.join(" AND "))
        };
        let total: i64 = {
            let count_sql = format!("SELECT count(*) FROM route_units{w}");
            let mut q = sqlx::query_scalar::<_, i64>(&count_sql);
            for a in &binds {
                q = q.bind(a);
            }
            q.fetch_one(&self.pool).await?
        };
        let list_sql = format!(
            "SELECT {COLS} FROM route_units{w} ORDER BY priority DESC, created_at DESC LIMIT ${} OFFSET ${}",
            binds.len() + 1,
            binds.len() + 2
        );
        let mut lq = sqlx::query_as::<_, RouteUnitRow>(&list_sql);
        for a in &binds {
            lq = lq.bind(a);
        }
        let rows = lq.bind(size).bind(offset).fetch_all(&self.pool).await?;
        Ok((rows.into_iter().map(row_to_view).collect(), total))
    }

    pub async fn update(
        &self,
        key: Uuid,
        group_id: Option<&str>,
        public_model: Option<&str>,
        key_index: Option<i32>,
        upstream_model: Option<&str>,
        priority: Option<i32>,
        weight: Option<i32>,
        status: Option<i16>,
    ) -> Result<RouteUnitView, AuthError> {
        let existing = self.fetch(key).await?;
        // 变更了 channel/key_index 需要重新校验（channel 不允许换）
        if key_index.is_some() || group_id.is_some() || public_model.is_some() {
            let g = group_id.unwrap_or(&existing.group_id);
            let m = public_model.unwrap_or(&existing.public_model);
            self.validate(g, m, existing.channel_key, key_index.unwrap_or(existing.key_index)).await?;
        }
        if let Some(s) = status {
            if ![1, 2].contains(&s) {
                return Err(AuthError::BadRequest("status must be 1|2".into()));
            }
        }
        sqlx::query(
            r#"UPDATE route_units SET
               group_id = COALESCE($2, group_id), public_model = COALESCE($3, public_model),
               key_index = COALESCE($4, key_index), upstream_model = COALESCE($5, upstream_model),
               priority = COALESCE($6, priority), weight = COALESCE($7, weight),
               status = COALESCE($8, status), updated_at = now() WHERE key = $1"#,
        )
        .bind(key)
        .bind(group_id.map(str::trim))
        .bind(public_model.map(str::trim))
        .bind(key_index)
        .bind(upstream_model.map(str::trim))
        .bind(priority)
        .bind(weight)
        .bind(status)
        .execute(&self.pool)
        .await?;
        Ok(row_to_view(self.fetch(key).await?))
    }

    pub async fn delete(&self, key: Uuid) -> Result<(), AuthError> {
        if sqlx::query("DELETE FROM route_units WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected() == 0
        {
            return Err(AuthError::NotFound("route unit not found".into()));
        }
        Ok(())
    }

    /// 渠道删除后的写侧级联（平表无 FK）。
    pub async fn invalidate_by_channel(&self, channel_key: Uuid) -> Result<u64, AuthError> {
        let n = sqlx::query("UPDATE route_units SET status = 2, updated_at = now() WHERE channel_key = $1 AND status = 1")
            .bind(channel_key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        Ok(n)
    }

    pub async fn get(&self, key: Uuid) -> Result<RouteUnitView, AuthError> {
        Ok(row_to_view(self.fetch(key).await?))
    }

    async fn fetch(&self, key: Uuid) -> Result<RouteUnitRow, AuthError> {
        sqlx::query_as(&format!("SELECT {COLS} FROM route_units WHERE key = $1"))
            .bind(key)
            .fetch_optional(&self.pool)
            .await?
            .ok_or(AuthError::NotFound("route unit not found".into()))
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct RouteUnitAppState {
    pub svc: std::sync::Arc<RouteUnitService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: RouteUnitAppState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/route_unit", get(list).post(create))
        .route("/api/route_unit/{key}", get(get_one).put(update).delete(remove))
        .with_state(state)
}

async fn require_admin(auth: &AuthService, h: &HeaderMap) -> Result<(), AuthError> {
    let u = bearer_user(auth, h).await?;
    if u.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

type ErrResp = (StatusCode, Json<Value>);
fn err_json(e: AuthError) -> ErrResp {
    (e.status(), Json(json!({ "code": e.code(), "message": e.to_string() })))
}

async fn parse_key(key: &str) -> Result<Uuid, ErrResp> {
    Uuid::parse_str(key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateRouteUnitRequest {
    #[serde(rename = "groupId")]
    group_id: String,
    public_model: String,
    channel_key: String,
    #[serde(default)]
    key_index: i32,
    upstream_model: String,
    #[serde(default)]
    priority: i32,
    #[serde(default = "default_weight")]
    weight: i32,
}
fn default_weight() -> i32 {
    10
}

async fn create(
    State(s): State<RouteUnitAppState>,
    h: HeaderMap,
    Json(req): Json<CreateRouteUnitRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let channel_key = parse_key(&req.channel_key).await?;
    match s
        .svc
        .create(
            &req.group_id,
            &req.public_model,
            channel_key,
            req.key_index,
            &req.upstream_model,
            req.priority,
            req.weight,
        )
        .await
    {
        Ok(v) => Ok(Json(json!(v))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    #[serde(rename = "groupId")]
    group_id: Option<String>,
    public_model: Option<String>,
    #[serde(default)]
    page: i64,
    #[serde(default = "default_size")]
    size: i64,
}
fn default_size() -> i64 {
    20
}

async fn list(
    State(s): State<RouteUnitAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    match s
        .svc
        .list(q.group_id.as_deref(), q.public_model.as_deref(), q.page, q.size)
        .await
    {
        Ok((items, total)) => Ok(Json(json!({ "items": items, "total": total }))),
        Err(e) => Err(err_json(e)),
    }
}

async fn get_one(
    State(s): State<RouteUnitAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    match s.svc.get(key).await {
        Ok(v) => Ok(Json(json!(v))),
        Err(e) => Err(err_json(e)),
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRouteUnitRequest {
    #[serde(rename = "groupId")]
    group_id: Option<String>,
    public_model: Option<String>,
    key_index: Option<i32>,
    upstream_model: Option<String>,
    priority: Option<i32>,
    weight: Option<i32>,
    status: Option<i16>,
}

async fn update(
    State(s): State<RouteUnitAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
    Json(req): Json<UpdateRouteUnitRequest>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    match s
        .svc
        .update(
            key,
            req.group_id.as_deref(),
            req.public_model.as_deref(),
            req.key_index,
            req.upstream_model.as_deref(),
            req.priority,
            req.weight,
            req.status,
        )
        .await
    {
        Ok(v) => Ok(Json(json!(v))),
        Err(e) => Err(err_json(e)),
    }
}

async fn remove(
    State(s): State<RouteUnitAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = parse_key(&key).await?;
    match s.svc.delete(key).await {
        Ok(()) => Ok(Json(json!({ "success": true }))),
        Err(e) => Err(err_json(e)),
    }
}
