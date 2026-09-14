//! 充值服务：provider 抽象（开单/验签）+ 订单落库 + settle 入金（CAS 幂等）。
//!
//! 表（0008 建）：
//! - `billing_topups(key PK, user_key, currency, amount, state, provider, created_at, settled_at)`
//!
//! 状态机：pending(未支付) → paid|failed|refunded。
//! open_topup：建 pending 行，返回订单 id（`POST /api/user/topup/order`——
//! `/api/user/topup` 归兑换码核销，见 router 文档）。
//! settle_topup：手工 settle → 调 WalletService.credit_topup 入金 + 更新状态。
//! topup_webhook：`POST /api/topup/webhook/{provider_id}`，验签即鉴权，
//! 验通过走 settle_topup 的 CAS 幂等结算；重放回执 200 + 已入账额。
//! provider 注入表由 [`TopupService`] 持有（默认 [`ManualProvider`]）。

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::post,
};
use contract::api::billing::TopUpRequest;
use sqlx::PgPool;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

use auth::routes::bearer_user;

use crate::currency::BillingErr;
use crate::wallet::WalletService;

// ---------- provider 抽象 ----------

/// dyn 兼容的异步返回：`async fn` in trait 不能装进 `Arc<dyn TopupProvider>`，
/// 且工作区无 async-trait 依赖。ponytail: 第二个真渠道落地前不再加依赖。
pub type ProviderFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// 充值渠道抽象：开单 → 回调验签。
/// 实现方负责外部协议；金额/货币语义在本域（internal 单位换算归 WalletService）。
pub trait TopupProvider: Send + Sync {
    /// 渠道标识（存 billing_topups.provider，也是 webhook 路径参数）。
    fn id(&self) -> &'static str;
    /// 开单：返回 provider 侧的支付引用（订单号/跳转 URL）。不落库——落库归 TopupService。
    fn create<'a>(
        &'a self,
        order_key: &'a str,
        currency: &'a str,
        amount: i64,
    ) -> ProviderFuture<'a, Result<TopupSession, ProviderError>>;
    /// 回调验签：验通过 → 返回 Some(order_key)；验失败 → None（handler 返回 401）。
    fn verify_callback<'a>(
        &'a self,
        payload: &'a serde_json::Value,
    ) -> ProviderFuture<'a, Option<String>>;
}

/// provider 侧开单结果。
pub struct TopupSession {
    /// 外部订单号 / 支付跳转 URL（渠道语义，本域只透传）。
    pub reference: String,
}

/// 渠道协议错误（网络/上游拒绝等），与 BillingErr（本域错误）分离。
#[derive(Debug, thiserror::Error)]
#[error("provider error: {0}")]
pub struct ProviderError(pub String);

/// 默认渠道：无外部支付系统——开单只回订单号，验签恒拒。
/// manual 单据只能走 admin settle 端点入金，伪造 webhook 也拿不到 Some。
pub struct ManualProvider;

impl TopupProvider for ManualProvider {
    fn id(&self) -> &'static str {
        "manual"
    }

    fn create<'a>(
        &'a self,
        order_key: &'a str,
        _currency: &'a str,
        _amount: i64,
    ) -> ProviderFuture<'a, Result<TopupSession, ProviderError>> {
        Box::pin(async move {
            Ok(TopupSession {
                reference: order_key.to_string(),
            })
        })
    }

    fn verify_callback<'a>(
        &'a self,
        _payload: &'a serde_json::Value,
    ) -> ProviderFuture<'a, Option<String>> {
        Box::pin(async { None })
    }
}

pub struct TopupService {
    pool: PgPool,
    wallet: WalletService,
    /// provider 注入表（id → 实现）。构造时只含 manual；`with_provider` 追加。
    providers: HashMap<&'static str, Arc<dyn TopupProvider>>,
}

impl TopupService {
    pub fn new(pool: PgPool) -> Self {
        let mut providers: HashMap<&'static str, Arc<dyn TopupProvider>> = HashMap::new();
        providers.insert("manual", Arc::new(ManualProvider));
        Self {
            pool: pool.clone(),
            wallet: WalletService::new(pool),
            providers,
        }
    }

    /// 追加/替换一个 provider（构造期 builder，链式）。真接 epay/stripe 在组装时注入。
    #[must_use]
    pub fn with_provider(mut self, provider: Arc<dyn TopupProvider>) -> Self {
        self.providers.insert(provider.id(), provider);
        self
    }

    /// 按 id 查 provider（webhook 路由入口）。
    pub fn provider(&self, id: &str) -> Option<Arc<dyn TopupProvider>> {
        self.providers.get(id).cloned()
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
    /// paid 终态订单的入账额（webhook 重放的幂等回执）；非 paid/不存在 → None。
    ///
    /// 值取订单行 amount：`credit_topup` 入金路径纯加法无截断（wallet.rs 只在
    /// *扣费* 侧 clamp），且 settle 事务里 paid 落态与入金成对提交，
    /// 故 paid ⇒ 该 amount 已入账，与首次 settle 返回值一致。
    pub async fn settled_credit(&self, key: &str) -> Result<Option<i64>, BillingErr> {
        sqlx::query_scalar("SELECT amount FROM billing_topups WHERE key = $1 AND state = 'paid'")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(BillingErr::Db)
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
        .route("/api/topup/webhook/{provider_id}", post(topup_webhook))
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

/// 支付回调 webhook：**无鉴权——验签就是鉴权**（verify_callback 是唯一信任边界）。
///
/// 流程：provider_id 查注入表（未知 → 404）→ `verify_callback`（失败 → 401）
/// → `settle_topup`（CAS pending→settling→paid，天然幂等）。
/// 渠道重放已入账的订单：CAS 失败但订单 paid → 200 + 已入账额（回执成功，
/// 否则渠道侧会无限重试）。其余 settle 错误原样透传。
///
/// `pub` 只为集成测试直调（本 crate 无 tower dev-dep，不为此加依赖做 oneshot；
/// 需要行为证明的是三分支语义而非路由字符串）。
pub async fn topup_webhook(
    State(s): State<TopupAppState>,
    Path(provider_id): Path<String>,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let Some(provider) = s.svc.provider(&provider_id) else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(
                serde_json::json!({ "message": format!("unknown topup provider: {provider_id}") }),
            ),
        ));
    };
    let Some(order_key) = provider.verify_callback(&payload).await else {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "message": "callback verification failed" })),
        ));
    };
    match s.svc.settle_topup(&order_key).await {
        Ok(credited) => Ok(Json(serde_json::json!({ "credited": credited }))),
        Err(e) => {
            // 重放护栏：CAS 失败但订单已 paid ⇒ 本次是渠道重发，回执幂等成功。
            match s.svc.settled_credit(&order_key).await {
                Ok(Some(credited)) => Ok(Json(serde_json::json!({ "credited": credited }))),
                _ => Err(err_json(e)),
            }
        }
    }
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
