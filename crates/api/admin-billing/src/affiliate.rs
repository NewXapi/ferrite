//! 拉人奖励（affiliate）— 被邀人注册/充值 → inviter FREE 余额加分（平表直连 sqlx）。
//!
//! 奖励金额：先读 `options` 表 `site.affiliate_reward`（内部单位），未配置回落到常量
//! 占位（`DEFAULT_INVITE_REWARD`）。真实配置表对齐 admin-ops options 语义。
//! 入账走 `WalletService::credit_reward`（FREE，kind 区分来源；审计表/冻结额度是后续扩展，
//! 设计见 todo/billing-implementation.md 阶段 2）。

use axum::{
    Router,
    extract::State,
    http::HeaderMap,
    response::Json,
    routing::{get, post},
};
use contract::api::billing::RewardRequest;
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

/// 拉人统计占位视图（真实统计待 affiliate_links 表，issue 未分配前 TODO）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AffiliateOverview {
    pub user_key: String,
    /// TODO(affiliate-stats): 被邀人数 — 需建 affiliate_links(inviter_key, invitee_key, created_at)
    pub invite_count: i64,
    /// TODO(affiliate-stats): 累计奖励（内部单位）
    pub total_reward: i64,
}

impl AffiliateService {
    pub fn new(pool: PgPool, wallet: WalletService) -> Self {
        Self { pool, wallet }
    }

    /// 被邀人注册成功 → inviter FREE 余额 += 配置值。
    /// 返回值：入账后 inviter FREE 余额（货币单位）。
    pub async fn on_invitee_registered(&self, inviter_key: Uuid) -> Result<i64, BillingErr> {
        let reward = self.fetch_invite_reward().await;
        self.wallet
            .credit_reward("invite", inviter_key, reward)
            .await
    }

    /// 奖励触发入口（内部调用/admin）：按 kind 给 user 入账。
    pub async fn reward_invite_referral(
        &self,
        user_key: Uuid,
        kind: &str,
        amount: i64,
    ) -> Result<i64, BillingErr> {
        self.wallet.credit_reward(kind, user_key, amount).await
    }

    /// 拉人统计占位：被邀数/累计奖励。
    /// ponytail: 真实统计需 affiliate_links 表，先返回 0/空（TODO issue 未分配）。
    pub async fn user_overview(&self, user_key: Uuid) -> Result<AffiliateOverview, BillingErr> {
        Ok(AffiliateOverview {
            user_key: user_key.to_string(),
            invite_count: 0,
            total_reward: 0,
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
        .reward_invite_referral(user_key, &req.kind, req.amount)
        .await
        .map_err(err_json)?;
    Ok(Json(json!({ "credited": credited })))
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
