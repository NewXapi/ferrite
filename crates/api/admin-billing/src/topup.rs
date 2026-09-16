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
    routing::{get, post},
};
use contract::api::billing::{TopUpRequest, TopupOrderView};
use sqlx::FromRow;
use sqlx::PgPool;
use sqlx::types::chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use uuid::Uuid;

use auth::routes::bearer_user;

use crate::currency::{BillingErr, CurrencyService};
use crate::topup_epay::{EpayMerchant, EpayProvider};
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
    /// 外部订单号（渠道语义；epay 的 out_trade_no = 本域订单 key）。
    pub reference: String,
    /// 支付跳转 URL（真渠道开单才有；manual 为 None → 前端不渲染「去支付」）。
    pub payment_url: Option<String>,
}

/// 渠道协议错误（网络/上游拒绝等），与 BillingErr（本域错误）分离。
#[derive(Debug, thiserror::Error)]
#[error("provider error: {0}")]
pub struct ProviderError(pub String);

/// 开单结果（open_topup 返回，handler 直接序列化给前端）。
#[derive(Debug, Clone, serde::Serialize)]
pub struct OpenTopupResult {
    /// pending 订单 key（UUID 字符串）。
    pub order_id: String,
    /// 支付跳转 URL（真渠道开单才有；manual 为 None 时 key 被省略，
    /// 前端降级为「已创建订单」文案，不渲染「去支付」）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_url: Option<String>,
}

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
                payment_url: None,
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

    /// 注入易支付渠道：`[payment.epay]` 配置存在时由组装层调用（未配置则不注册，
    /// 请求指 epay 时 open_topup 报 `payment provider not configured`）。
    #[must_use]
    pub fn with_epay(mut self, merchant: EpayMerchant) -> Self {
        self.providers
            .insert("epay", Arc::new(EpayProvider::new(merchant)));
        self
    }

    /// 按 id 查 provider（webhook 路由入口）。
    pub fn provider(&self, id: &str) -> Option<Arc<dyn TopupProvider>> {
        self.providers.get(id).cloned()
    }

    /// 开单：建 pending 订单（0008），返回订单 id 与支付跳转 URL。
    ///
    /// `req.provider` 缺省/空串 = manual（行为与旧版零差异）；指定真渠道时
    /// 先调 `provider.create()` 拿外部引用与 payment_url，**成功才落库**
    /// （失败不留无法支付的僵尸单），订单行写 provider 与 reference。
    ///
    /// 只收 `kind='points'` 货币（0014）：fiat 只是计价展示单位、永远无法入账，
    /// 开了就是永远 settle 不了的僵尸单（settle 时 credit_topup 失败、事务
    /// 回滚、订单退回 pending）。在开单入口拒绝，而不是让单据进状态机后卡死。
    /// epay 的金额口径例外说明见下方折算段。
    pub async fn open_topup(&self, req: TopUpRequest) -> Result<OpenTopupResult, BillingErr> {
        // provider 解析：空串/缺省 = manual（旧请求零差异）。
        let provider_id: &str = if req.provider.is_empty() {
            "manual"
        } else {
            req.provider.as_str()
        };
        // 真渠道必须在注入表里注册；epay 未配置时报明确的「未配置」，
        // 拼错的渠道名报「未知 provider」，两者对运维的排障含义不同。
        // 放在货币校验之前：配置缺失是组装级故障，fail-fast 不必先查库。
        let provider: Option<Arc<dyn TopupProvider>> = if provider_id == "manual" {
            None
        } else {
            Some(match self.provider(provider_id) {
                Some(p) => p,
                None => {
                    return Err(BillingErr::BadRequest(if provider_id == "epay" {
                        "payment provider not configured".into()
                    } else {
                        format!("unknown payment provider: {provider_id}")
                    }));
                }
            })
        };

        // 查启用货币的 kind 再分流报错：fiat「存在且启用、但不能充值」与
        // 「不存在/停用」是两种不同的状况，混为一谈会把后者误导成前者。
        let kind: Option<String> =
            sqlx::query_scalar("SELECT kind FROM currency_defs WHERE code = $1 AND enabled = true")
                .bind(&req.currency)
                .fetch_optional(&self.pool)
                .await
                .map_err(BillingErr::Db)?;
        match kind.as_deref() {
            // points 放行（正常充值路径，行为零变化）。
            Some("points") => {}
            // 0014 CHECK 约束只有 points|fiat；文案带 kind 值而非硬编码 "fiat"，
            // 将来若加第三种 kind 不会误报（ocr review 建议，采纳）。
            Some(unexpected_kind) => {
                return Err(BillingErr::BadRequest(format!(
                    "currency {} (kind {unexpected_kind}) cannot be topped up (points only)",
                    req.currency
                )));
            }
            None => {
                return Err(BillingErr::BadRequest(format!(
                    "currency {} not found or disabled",
                    req.currency
                )));
            }
        }

        // epay 金额口径：渠道收 CNY（元），订单行存折算后的点数——fiat 不进
        // 余额（不变式），只有点数金额能被 settle 入账。其余 provider 直接
        // 用请求金额。charge_* = 喂给 provider.create 的支付口径。
        let (order_currency, order_amount, charge_currency, charge_amount) =
            if provider_id == "epay" {
                // 0/负金额不开单（convert 对 <=0 返回 0，渠道也拒收，入口挡住）。
                if req.amount <= 0 {
                    return Err(BillingErr::BadRequest(
                        "topup amount must be positive".into(),
                    ));
                }
                let points = CurrencyService::new(self.pool.clone())
                    .convert(req.amount, "CNY", &req.currency)
                    .await?;
                (req.currency.clone(), points, "CNY", req.amount)
            } else {
                (
                    req.currency.clone(),
                    req.amount,
                    req.currency.as_str(),
                    req.amount,
                )
            };

        // key = UUID 字符串（不使用 Uuid 包装，以便在前端易于 copy）
        let key = Uuid::new_v4().to_string();
        // 真渠道先开单拿外部引用与支付 URL；失败则不落库（不留无法支付的僵尸单）。
        let (reference, payment_url) = match provider {
            None => (None, None),
            Some(p) => {
                let session = p
                    .create(&key, charge_currency, charge_amount)
                    .await
                    .map_err(|e| BillingErr::BadRequest(format!("payment provider error: {e}")))?;
                (Some(session.reference), session.payment_url)
            }
        };
        sqlx::query(
            r#"
            INSERT INTO billing_topups (key, user_key, currency, amount, state, provider, reference)
            VALUES ($1, $2, $3, $4, 'pending', $5, $6)
            ON CONFLICT (key) DO NOTHING
            "#,
        )
        .bind(&key)
        .bind(
            Uuid::parse_str(&req.user_key)
                .map_err(|e| BillingErr::BadRequest(format!("invalid user key: {e}")))?,
        )
        .bind(&order_currency)
        .bind(order_amount)
        .bind(provider_id)
        .bind(reference)
        .execute(&self.pool)
        .await
        .map_err(BillingErr::Db)?;

        Ok(OpenTopupResult {
            order_id: key,
            payment_url,
        })
    }

    /// 用户的充值订单列表（`GET /api/user/topup/orders`）。
    ///
    /// 按 `user_key` 过滤 `billing_topups`，`created_at` 倒序取 `limit` 条；
    /// `created_at` 输出 RFC3339 字符串（与 redeem.rs 的 DateTime 序列化同口径）。
    pub async fn list_orders(
        &self,
        user_key: Uuid,
        limit: i64,
    ) -> Result<Vec<TopupOrderView>, BillingErr> {
        let rows = sqlx::query_as::<_, TopupOrderRow>(
            r#"
            SELECT key, currency, amount, state, provider, created_at
            FROM billing_topups
            WHERE user_key = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_key)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| TopupOrderView {
                key: r.key,
                currency: r.currency,
                amount: r.amount,
                state: r.state,
                provider: r.provider,
                created_at: r.created_at.to_rfc3339(),
            })
            .collect())
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

/// `billing_topups` 行的解码形状（`list_orders` 用）；`created_at` 在转 DTO 时
/// 走 `to_rfc3339()`，DTO 持 String。
#[derive(Debug, Clone, FromRow)]
struct TopupOrderRow {
    key: String,
    currency: String,
    amount: i64,
    state: String,
    provider: String,
    created_at: DateTime<Utc>,
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
        // 列表 GET 与开单 POST 同路径不同 method：axum 0.8 链式 method router
        // （get(..).post(..)）注册，不与 redeem 的 POST /api/user/topup 冲突。
        .route("/api/user/topup/orders", get(list_orders).post(open_topup))
        .route("/api/user/topup/{key}/settle", post(settle_topup))
        .route("/api/topup/webhook/{provider_id}", post(topup_webhook))
        .with_state(state)
}

async fn open_topup(
    State(s): State<TopupAppState>,
    h: HeaderMap,
    Json(req): Json<TopUpRequest>,
) -> Result<Json<OpenTopupResult>, ErrResp> {
    // 需要认证用户，但 admin 覆盖全部用户；这里使用 bearer_user，确保用户操作自身资源。
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    // 防伪造他人 user_key：仅允许操作自身帐号（admin 走 admin 前缀端点）。

    if req.user_key != u.key {
        return Err(err_json(auth::AuthError::Forbidden));
    }

    let res = s.svc.open_topup(req).await.map_err(err_json)?;
    Ok(Json(res))
}

/// 用户的充值订单列表 — GET /api/user/topup/orders（self）。
/// 响应：`{ "items": [TopupOrderView] }`（前端奖励面板「充值记录」消费）。
/// 鉴权同 open_topup：bearer_user 取自身 user_key，只查本人的订单。
async fn list_orders(
    State(s): State<TopupAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&u.key).map_err(|_| err_json(auth::AuthError::InvalidToken))?;
    // ponytail: 无分页参数——奖励面板一屏列表，固定 50 条够用；
    // 真要分页时加 axum Query<Page> 再在此处透传 limit。
    let items = s.svc.list_orders(user_key, 50).await.map_err(err_json)?;
    Ok(Json(serde_json::json!({ "items": items })))
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
