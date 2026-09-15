//! 货币定义 + 用户货币余额 seed/折算（平表直连 sqlx）。
//!
//! 表（T1 迁移 0007 建）：
//! - `currency_defs(code PK, name, internal_rate DOUBLE, enabled, remark)`
//! - `user_balances(user_key, currency_code PK, amount BIGINT)`
//!
//! 折算综合可用值：`available_i64 = COALESCE(SUM(amount * internal_rate), 0)::BIGINT WHERE enabled`。
//! 内部单位 500_000 = $1（new-api 语义）；cost 即内部单位 i64。
//!
//! 并发语义：seed 用 `ON CONFLICT DO NOTHING`（幂等）；upsert_def 单事务内
//! `ON CONFLICT DO UPDATE`（行锁）+ 补 seed，原子提交。

use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::get,
};
use contract::api::billing::CurrencyView;
use serde::Deserialize;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;
use serde_json::json;

use crate::affiliate::AffiliateService;
use crate::wallet::WalletService;

/// 货币服务：注册 seed / 折算可用值 / 定义管理。
pub struct CurrencyService {
    pool: PgPool,
}

/// 注册后置 hook（auth::routes::OnUserRegistered 实现）：
/// 新用户注册成功 → seed 全部启用货币（幂等，amount=0）；若注册请求带
/// 邀请码，校验邀请人存在后绑定邀请归属（affiliate_links）。
///
/// 放在 billing 而非 auth：依赖方向 billing→auth，trait 由 auth 定义、
/// 本侧实现并经 admin-router 注入（#179 多货币；#197 邀请链接闭环）。
pub struct WalletSeedHook {
    pool: PgPool,
}

impl WalletSeedHook {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl auth::routes::OnUserRegistered for WalletSeedHook {
    fn on_registered(&self, user_key: uuid::Uuid, invite: Option<&str>) {
        // fire-and-forget：注册路径不该被货币层拖慢/拖死，seed 失败有
        // warn 可追，钱包首次入账时 available_i64 查无行按 0 兜底。
        let currency = CurrencyService::new(self.pool.clone());
        tokio::spawn(async move {
            if let Err(e) = currency.seed_for_user(user_key).await {
                tracing::warn!(error = %e, user_key = %user_key, "currency seed for new user failed");
            }
        });

        // 邀请归属：独立 spawn，与 seed 并行——两边互不拖累，任一失败
        // 都只是少一个旁路增益（钱/归属都能事后补），不回灌注册失败。
        if let Some(inviter) = parse_invite_code(invite) {
            let pool = self.pool.clone();
            tokio::spawn(async move {
                bind_invite_relation(&pool, inviter, user_key).await;
            });
        }
    }
}

/// 邀请码 → 邀请人 user_key：`None`、空串、非 UUID 文本一律 `None`。
///
/// 邀请码即邀请人的 user_key（UUID），注册链接 `?invite=<uuid>` 直传。
/// 解析失败静默丢弃——邀请是注册的旁路增益，脏输入/手改链接不该阻断
/// 账号创建（恶意输入见 `bind_invite_relation`：连 DB 都不会碰）。
pub fn parse_invite_code(invite: Option<&str>) -> Option<Uuid> {
    Uuid::parse_str(invite?).ok()
}

/// 校验邀请人存在后绑定归属（fire-and-forget 任务载荷）。
///
/// 全程静默，理由见 trait 文档：邀请人不存在（脏/过期邀请码）→ debug，
/// 这是可预期的脏数据，不值得 warn 污染日志；被邀人已归属他人 → warn，
/// 先到先得是正常竞态但值得追查是否有重复发奖；绑定失败 → warn。
/// 自邀请（inviter == invitee）由 `bind_inviter` 拒绝，此处不重复拦。
async fn bind_invite_relation(pool: &PgPool, inviter: Uuid, invitee: Uuid) {
    let exists: bool =
        match sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_users WHERE key = $1)")
            .bind(inviter)
            .fetch_one(pool)
            .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(error = %e, inviter = %inviter, "inviter existence check failed");
                return;
            }
        };
    if !exists {
        tracing::debug!(inviter = %inviter, "invite code references nonexistent user, skipping bind");
        return;
    }
    let svc = AffiliateService::new(pool.clone(), WalletService::new(pool.clone()));
    match svc.bind_inviter(inviter, invitee).await {
        Ok(true) => {
            tracing::debug!(inviter = %inviter, invitee = %invitee, "invite attribution bound")
        }
        Ok(false) => {
            tracing::warn!(inviter = %inviter, invitee = %invitee, "invite bind skipped: invitee already attributed to another inviter")
        }
        Err(e) => {
            tracing::warn!(error = %e, inviter = %inviter, invitee = %invitee, "invite bind failed")
        }
    }
}

/// 本域统一错误（wallet/affiliate/topup 复用；handler 边界转 AuthError 响应）。
#[derive(Debug, thiserror::Error)]
pub enum BillingErr {
    #[error("db error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<BillingErr> for AuthError {
    fn from(e: BillingErr) -> Self {
        match e {
            BillingErr::Db(e) => AuthError::Db(e),
            BillingErr::BadRequest(msg) => AuthError::BadRequest(msg),
            BillingErr::NotFound(msg) => AuthError::NotFound(msg),
        }
    }
}

impl CurrencyService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// 为注册用户 seed 所有 enabled 货币（amount = 0）。
    /// 幂等：`ON CONFLICT DO NOTHING`，重复调用无副作用。
    pub async fn seed_for_user(&self, user_key: Uuid) -> Result<(), BillingErr> {
        sqlx::query(
            r#"
            INSERT INTO user_balances (user_key, currency_code, amount)
            SELECT $1, code, 0 FROM currency_defs WHERE enabled
            ON CONFLICT (user_key, currency_code) DO NOTHING
            "#,
        )
        .bind(user_key)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// 折算综合可用值（内部单位 i64），喂 QuotaGate/快照。
    pub async fn available_i64(&self, user_key: Uuid) -> Result<i64, BillingErr> {
        let row: (i64,) = sqlx::query_as(
            r#"
            SELECT COALESCE(SUM(ub.amount * cd.internal_rate), 0)::BIGINT
            FROM user_balances ub
            JOIN currency_defs cd ON cd.code = ub.currency_code AND cd.enabled
            WHERE ub.user_key = $1
            "#,
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(row.0)
    }

    /// 货币定义列表（admin 查看）。
    pub async fn list_defs(&self) -> Result<Vec<CurrencyView>, BillingErr> {
        let rows = sqlx::query_as::<_, CurrencyDefRow>(
            "SELECT code, name, internal_rate, enabled, remark FROM currency_defs ORDER BY code",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    /// 新增/更新货币定义（admin）。
    /// 单事务：`ON CONFLICT DO UPDATE` 行锁；enabled 时顺带为缺余额行的现有用户补 seed 0（幂等）。
    pub async fn upsert_def(
        &self,
        code: &str,
        name: &str,
        internal_rate: f64,
        enabled: bool,
        remark: &str,
    ) -> Result<CurrencyView, BillingErr> {
        if code.trim().is_empty() {
            return Err(BillingErr::BadRequest("code required".into()));
        }
        let internal_rate = if internal_rate.is_finite() && internal_rate > 0.0 {
            internal_rate
        } else {
            return Err(BillingErr::BadRequest(
                "internal_rate must be finite and > 0".into(),
            ));
        };
        let mut tx = self.pool.begin().await?;
        let row: CurrencyDefRow = sqlx::query_as(
            r#"
            INSERT INTO currency_defs (code, name, internal_rate, enabled, remark)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (code) DO UPDATE SET
                name = EXCLUDED.name,
                internal_rate = EXCLUDED.internal_rate,
                enabled = EXCLUDED.enabled,
                remark = EXCLUDED.remark,
                updated_at = now()
            RETURNING code, name, internal_rate, enabled, remark
            "#,
        )
        .bind(code)
        .bind(name)
        .bind(internal_rate)
        .bind(enabled)
        .bind(remark)
        .fetch_one(&mut *tx)
        .await?;
        // 启用新货币：现有用户缺余额行则补 0（ON CONFLICT DO NOTHING，幂等）。
        if enabled {
            sqlx::query(
                r#"
                INSERT INTO user_balances (user_key, currency_code, amount)
                SELECT u.key, $1, 0 FROM auth_users u
                WHERE NOT EXISTS (
                    SELECT 1 FROM user_balances ub
                    WHERE ub.user_key = u.key AND ub.currency_code = $1
                )
                "#,
            )
            .bind(code)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(row.into())
    }
}

#[derive(Debug, Clone, FromRow)]
struct CurrencyDefRow {
    code: String,
    name: String,
    internal_rate: f64,
    enabled: bool,
    remark: String,
}

impl From<CurrencyDefRow> for CurrencyView {
    fn from(r: CurrencyDefRow) -> Self {
        CurrencyView {
            code: r.code,
            name: r.name,
            internal_rate: r.internal_rate,
            enabled: r.enabled,
            remark: r.remark,
        }
    }
}

// ---------- axum 路由（对齐 redeem.rs 鉴权/err_json 约定）----------

#[derive(Clone)]
pub struct CurrencyAppState {
    pub svc: std::sync::Arc<CurrencyService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: CurrencyAppState) -> axum::Router {
    Router::new()
        .route("/api/currency", get(list_defs).post(upsert_def))
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
fn err_json(e: impl Into<AuthError>) -> ErrResp {
    let e = e.into();
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

/// 货币定义列表 — GET /api/currency（admin）。
async fn list_defs(
    State(s): State<CurrencyAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let defs = s.svc.list_defs().await.map_err(err_json)?;
    Ok(Json(json!({ "items": defs })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertDefRequest {
    code: String,
    name: String,
    internal_rate: f64,
    #[serde(default)]
    enabled: bool,
    #[serde(default)]
    remark: String,
}

/// 新增/更新货币 — POST /api/currency（admin）。
async fn upsert_def(
    State(s): State<CurrencyAppState>,
    h: HeaderMap,
    Json(req): Json<UpsertDefRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let view = s
        .svc
        .upsert_def(
            &req.code,
            &req.name,
            req.internal_rate,
            req.enabled,
            &req.remark,
        )
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "currency": view })))
}
