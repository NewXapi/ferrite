//! 充值服务（订单占位，provider 占位；settle 入金到 user_balances）。
//!
//! 表（0008 建）：
//! - `billing_topups(key PK, user_key, currency, amount, state, provider, created_at, settled_at)`
//!
//! 状态机：pending(未支付) → paid|failed|refunded。
//! open_topup：建 pending 行，返回订单 id（`POST /api/user/topup/order`——
//! `/api/user/topup` 归兑换码核销，见 router 文档）。
//! settle_topup：手工 settle → 调 WalletService.credit_topup 入金 + 更新状态。

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::post,
};
use contract::api::billing::TopUpRequest;
use sqlx::PgPool;
use uuid::Uuid;

use auth::routes::bearer_user;

use crate::currency::BillingErr;
use crate::wallet::WalletService;

pub struct TopupService {
    pool: PgPool,
    wallet: WalletService,
}

impl TopupService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            wallet: WalletService::new(pool),
        }
    }

    /// 开单（不接真支付），建 pending 订单，返回 order id。
    /// 直接写入 billing_topups 表（0008）state='pending'。
    pub async fn open_topup(&self, req: TopUpRequest) -> Result<String, BillingErr> {
        // 验证货币是否存在且启用
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM currency_defs WHERE code = $1 AND enabled = true)",
        )
        .bind(&req.currency)
        .fetch_one(&self.pool)
        .await
        .map_err(BillingErr::Db)?;
        if !exists {
            return Err(BillingErr::BadRequest(format!(
                "currency {} not found or disabled",
                req.currency
            )));
        }

        // key = UUID 字符串（不使用 Uuid 包装，以便在前端易于 copy）
        let key = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO billing_topups (key, user_key, currency, amount, state, provider)
            VALUES ($1, $2, $3, $4, 'pending', '')
            ON CONFLICT (key) DO NOTHING
            "#,
        )
        .bind(&key)
        .bind(
            Uuid::parse_str(&req.user_key)
                .map_err(|e| BillingErr::BadRequest(format!("invalid user key: {e}")))?,
        )
        .bind(&req.currency)
        .bind(req.amount)
        .execute(&self.pool)
        .await
        .map_err(BillingErr::Db)?;

        Ok(key)
    }

    /// 手工结算充值（admin 端或支付回调后调用）。
    /// - CAS 先行：`UPDATE ... SET state='settling' WHERE key=$1 AND state='pending'`，
    ///   rows_affected==1 才入账（并发 settle 只有一个成功，对齐 redeem.rs 行锁纪律）。
    /// - 入金 WalletService.credit_topup（user_key, currency, amount，该货币单位）。
    /// - 入金成功后 `state='paid', settled_at=now()`；入金失败保持 'settling'（可重试）。
    /// - 返回实入账金额。
    pub async fn settle_topup(&self, key: &str) -> Result<i64, BillingErr> {
        let mut tx = self.pool.begin().await.map_err(BillingErr::Db)?;

        // CAS：pending → settling（行锁），rows_affected==1 才继续
        let n = sqlx::query(
            "UPDATE billing_topups SET state = 'settling' WHERE key = $1 AND state = 'pending'",
        )
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(BillingErr::Db)?
        .rows_affected();
        if n == 0 {
            tx.rollback().await.map_err(BillingErr::Db)?;
            return Err(BillingErr::BadRequest(
                "order not found or already settled".into(),
            ));
        }

        let order: (Uuid, String, i64) = sqlx::query_as(
            r#"
            SELECT user_key, currency, amount
            FROM billing_topups
            WHERE key = $1
            "#,
        )
        .bind(key)
        .fetch_one(&mut *tx)
        .await
        .map_err(BillingErr::Db)?;

        let (user_key, currency, amount) = order;

        // 入金
        let credited = self
            .wallet
            .credit_topup(user_key, &currency, amount)
            .await?;

        // 入账成功 → paid（终态）
        sqlx::query(
            r#"
            UPDATE billing_topups
            SET state = 'paid', settled_at = now()
            WHERE key = $1 AND state = 'settling'
            "#,
        )
        .bind(key)
        .execute(&mut *tx)
        .await
        .map_err(BillingErr::Db)?;

        tx.commit().await.map_err(BillingErr::Db)?;
        Ok(credited)
    }
}

// ---------- axum 路由 ----------

#[derive(Clone)]
pub struct TopupAppState {
    pub svc: std::sync::Arc<TopupService>,
    pub auth: std::sync::Arc<auth::AuthService>,
}

/// 充值路由。
///
/// **路径不能是 `/api/user/topup`**：那条被 [`crate::redeem`] 的兑换码核销
/// 占用（new-api 惯例，前端已按该形状接线）。axum 0.8 的 `Router::merge`
/// 对同 path 同 method 重叠会直接 panic，admin-router 聚合时两条一起 merge
/// 会让 `apps/api` 启动即崩——开单因此挂在 `/api/user/topup/order`。
pub fn router(state: TopupAppState) -> Router {
    Router::new()
        // 订单式开单走 /orders：/api/user/topup 已被 redeem 兑换码核销占用
        // （#152，main 前端 rewards 面板消费中），同路径双注册会让 axum
        // merge 直接 panic —— apps/api 整体起不来（e2e wire-contract 实锤）。
        .route("/api/user/topup/orders", post(open_topup))
        .route("/api/user/topup/{key}/settle", post(settle_topup))
        .with_state(state)
}

async fn open_topup(
    State(s): State<TopupAppState>,
    h: HeaderMap,
    Json(req): Json<TopUpRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    // 需要认证用户，但 admin 覆盖全部用户；这里使用 bearer_user，确保用户操作自身资源。
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    // 防伪造他人 user_key：仅允许操作自身帐号（admin 走 admin 前缀端点）。

    if req.user_key != u.key {
        return Err(err_json(auth::AuthError::Forbidden));
    }

    let order_id = s.svc.open_topup(req).await.map_err(err_json)?;
    Ok(Json(serde_json::json!({ "order_id": order_id })))
}

async fn settle_topup(
    State(s): State<TopupAppState>,
    Path(key): Path<String>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let credited = s.svc.settle_topup(&key).await.map_err(err_json)?;
    Ok(Json(serde_json::json!({ "credited": credited })))
}

async fn require_admin(auth: &auth::AuthService, h: &HeaderMap) -> Result<(), auth::AuthError> {
    let u = bearer_user(auth, h).await?;
    if u.role >= auth::routes::ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(auth::AuthError::Forbidden)
    }
}

type ErrResp = (StatusCode, Json<serde_json::Value>);
fn err_json(e: impl Into<auth::AuthError>) -> ErrResp {
    let e = e.into();
    (
        e.status(),
        Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}
