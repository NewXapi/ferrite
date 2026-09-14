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
//! - bind_inviter 幂等 / 拒绝自邀请（0009 affiliate_links）
//! - reward_invite_referral 同一 invitee 只领一次、无绑定关系拒绝、入账与审计同事务
//! - user_overview 真实统计（affiliate_links COUNT + affiliate_rewards SUM）
//!
//! 每个测试说明测什么行为、为什么是这个预期。

use billing::currency::BillingErr;
use billing::{AffiliateService, TopupService, WalletService};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 建服务与池；**PG 不可达返回 `None` 让调用方 skip**。
///
/// 为什么不是 `expect` + `#[ignore]`：CI 跑的是 `cargo test -p <pkg>`（见
/// `scripts/ci-affected.sh`），**不带 `--ignored`**——`#[ignore]` 的测试在 CI
/// 上永远不执行，等于没写。改成 skip 模式后，有 PG 的环境（CI service
/// container / 本地 dev 库）真跑断言，无 PG 的环境静默跳过。
/// 与 `apps/api/tests/billing_authority.rs::pg_pool` 同款约定。
async fn make_svcs() -> Option<(TopupService, AffiliateService, WalletService, sqlx::PgPool)> {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .map_err(|e| eprintln!("skipping: postgres unreachable at {}: {e}", db_url()))
        .ok()?;
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("migrations must apply once PG is reachable");
    Some((
        TopupService::new(pool.clone()),
        AffiliateService::new(pool.clone(), WalletService::new(pool.clone())),
        WalletService::new(pool.clone()),
        pool,
    ))
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

/// 清理某用户全部 billing/affiliate 痕迹（topup 订单、余额行、邀请关系、奖励审计）。
/// 邀请两表按 inviter/invitee 双角色删——测试里同一用户常兼两角。
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
    sqlx::query("DELETE FROM affiliate_links WHERE inviter_key = $1 OR invitee_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM affiliate_rewards WHERE inviter_key = $1 OR invitee_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
}

/// 用户 FREE 余额；无行按 0（make_user 直插 auth_users，不走注册 seed hook）。
async fn free_balance(pool: &sqlx::PgPool, user: Uuid) -> i64 {
    sqlx::query_scalar(
        "SELECT COALESCE((SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'), 0)",
    )
    .bind(user)
    .fetch_one(pool)
    .await
    .expect("free balance")
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
async fn open_topup_validates_currency() {
    let Some((topup, _aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
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
async fn settle_topup_cas_idempotent() {
    let Some((topup, _aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
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
async fn settle_topup_missing_order() {
    let Some((topup, _aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
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
async fn credit_reward_accumulates() {
    let Some((_topup, _aff, wallet, pool)) = make_svcs().await else {
        return;
    };
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

/// bind_inviter 幂等：同一 invitee 绑两次（含换人重绑）。
/// 预期：首次 true（新建）、二次 false（invitee_key PK + ON CONFLICT DO
/// NOTHING——归属先到先得，重复调用/他人抢绑都不覆盖首邀人），表里仍 1 行。
#[tokio::test]
async fn bind_inviter_idempotent() {
    let Some((_topup, aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let invitee = make_user(&pool).await;
    let other = make_user(&pool).await;

    assert!(
        aff.bind_inviter(inviter, invitee).await.expect("bind1"),
        "首次绑定应成功"
    );
    assert!(
        !aff.bind_inviter(inviter, invitee).await.expect("bind2"),
        "同一 invitee 二次绑定返回 false（幂等，不报错）"
    );
    assert!(
        !aff.bind_inviter(other, invitee).await.expect("bind3"),
        "换邀请人重绑同 invitee 返回 false（一人一主）"
    );
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM affiliate_links WHERE invitee_key = $1")
            .bind(invitee)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 1, "被邀人恒一行归属记录");
    let held_by: Uuid =
        sqlx::query_scalar("SELECT inviter_key FROM affiliate_links WHERE invitee_key = $1")
            .bind(invitee)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(held_by, inviter, "首邀人保留，不被后来者覆盖");

    for u in [inviter, invitee, other] {
        cleanup(&pool, u).await;
    }
}

/// bind_inviter 拒绝自邀请：inviter == invitee → Err(BadRequest)。
/// 自邀是刷奖路径，必须在入口拦下且表里不留行（不是靠 PK 兜底成 no-op）。
#[tokio::test]
async fn bind_inviter_rejects_self() {
    let Some((_topup, aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;

    let e = aff
        .bind_inviter(user, user)
        .await
        .expect_err("自邀请必须报错而非静默 no-op");
    assert!(
        matches!(e, BillingErr::BadRequest(_)),
        "期望 BadRequest，实际 {e:?}"
    );
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM affiliate_links WHERE invitee_key = $1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "自邀请不写归属行");

    cleanup(&pool, user).await;
}

/// reward_invite_referral 幂等 + 事务原子性：同一 invitee 领两次。
/// 预期：首次返回入账额 r1(>0)，二次返回 0；affiliate_rewards 仍 1 行且
/// 行内 amount == 余额实际增量（审计行数 == 余额增量次数 → 入账与审计
/// 同事务成对，不存在「加了钱没审计」或「有审计没加钱」）。
#[tokio::test]
async fn reward_invite_referral_once_per_invitee() {
    let Some((_topup, aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let invitee = make_user(&pool).await;
    aff.bind_inviter(inviter, invitee).await.expect("bind");

    let before = free_balance(&pool, inviter).await;
    let r1 = aff
        .reward_invite_referral(inviter, invitee)
        .await
        .expect("首次领奖");
    assert!(r1 > 0, "首次领奖应入账，实际 {r1}");
    let r2 = aff
        .reward_invite_referral(inviter, invitee)
        .await
        .expect("二次领奖应幂等成功（不是错误）");
    assert_eq!(r2, 0, "同一 invitee 只能领一次，二次返回 0");

    let after = free_balance(&pool, inviter).await;
    assert_eq!(after - before, r1, "余额只加了首次那一份");
    let audit: Vec<(i64,)> = sqlx::query_as(
        "SELECT amount FROM affiliate_rewards WHERE kind = 'invite' AND invitee_key = $1",
    )
    .bind(invitee)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audit.len(), 1, "审计行仍 1 条（与余额增量次数一致）");
    assert_eq!(audit[0].0, r1, "审计金额 == 实际入账额（同事务，不可分家）");

    for u in [inviter, invitee] {
        cleanup(&pool, u).await;
    }
}

/// reward_invite_referral 无绑定关系 → Err(BadRequest)。
/// 选 Err 而非返回 0：缺关系是调用方误用（应先 bind），返回 0 会和
/// 「已领过」的合法幂等混在一起吞掉 bug。报错路径事务回滚：
/// 余额为 0、审计 0 行。
#[tokio::test]
async fn reward_without_link_rejected() {
    let Some((_topup, aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let stranger = make_user(&pool).await;

    let e = aff
        .reward_invite_referral(inviter, stranger)
        .await
        .expect_err("未绑定直接领奖应报错");
    assert!(
        matches!(e, BillingErr::BadRequest(_)),
        "期望 BadRequest，实际 {e:?}"
    );
    assert_eq!(free_balance(&pool, inviter).await, 0, "报错路径不入账");
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM affiliate_rewards WHERE inviter_key = $1")
            .bind(inviter)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "报错路径不留审计行（事务整体回滚）");

    for u in [inviter, stranger] {
        cleanup(&pool, u).await;
    }
}

/// user_overview 真实统计：绑 2 个被邀人 + 对其中 1 人领奖。
/// 预期 invite_count=2、total_reward==首次入账额——断言统计真的查了两张
/// 表（占位版恒 0/0 区分不出回归）。另查未获奖的被邀人 b：统计为 0/0，
/// 断言 inviter 维度的行不污染他人总览。
#[tokio::test]
async fn user_overview_real_stats() {
    let Some((_topup, aff, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let a = make_user(&pool).await;
    let b = make_user(&pool).await;
    aff.bind_inviter(inviter, a).await.expect("bind a");
    aff.bind_inviter(inviter, b).await.expect("bind b");
    let r = aff
        .reward_invite_referral(inviter, a)
        .await
        .expect("reward a");

    let ov = aff.user_overview(inviter).await.expect("overview");
    assert_eq!(ov.invite_count, 2, "绑了 2 个被邀人");
    assert_eq!(ov.total_reward, r, "累计奖励 = 领奖一次的实际入账额");

    let ov_b = aff.user_overview(b).await.expect("overview b");
    assert_eq!(ov_b.invite_count, 0, "被邀人自己没邀人，计数为 0");
    assert_eq!(ov_b.total_reward, 0, "inviter 维度的奖励行不计入他人总览");

    for u in [inviter, a, b] {
        cleanup(&pool, u).await;
    }
}
