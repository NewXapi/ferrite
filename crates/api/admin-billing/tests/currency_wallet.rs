//! billing::currency + billing::wallet 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test currency_wallet -- --ignored`
//!
//! 覆盖场景（对齐 tests/redeem.rs 的 PG/skip 约定）：
//! - seed_for_user 幂等（调两次行数不变）
//! - available_i64 多货币折算（FREE rate=1 + 某货币 rate=2）
//! - upsert_def 新增 + 内部率校验（<=0/NaN 拒绝）
//! - deduct_by_cost 正常扣 / 不足 clamp 到 0 返回实扣
//! - credit_redeem/credit_topup/credit_reward 的 ON CONFLICT 叠加（两次 = 总和）
//! - balance_view 的 available_i64 与 deduct 扣后一致
//!
//! 每个测试说明测什么行为、为什么是这个预期（AGENTS.md 约定）。

use billing::{CurrencyService, WalletService};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 建 pool + 跑迁移 + 建两个服务（currency/wallet 共享 pool）。
/// 清理：测试自建 user_key/currency_code，结束时按 key 删 user_balances，
/// 删测试 currency_defs（FREE 不动，它是 seed 的）。
async fn make_svcs() -> Option<(CurrencyService, WalletService, sqlx::PgPool)> {
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
        CurrencyService::new(pool.clone()),
        WalletService::new(pool.clone()),
        pool,
    ))
}

/// 建测试用户 + 清掉该用户的所有余额行（幂等前置）。
async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("cw_user_{}", key.simple()))
    .bind("cw test")
    .execute(pool)
    .await
    .expect("insert user");
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(key)
        .execute(pool)
        .await
        .ok();
    key
}

/// 清理测试货币与用户余额。
async fn cleanup(pool: &sqlx::PgPool, user: Uuid, extra_code: &str) {
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    if extra_code != "FREE" {
        sqlx::query("DELETE FROM currency_defs WHERE code = $1")
            .bind(extra_code)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM user_balances WHERE currency_code = $1")
            .bind(extra_code)
            .execute(pool)
            .await
            .ok();
    }
}

/// 建一个启用的测试货币（非 FREE），返回其 code。
async fn make_currency(pool: &sqlx::PgPool, code: &str, rate: f64) {
    sqlx::query(
        "INSERT INTO currency_defs (code, name, internal_rate, enabled) VALUES ($1, $1, $2, true)
         ON CONFLICT (code) DO UPDATE SET internal_rate = $2, enabled = true",
    )
    .bind(code)
    .bind(rate)
    .execute(pool)
    .await
    .ok();
}

/// seed_for_user 幂等：调两次，该用户的余额行数不变（ON CONFLICT DO NOTHING）。
#[tokio::test]
async fn seed_for_user_idempotent() {
    let Some((cur, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    cur.seed_for_user(user).await.expect("seed 1");
    cur.seed_for_user(user).await.expect("seed 2");
    // 行数 = 启用货币数；seed 重复不叠加、不报错。
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM user_balances WHERE user_key = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    let enabled: i64 = sqlx::query_scalar("SELECT count(*) FROM currency_defs WHERE enabled")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, enabled,
        "seed 两次后余额行数应等于启用货币数（幂等）"
    );
    cleanup(&pool, user, "FREE").await;
}

/// available_i64 多货币折算：FREE(rate=1) 有 100 + TEST_RATE2(rate=2) 有 30
/// → 综合可用 = 100×1 + 30×2 = 160。
#[tokio::test]
async fn available_i64_multi_currency() {
    let Some((cur, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    cur.seed_for_user(user).await.expect("seed");
    make_currency(&pool, "TEST_RATE2", 2.0).await;
    // 先补 seed（新货币要 seed 一行 0），再直接设余额。
    cur.seed_for_user(user).await.expect("reseed");
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'FREE', 100)
         ON CONFLICT (user_key, currency_code) DO UPDATE SET amount = 100",
    )
    .bind(user)
    .execute(&pool)
    .await
    .ok();
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'TEST_RATE2', 30)
         ON CONFLICT (user_key, currency_code) DO UPDATE SET amount = 30",
    )
    .bind(user)
    .execute(&pool)
    .await
    .ok();

    let available = cur.available_i64(user).await.expect("available");
    assert_eq!(
        available, 160,
        "100×1 + 30×2 = 160（按 internal_rate 折算求和）"
    );
    cleanup(&pool, user, "TEST_RATE2").await;
}

/// upsert_def 新增货币 + internal_rate 校验（<=0 / NaN 拒绝）。
#[tokio::test]
async fn upsert_def_new_and_rate_validation() {
    let Some((cur, _wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;

    // 合法新增
    let view = cur
        .upsert_def("TEST_NEW", "Test New", 3.0, true, "")
        .await
        .expect("upsert ok");
    assert_eq!(view.code, "TEST_NEW");
    assert_eq!(view.internal_rate, 3.0);

    // rate <= 0 拒绝
    assert!(
        cur.upsert_def("TEST_NEW", "bad", 0.0, true, "")
            .await
            .is_err(),
        "rate=0 应拒绝"
    );
    // rate NaN 拒绝
    assert!(
        cur.upsert_def("TEST_NEW", "bad", f64::NAN, true, "")
            .await
            .is_err(),
        "rate=NaN 应拒绝"
    );

    cleanup(&pool, user, "TEST_NEW").await;
}

/// deduct_by_cost 正常扣：FREE(rate=1) 余额 1000，扣 300 → 剩 700，实扣 300，fully=true。
#[tokio::test]
async fn deduct_by_cost_normal() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'FREE', 1000)
         ON CONFLICT (user_key, currency_code) DO NOTHING",
    )
    .bind(user)
    .execute(&pool)
    .await
    .ok();

    let (deducted, fully) = wallet.deduct_by_cost(user, 300).await.expect("deduct");
    assert_eq!(deducted, 300, "FREE rate=1 扣 300 内部单位应实扣 300");
    assert!(fully, "余额充足应 fully=true");

    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 700, "扣后余额 1000-300=700");
    cleanup(&pool, user, "FREE").await;
}

/// deduct_by_cost 不足 clamp：FREE 余额 100，扣 1000 → clamp 到 0，实扣 100，fully=false。
#[tokio::test]
async fn deduct_by_cost_clamp() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'FREE', 100)
         ON CONFLICT (user_key, currency_code) DO NOTHING",
    )
    .bind(user)
    .execute(&pool)
    .await
    .ok();

    let (deducted, fully) = wallet.deduct_by_cost(user, 1000).await.expect("deduct");
    assert_eq!(deducted, 100, "余额不足应 clamp 到实际可用 100");
    assert!(!fully, "扣不够应 fully=false（余账下请求准入拦截）");

    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 0, "clamp 后余额归 0，不为负");
    cleanup(&pool, user, "FREE").await;
}

/// credit_redeem ON CONFLICT 叠加：两次入账 = 总和（幂等叠加，非覆盖）。
#[tokio::test]
async fn credit_redeem_accumulates() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;

    let after1 = wallet.credit_redeem(user, 500).await.expect("credit1");
    let after2 = wallet.credit_redeem(user, 300).await.expect("credit2");
    assert_eq!(after1, 500, "首次入账后余额 500");
    assert_eq!(after2, 800, "二次入账后 500+300=800（叠加）");

    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 800, "DB 余额应为两次叠加总和");
    cleanup(&pool, user, "FREE").await;
}

/// credit_topup 指定货币入账 + 未启用货币拒绝。
#[tokio::test]
async fn credit_topup_currency() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    make_currency(&pool, "TEST_PAID", 5.0).await;

    let v = wallet
        .credit_topup(user, "TEST_PAID", 10)
        .await
        .expect("topup ok");
    assert_eq!(v, 10, "TEST_PAID 入账 10（该货币单位）");

    // 不存在的货币 → 拒绝
    assert!(
        wallet.credit_topup(user, "NOPE", 10).await.is_err(),
        "未启用货币应拒绝"
    );

    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'TEST_PAID'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 10);
    cleanup(&pool, user, "TEST_PAID").await;
}

/// balance_view：各货币余额 + available_i64 一致；deduct 后 view 更新。
#[tokio::test]
async fn balance_view_reflects_deduct() {
    let Some((cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    cur.seed_for_user(user).await.expect("seed");
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'FREE', 1000)
         ON CONFLICT (user_key, currency_code) DO UPDATE SET amount = 1000",
    )
    .bind(user)
    .execute(&pool)
    .await
    .ok();

    let v = wallet.balance_view(user).await.expect("view1");
    assert_eq!(v.available_i64, 1000, "初始 view available=1000");

    wallet.deduct_by_cost(user, 250).await.expect("deduct");

    let v2 = wallet.balance_view(user).await.expect("view2");
    assert_eq!(v2.available_i64, 750, "扣 250 后 view available=750");
    assert!(
        v2.balances
            .iter()
            .any(|b| b.currency_code == "FREE" && b.amount == 750),
        "FREE 余额应反映为 750"
    );
    cleanup(&pool, user, "FREE").await;
}
