//! 兑换码 — 批量生成 / 单次核销（平表直连 sqlx）。
//!
//! 参考: one-api redemptions + new-api store_redemption.go (行锁/CAS)。
//!
//! - 码明文 = "fx-" + 32 hex (16B random)，明文存库（一次性优惠券码，管理员可复查）
//! - 核销 CAS: `UPDATE ... WHERE code = $1 AND status = 1 RETURNING quota`，
//!   并发核销同一码只有一个成功，其余 404/Conflict
//! - 核销事务内同时给 `auth_users.quota` 入账（用户余额，与网关扣费
//!   `used_quota` 相对；契约见 todo/admin-api.md P1）

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

pub async fn ensure_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        r#"CREATE TABLE IF NOT EXISTS billing_redemptions (
    key         UUID PRIMARY KEY,
    code_hash   TEXT UNIQUE NOT NULL,
    code_preview TEXT NOT NULL DEFAULT '',
    quota       BIGINT NOT NULL,
    status      SMALLINT NOT NULL DEFAULT 1,
    redeemed_by UUID,
    redeemed_at TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS idx_billing_redemptions_status ON billing_redemptions(status);"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RedemptionView {
    pub key: String,
    pub code_preview: String,
    pub quota: i64,
    pub status: i16,
    pub redeemed_by: Option<String>,
    pub redeemed_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
    pub created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

#[derive(Debug, Clone, FromRow)]
struct RedemptionRow {
    key: Uuid,
    code_preview: String,
    quota: i64,
    status: i16,
    redeemed_by: Option<Uuid>,
    redeemed_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
    created_at: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
}

const COLS: &str =
    "key, code_preview, quota, status, redeemed_by, redeemed_at, created_at";

fn row_to_view(r: RedemptionRow) -> RedemptionView {
    RedemptionView {
        key: r.key.to_string(),
        code_preview: r.code_preview,
        quota: r.quota,
        status: r.status,
        redeemed_by: r.redeemed_by.map(|u| u.to_string()),
        redeemed_at: r.redeemed_at,
        created_at: r.created_at,
    }
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

fn preview(plaintext: &str) -> String {
    let body = plaintext.strip_prefix("fx-").unwrap_or(plaintext);
    format!("fx-{}****{}", &body[..4.min(body.len())], &body[body.len().saturating_sub(4)..])
}

pub struct RedeemService {
    pool: PgPool,
}

impl RedeemService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 批量生成: count 条唯一码，同 quota；明文仅返回一次，库内 sha256。
    pub async fn generate(
        &self,
        quota: i64,
        count: u32,
    ) -> Result<Vec<String>, AuthError> {
        if quota <= 0 {
            return Err(AuthError::BadRequest("quota must be > 0".into()));
        }
        let count = count.clamp(1, 100);
        let mut plaintexts = Vec::with_capacity(count as usize);
        let mut tx = self.pool.begin().await?;
        for _ in 0..count {
            let mut buf = [0u8; 16];
            rand::thread_rng().fill_bytes(&mut buf);
            let plaintext = format!("fx-{}", hex::encode(buf));
            let code_hash = sha256_hex(&plaintext);
            let key = Uuid::new_v4();
            sqlx::query(
                r#"INSERT INTO billing_redemptions (key, code_hash, code_preview, quota)
                   VALUES ($1, $2, $3, $4)"#,
            )
            .bind(key)
            .bind(&code_hash)
            .bind(preview(&plaintext))
            .bind(quota)
            .execute(&mut *tx)
            .await?;
            plaintexts.push(plaintext);
        }
        tx.commit().await?;
        Ok(plaintexts)
    }

    /// 核销: 单次有效（CAS），事务内入账 auth_users.quota。
    /// 并发核销同一码 → 只有一个成功，其余 NotFound。
    pub async fn redeem(&self, code: &str, user_key: Uuid) -> Result<i64, AuthError> {
        if code.trim().is_empty() {
            return Err(AuthError::BadRequest("code required".into()));
        }
        let code_hash = sha256_hex(code.trim());
        let mut tx = self.pool.begin().await?;
        let row: Option<(i64,)> = sqlx::query_as(
            r#"UPDATE billing_redemptions
               SET status = 2, redeemed_by = $2, redeemed_at = now()
               WHERE code_hash = $1 AND status = 1
               RETURNING quota"#,
        )
        .bind(&code_hash)
        .bind(user_key)
        .fetch_optional(&mut *tx)
        .await?;
        let quota = row.ok_or(AuthError::NotFound("redemption code invalid or used".into()))?.0;
        let applied = sqlx::query("UPDATE auth_users SET quota = quota + $2, updated_at = now() WHERE key = $1")
            .bind(user_key)
            .bind(quota)
            .execute(&mut *tx)
            .await?
            .rows_affected();
        // 入账目标不存在 → 回滚（码保持未核销，资金不丢）
        if applied == 0 {
            return Err(AuthError::NotFound("user not found".into()));
        }
        tx.commit().await?;
        Ok(quota)
    }

    /// admin 列表（分页）。
    pub async fn list(&self, status: Option<i16>, page: i64, size: i64) -> Result<(Vec<RedemptionView>, i64), AuthError> {
        let size = size.clamp(1, 100);
        let offset = (page.max(1) - 1) * size;
        let (count_sql, list_sql) = if status.is_some() {
            ("SELECT count(*) FROM billing_redemptions WHERE status = $1".to_string(),
             format!("SELECT {COLS} FROM billing_redemptions WHERE status = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3"))
        } else {
            ("SELECT count(*) FROM billing_redemptions".to_string(),
             format!("SELECT {COLS} FROM billing_redemptions ORDER BY created_at DESC LIMIT $1 OFFSET $2"))
        };
        let total: i64 = if let Some(s) = status {
            sqlx::query_scalar(&count_sql).bind(s).fetch_one(&self.pool).await?
        } else {
            sqlx::query_scalar(&count_sql).fetch_one(&self.pool).await?
        };
        let rows: Vec<RedemptionRow> = if let Some(s) = status {
            sqlx::query_as(&list_sql).bind(s).bind(size).bind(offset).fetch_all(&self.pool).await?
        } else {
            sqlx::query_as(&list_sql).bind(size).bind(offset).fetch_all(&self.pool).await?
        };
        Ok((rows.into_iter().map(row_to_view).collect(), total))
    }

    /// admin 禁用未核销的码。
    pub async fn disable(&self, key: Uuid) -> Result<(), AuthError> {
        let n = sqlx::query("UPDATE billing_redemptions SET status = 3 WHERE key = $1 AND status = 1")
            .bind(key)
            .execute(&self.pool)
            .await?
            .rows_affected();
        if n == 0 {
            return Err(AuthError::NotFound("redemption not found or already used".into()));
        }
        Ok(())
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct RedeemAppState {
    pub svc: std::sync::Arc<RedeemService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: RedeemAppState) -> axum::Router {
    use axum::routing::{get, post};
    axum::Router::new()
        .route("/api/redemption", get(list).post(generate))
        .route("/api/redemption/{key}", axum::routing::delete(remove))
        .route("/api/user/topup", post(topup))
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

type ErrResp = (StatusCode, Json<serde_json::Value>);
fn err_json(e: AuthError) -> ErrResp {
    (e.status(), Json(json!({ "code": e.code(), "message": e.to_string() })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateRequest {
    #[serde(default)]
    quota: i64,
    #[serde(default = "default_count")]
    count: u32,
}
fn default_count() -> u32 {
    1
}

async fn generate(
    State(s): State<RedeemAppState>,
    h: HeaderMap,
    Json(req): Json<GenerateRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let plaintexts = s.svc.generate(req.quota, req.count).await.map_err(err_json)?;
    Ok(Json(json!({ "codes": plaintexts })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ListQuery {
    status: Option<i16>,
    #[serde(default)]
    page: i64,
    #[serde(default = "default_size")]
    size: i64,
}
fn default_size() -> i64 {
    20
}

async fn list(
    State(s): State<RedeemAppState>,
    h: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let (items, total) = s.svc.list(q.status, q.page, q.size).await.map_err(err_json)?;
    Ok(Json(json!({ "items": items, "total": total })))
}

async fn remove(
    State(s): State<RedeemAppState>,
    h: HeaderMap,
    Path(key): Path<String>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let key = Uuid::parse_str(&key).map_err(|_| err_json(AuthError::BadRequest("invalid key".into())))?;
    s.svc.disable(key).await.map_err(err_json)?;
    Ok(Json(json!({ "success": true })))
}

#[derive(Debug, Deserialize)]
struct TopupRequest {
    key: String,
}

/// 用户兑换 — POST /api/user/topup { key }，入账 auth_users.quota。
async fn topup(
    State(s): State<RedeemAppState>,
    h: HeaderMap,
    Json(req): Json<TopupRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let user = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&user.key).map_err(|_| err_json(AuthError::InvalidToken))?;
    let quota = s.svc.redeem(&req.key, user_key).await.map_err(err_json)?;
    Ok(Json(json!({ "quota": quota, "success": true })))
}
