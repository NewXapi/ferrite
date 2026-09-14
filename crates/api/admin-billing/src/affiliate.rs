//! 拉人奖励（affiliate）— 邀请绑定 / 领奖入账 / 归属审计 / 真实统计（平表直连 sqlx）。
//!
//! 表（0009 建）：
//! - `affiliate_links(invitee_key PK, inviter_key, created_at)` — 邀请归属，
//!   被邀人一人一主（PK 防多人重复绑定→重复领奖）。
//! - `affiliate_rewards(key PK, inviter_key, invitee_key, kind, amount, frozen_until)` — 奖励
//!   入账审计，与 `user_balances` 入金同事务（金额与审计不可分家）；
//!   局部唯一索引 `(invitee_key) WHERE kind='invite'` 做 DB 级幂等护栏；
//!   `frozen_until`（0012）非空 = 该笔奖励冻结至到期时间，`thaw_frozen` 到期搬回可用。
//!
//! 奖励金额：先读 `options` 表 `site.affiliate_reward`（内部单位），未配置回落到常量
//! 占位（`DEFAULT_INVITE_REWARD`）。冻结时长：`site.affiliate_reward_freeze_hours`
//! （小时，缺省 0 = 不冻结）。真实配置表对齐 admin-ops options 语义。
//! 设计见 todo/billing-implementation.md 阶段 2。

use axum::{
    Router,
    extract::State,
    http::HeaderMap,
    response::Json,
    routing::{get, post},
};
use contract::api::billing::RewardRequest;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use auth::error::AuthError;
use auth::routes::bearer_user;
use auth::service::AuthService;

use crate::currency::BillingErr;
use crate::wallet::WalletService;

pub struct AffiliateService {
    pool: PgPool,
    wallet: WalletService,
}

/// 拉人统计视图（真实查询，inviter 维度）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateOverview {
    pub user_key: String,
    /// 已归属绑定的被邀人数（`affiliate_links` 按 inviter_key 计数）。
    pub invite_count: i64,
    /// 累计奖励入账额（`affiliate_rewards.amount` 求和，FREE 货币单位；
    /// 含冻结部分——口径是累计已得，不管可用性）。
    pub total_reward: i64,
}

impl AffiliateService {
    pub fn new(pool: PgPool, wallet: WalletService) -> Self {
        Self { pool, wallet }
    }

    /// 被邀人注册（带邀请归属）→ 只绑定关系，不发奖。
    ///
    /// 返回值：是否本次新建绑定（false = 被邀人已归属他人，先到先得）。
    /// 领奖由 `reward_invite_referral` 独立触发（admin/内部调用），绑定与
    /// 发奖分离：注册事务不携带资金写。
    ///
    /// 边界：邀请码字符串 → `inviter_key` 的解析由上层（注册表单/auth）完成，
    /// 本方法只收已解析的 UUID。
    ///
    /// TODO(#188): 邀请码字符串解析待 auth 域提供 aff_code 列。
    pub async fn on_invitee_registered(
        &self,
        inviter_key: Uuid,
        invitee_key: Uuid,
    ) -> Result<bool, BillingErr> {
        self.bind_inviter(inviter_key, invitee_key).await
    }

    /// 绑定邀请归属。返回 true = 本次绑定成功；false = 被邀人已绑过（不覆盖首邀人）。
    ///
    /// 自邀请（inviter == invitee）直接 `BadRequest` 拒绝——自己吃自己奖励是
    /// 刷钱路径，在入口拦，不留给唯一约束兜底。
    pub async fn bind_inviter(
        &self,
        inviter_key: Uuid,
        invitee_key: Uuid,
    ) -> Result<bool, BillingErr> {
        if inviter_key == invitee_key {
            return Err(BillingErr::BadRequest("cannot bind self as inviter".into()));
        }
        let n = sqlx::query(
            r#"
            INSERT INTO affiliate_links (inviter_key, invitee_key)
            VALUES ($1, $2)
            ON CONFLICT (invitee_key) DO NOTHING
            "#,
        )
        .bind(inviter_key)
        .bind(invitee_key)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(n == 1)
    }

    /// invite 领奖：inviter FREE 余额 += 配置额，并写审计行（同一事务）。
    ///
    /// 语义：
    /// - (inviter, invitee) 必须已在 `affiliate_links` 绑定，否则
    ///   `BadRequest`——缺关系是调用方误用，与幂等 0 区分开，不静默吞错；
    /// - 同一 invitee 全局只能领一次：先插审计行，撞局部唯一索引
    ///   `(invitee_key) WHERE kind='invite'` 即已领 → `Ok(0)`（单语句判定，
    ///   并发双领奖也只有第一条能入金）；
    /// - 审计行与入金同事务提交：任一失败整体回滚，金额与审计不可分家；
    /// - 冻结：`site.affiliate_reward_freeze_hours` > 0 时审计行写
    ///   `frozen_until = now() + hours` 且入金走冻结口径（amount 与
    ///   frozen_amount 同加，可用不变，到期由 `WalletService::thaw_frozen`
    ///   搬回）；缺省 0 = 立即可用（与历史行为一致）。
    ///
    /// 返回值：本次实际入账额（0 = 已领过 / 配置为非正值时不占名额）。
    pub async fn reward_invite_referral(
        &self,
        inviter_key: Uuid,
        invitee_key: Uuid,
    ) -> Result<i64, BillingErr> {
        let amount = self.fetch_invite_reward().await;
        if amount <= 0 {
            // 站点把奖励配成 0 = 停用；不入账、也不烧掉被邀人的一次性领奖名额。
            return Ok(0);
        }
        let freeze_hours = self.fetch_freeze_hours().await;
        let mut tx = self.pool.begin().await?;
        let linked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM affiliate_links WHERE inviter_key = $1 AND invitee_key = $2)",
        )
        .bind(inviter_key)
        .bind(invitee_key)
        .fetch_one(&mut *tx)
        .await?;
        if !linked {
            return Err(BillingErr::BadRequest(format!(
                "no affiliate link: inviter {inviter_key} invitee {invitee_key}"
            )));
        }
        let claimed = sqlx::query(
            r#"
            INSERT INTO affiliate_rewards
                (key, inviter_key, invitee_key, kind, amount, frozen_until)
            VALUES ($1, $2, $3, 'invite', $4,
                    CASE WHEN $5 > 0 THEN now() + make_interval(hours => $5) ELSE NULL END)
            ON CONFLICT (invitee_key) WHERE kind = 'invite' DO NOTHING
            "#,
        )
        .bind(Uuid::new_v4())
        .bind(inviter_key)
        .bind(invitee_key)
        .bind(amount)
        .bind(freeze_hours)
        .execute(&mut *tx)
        .await?
        .rows_affected();
        if claimed == 0 {
            return Ok(0);
        }
        // 入金两口径：带冻结走 credit_reward_frozen_in_tx（frozen_amount 同加，
        // 上面审计行的 frozen_until 是 thaw 的到期依据，跟踪与入账同一提交，
        // 不存在「冻结了但没人解冻」的孤儿）；不冻结走原路径——
        // credit_redeem_in_tx 是 wallet 的通用「事务内入账 FREE」入口
        // （credit_reward/credit_topup 皆为同款 credit_in_tx 包装），复用不重造。
        if freeze_hours > 0 {
            WalletService::credit_reward_frozen_in_tx(&mut tx, inviter_key, amount).await?;
        } else {
            self.wallet
                .credit_redeem_in_tx(&mut tx, inviter_key, amount)
                .await?;
        }
        tx.commit().await?;
        tracing::info!(%inviter_key, %invitee_key, amount, freeze_hours, "invite reward credited");
        Ok(amount)
    }

    /// 通用奖励入账（活动等，无绑定关系/无领奖护栏）— /api/affiliate/reward 语义。
    /// 返回值：入账后 FREE 余额（货币单位）。
    pub async fn credit_reward_by_kind(
        &self,
        kind: &str,
        user_key: Uuid,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        self.wallet.credit_reward(kind, user_key, amount).await
    }

    /// 拉人统计：被邀数 + 累计奖励额（`affiliate_links` COUNT +
    /// `affiliate_rewards` SUM，均按 inviter_key 维度）。
    pub async fn user_overview(&self, user_key: Uuid) -> Result<AffiliateOverview, BillingErr> {
        let invite_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM affiliate_links WHERE inviter_key = $1")
                .bind(user_key)
                .fetch_one(&self.pool)
                .await?;
        let total_reward: i64 = sqlx::query_scalar(
            "SELECT COALESCE(SUM(amount), 0)::BIGINT FROM affiliate_rewards WHERE inviter_key = $1",
        )
        .bind(user_key)
        .fetch_one(&self.pool)
        .await?;
        Ok(AffiliateOverview {
            user_key: user_key.to_string(),
            invite_count,
            total_reward,
        })
    }

    async fn fetch_invite_reward(&self) -> i64 {
        // ponytail: options 表 site.affiliate_reward 未配置则常量占位（= $2 @ 500_000/$1）。
        const DEFAULT_INVITE_REWARD: i64 = 1_000_000;
        let value: Option<String> =
            sqlx::query_scalar("SELECT value FROM options WHERE key = 'site.affiliate_reward'")
                .fetch_optional(&self.pool)
                .await
                .unwrap_or(None);
        value
            .as_deref()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_INVITE_REWARD)
    }

    /// 奖励冻结时长（小时）：options `site.affiliate_reward_freeze_hours`。
    /// 未配置/解析失败/非正 → 0 = 不冻结（钱路径上「保守不动」优于误冻用户余额）。
    async fn fetch_freeze_hours(&self) -> i32 {
        let value: Option<String> = sqlx::query_scalar(
            "SELECT value FROM options WHERE key = 'site.affiliate_reward_freeze_hours'",
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);
        value
            .as_deref()
            .and_then(|v| v.parse().ok())
            .filter(|h: &i32| *h > 0)
            .unwrap_or(0)
    }
}

// ---------- axum 路由（对齐 redeem.rs 鉴权/err_json 约定）----------

#[derive(Clone)]
pub struct AffiliateAppState {
    pub svc: std::sync::Arc<AffiliateService>,
    pub auth: std::sync::Arc<AuthService>,
}

pub fn router(state: AffiliateAppState) -> Router {
    Router::new()
        .route("/api/affiliate/reward", post(reward))
        .route("/api/affiliate/bind", post(bind))
        .route("/api/affiliate/overview", get(overview))
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

type ErrResp = (axum::http::StatusCode, Json<serde_json::Value>);
fn err_json(e: impl Into<AuthError>) -> ErrResp {
    let e = e.into();
    (
        e.status(),
        Json(json!({ "code": e.code(), "message": e.to_string() })),
    )
}

/// 拉人奖励触发 — POST /api/affiliate/reward（admin/内部调用）。
/// 请求体：RewardRequest { kind, user_key, amount }。
async fn reward(
    State(s): State<AffiliateAppState>,
    h: HeaderMap,
    Json(req): Json<RewardRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&req.user_key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid userKey".into())))?;
    let credited = s
        .svc
        .credit_reward_by_kind(&req.kind, user_key, req.amount)
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "credited": credited })))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BindRequest {
    inviter_key: String,
    invitee_key: String,
}

/// 邀请归属绑定 — POST /api/affiliate/bind（admin/内部调用）。
/// 请求体：`{ inviterKey, inviteeKey }`（UUID 字符串）。
/// 响应：`{ bound }` — false = 被邀人已归属他人（自邀请 400）。
async fn bind(
    State(s): State<AffiliateAppState>,
    h: HeaderMap,
    Json(req): Json<BindRequest>,
) -> Result<Json<serde_json::Value>, ErrResp> {
    require_admin(&s.auth, &h).await.map_err(err_json)?;
    let inviter_key = Uuid::parse_str(&req.inviter_key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid inviterKey".into())))?;
    let invitee_key = Uuid::parse_str(&req.invitee_key)
        .map_err(|_| err_json(AuthError::BadRequest("invalid inviteeKey".into())))?;
    let bound = s
        .svc
        .bind_inviter(inviter_key, invitee_key)
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "bound": bound })))
}

/// 拉人统计 — GET /api/affiliate/overview（self）。
async fn overview(
    State(s): State<AffiliateAppState>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, ErrResp> {
    let u = bearer_user(&s.auth, &h).await.map_err(err_json)?;
    let user_key = Uuid::parse_str(&u.key).map_err(|_| err_json(AuthError::InvalidToken))?;
    let ov = s.svc.user_overview(user_key).await.map_err(err_json)?;
    Ok(Json(json!({ "overview": ov })))
}
