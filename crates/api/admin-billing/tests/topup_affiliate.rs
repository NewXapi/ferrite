//! billing::topup + billing::affiliate 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test topup_affiliate -- --ignored`
//!
//! 覆盖场景：
//! - open_topup 建 pending 订单（货币启用校验）
//! - settle_topup CAS 幂等：首次 pending→settling→paid 入金；二次 settle 拒绝（already settled）
//! - settle 非 pending 订单拒绝
//! - credit_reward 拉人奖励入账 FREE
//! - user_overview 占位（当前返回 0，affiliate_links 表未建，TODO 挂账）
//!
//! 每个测试说明测什么行为、为什么是这个预期。

use billing::{AffiliateService, TopupService, WalletService};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svcs() -> (TopupService, AffiliateService, WalletService, sqlx::PgPool) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("migrations");
    (
        TopupService::new(pool.clone()),
        AffiliateService::new(pool.clone(), WalletService::new(pool.clone())),
        WalletService::new(pool.clone()),
        pool,
    )
}

async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("ta_user_{}", key.simple()))
    .bind("ta test")
    .execute(pool)
    .await
    .expect("insert user");
    key
}

/// 清理某用户所有 topup 订单 + 余额行。
async fn cleanup(pool: &sqlx::PgPool, user: Uuid) {
    sqlx::query("DELETE FROM billing_topups WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
}

/// 直接插一条 pending 订单（绕过 open_topup，专注 settle 的 CAS 语义）。
async fn make_pending_order(
    pool: &sqlx::PgPool,
    user: Uuid,
    currency: &str,
    amount: i64,
) -> String {
    let key = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO billing_topups (key, user_key, currency, amount, state, provider)
           VALUES ($1, $2, $3, $4, 'pending', '')"#,
    )
    .bind(&key)
    .bind(user)
    .bind(currency)
    .bind(amount)
    .execute(pool)
    .await
    .expect("insert order");
    key
}

/// open_topup：启用货币建 pending 订单，返回 order id；未启用货币拒绝。
#[tokio::test]
#[ignore]
async fn open_topup_validates_currency() {
    let (topup, _aff, _wallet, pool) = make_svcs().await;
    let user = make_user(&pool).await;

    // FREE 默认 seed 启用 → 开单成功
    let req = contract::api::billing::TopUpRequest {
        user_key: user.to_string(),
        currency: "FREE".into(),
        amount: 100,
    };
    let order_id = topup.open_topup(req.clone()).await.expect("open FREE");
    assert!(!order_id.is_empty());

    // 不存在的货币 → 拒绝
    let bad = contract::api::billing::TopUpRequest {
        user_key: user.to_string(),
        currency: "NOPE".into(),
        amount: 100,
    };
    assert!(topup.open_topup(bad).await.is_err(), "未启用货币应拒绝开单");

    cleanup(&pool, user).await;
}

/// settle_topup CAS 幂等：
/// - 首次 pending → settling → paid，入金 FREE
/// - 二次 settle 同订单 → 拒绝（already settled，不重复入金）
/// 预期：入金只发生一次，第二次是 no-op 报错。
#[tokio::test]
#[ignore]
async fn settle_topup_cas_idempotent() {
    let (topup, _aff, _wallet, pool) = make_svcs().await;
    let user = make_user(&pool).await;
    let order = make_pending_order(&pool, user, "FREE", 500).await;

    // 首次 settle：入金 500
    let credited = topup.settle_topup(&order).await.expect("settle1");
    assert_eq!(credited, 500, "首次 settle 实入账 500");

    // 订单状态应为 paid
    let state: String = sqlx::query_scalar("SELECT state FROM billing_topups WHERE key = $1")
        .bind(&order)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(state, "paid", "settle 后订单转 paid");

    // 二次 settle：拒绝（CAS pending→settling 已不成立）
    assert!(
        topup.settle_topup(&order).await.is_err(),
        "二次 settle 应拒绝（防重复入金）"
    );

    // 余额仍是 500（未被二次扣加）
    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 500, "余额只入账一次");

    cleanup(&pool, user).await;
}

/// settle 不存在的订单 → 拒绝。
#[tokio::test]
#[ignore]
async fn settle_topup_missing_order() {
    let (topup, _aff, _wallet, pool) = make_svcs().await;
    let user = make_user(&pool).await;
    let missing = Uuid::new_v4().to_string();
    assert!(
        topup.settle_topup(&missing).await.is_err(),
        "不存在的订单 settle 应拒绝"
    );
    cleanup(&pool, user).await;
}

/// credit_reward：拉人奖励入账 FREE（kind 区分来源），两次叠加。
#[tokio::test]
#[ignore]
async fn credit_reward_accumulates() {
    let (_topup, _aff, wallet, pool) = make_svcs().await;
    let user = make_user(&pool).await;

    let v1 = wallet
        .credit_reward("invite", user, 1_000_000)
        .await
        .expect("r1");
    let v2 = wallet
        .credit_reward("invite", user, 1_000_000)
        .await
        .expect("r2");
    assert_eq!(v1, 1_000_000, "首次奖励入账后余额");
    assert_eq!(
        v2, 2_000_000,
        "二次叠加 = 2,000,000（默认 $2 @ 500_000/$1）"
    );

    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 2_000_000, "DB 余额为两次奖励叠加");
    cleanup(&pool, user).await;
}

/// user_overview 占位：affiliate_links 表未建前返回 0（不是报错，是诚实占位）。
#[tokio::test]
#[ignore]
async fn user_overview_placeholder() {
    let (_topup, aff, _wallet, pool) = make_svcs().await;
    let user = make_user(&pool).await;
    let overview = aff.user_overview(user).await.expect("overview");
    assert_eq!(
        overview.invite_count, 0,
        "affiliate_links 表未建，invite_count 诚实占位 0（TODO）"
    );
    assert_eq!(overview.total_reward, 0, "同上，total_reward 占位 0");
    cleanup(&pool, user).await;
}
