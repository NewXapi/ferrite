//! billing::currency + billing::wallet 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test currency_wallet -- --ignored`
//!
//! 覆盖场景（对齐 tests/redeem.rs 的 PG/skip 约定）：
//! - seed_for_user 幂等（调两次行数不变）
//! - available_i64 多货币折算（FREE rate=1 + 某货币 rate=2）
//! - upsert_def 新增 + 内部率校验（<=0/NaN 拒绝）
//! - deduct_by_cost_group 正常扣 / 不足 clamp 到 0 返回实扣
//! - credit_redeem/credit_topup/credit_reward 的 ON CONFLICT 叠加（两次 = 总和）
//! - balance_view 的 available_i64 与 deduct 扣后一致
//! - 组差异化倍率（0011）：available/deduct 同口径按 group_rates 折算
//! - 奖励冻结（0012）：冻结不入可用、不可被扣、到期 thaw 幂等搬回
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
    sqlx::query("DELETE FROM affiliate_rewards WHERE inviter_key = $1 OR invitee_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
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

/// 直接灌 FREE 余额（frozen 清零），跳过 credit 路径，供冻结/倍率场景搭前置。
async fn set_balance(pool: &sqlx::PgPool, user: Uuid, amount: i64) {
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount, frozen_amount)
         VALUES ($1, 'FREE', $2, 0)
         ON CONFLICT (user_key, currency_code) DO UPDATE SET amount = $2, frozen_amount = 0",
    )
    .bind(user)
    .bind(amount)
    .execute(pool)
    .await
    .ok();
}

/// 设置 FREE 货币的组倍率 JSONB（0011）。只有显式传组名的测试会读到倍率，
/// 其余测试走 None/缺省组恒 1.0，并行跑互不影响；用完须复位 "{}"。
async fn set_group_rates(pool: &sqlx::PgPool, rates: &str) {
    sqlx::query("UPDATE currency_defs SET group_rates = $1::jsonb WHERE code = 'FREE'")
        .bind(rates)
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
        .upsert_def("TEST_NEW", "Test New", 3.0, true, "", "T", "points", 0)
        .await
        .expect("upsert ok");
    assert_eq!(view.code, "TEST_NEW");
    assert_eq!(view.internal_rate, 3.0);

    // rate <= 0 拒绝
    assert!(
        cur.upsert_def("TEST_NEW", "bad", 0.0, true, "", "T", "points", 0)
            .await
            .is_err(),
        "rate=0 应拒绝"
    );
    // rate NaN 拒绝
    assert!(
        cur.upsert_def("TEST_NEW", "bad", f64::NAN, true, "", "T", "points", 0)
            .await
            .is_err(),
        "rate=NaN 应拒绝"
    );

    cleanup(&pool, user, "TEST_NEW").await;
}

/// deduct_by_cost_group 正常扣（None 组）：FREE(rate=1) 余额 1000，扣 300 → 剩 700，实扣 300，fully=true。
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

    let (deducted, fully) = wallet.deduct_by_cost_group(user, 300, None).await.expect("deduct");
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

/// deduct_by_cost_group 不足 clamp（None 组）：FREE 余额 100，扣 1000 → clamp 到 0，实扣 100，fully=false。
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

    let (deducted, fully) = wallet.deduct_by_cost_group(user, 1000, None).await.expect("deduct");
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

    let v = wallet.balance_view(user, None).await.expect("view1");
    assert_eq!(v.available_i64, 1000, "初始 view available=1000");

    wallet.deduct_by_cost_group(user, 250, None).await.expect("deduct");

    let v2 = wallet.balance_view(user, None).await.expect("view2");
    assert_eq!(v2.available_i64, 750, "扣 250 后 view available=750");
    assert!(
        v2.balances
            .iter()
            .any(|b| b.currency_code == "FREE" && b.amount == 750),
        "FREE 余额应反映为 750"
    );
    cleanup(&pool, user, "FREE").await;
}

/// 组倍率折扣：FREE 余额 1000，currency_defs.group_rates = {"vip": 0.8}。
/// 预期：available_i64("vip") = 800（×0.8 缩小可见余额）；"__unset__"（不区分组）
/// 与未配置组 "gold"（倍率缺省 1.0）= 1000——缺组行为必须与 0011 之前完全一致
/// （迁移 `'{}'` 默认 = 全员无倍率，no-op 保证）。
#[tokio::test]
async fn group_rate_discounts_available() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    set_balance(&pool, user, 1000).await;
    set_group_rates(&pool, r#"{"vip": 0.8}"#).await;

    assert_eq!(
        wallet.available_i64(user, "vip").await.expect("vip"),
        800,
        "vip 组 1000×1×0.8 = 800"
    );
    assert_eq!(
        wallet.available_i64(user, "__unset__").await.expect("none"),
        1000,
        "None = 不区分组，倍率恒 1.0"
    );
    assert_eq!(
        wallet
            .available_i64(user, "gold")
            .await
            .expect("gold"),
        1000,
        "未配置倍率的组缺省 1.0"
    );

    set_group_rates(&pool, "{}").await;
    cleanup(&pool, user, "FREE").await;
}

/// 冻结奖励闭环：底余额 1000，credit_reward_frozen(500, 5h) 冻结入账。
/// 预期：amount=1500 但 available 仍 1000（冻结不增加购买力）；到期前
/// thaw_frozen 返回 0 且可用不变——到期判定按「奖励行 frozen_until」粒度
/// （0012），不按余额行；把跟踪行强制过期 1h 后 thaw 搬回 500，可用 1500；
/// 跟踪行 frozen_until 已清 NULL = 幂等标记，二次 thaw 返回 0 不双搬。
#[tokio::test]
async fn frozen_reward_not_available_until_thaw() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    set_balance(&pool, user, 1000).await;

    let v = wallet
        .credit_reward_frozen("activity", user, 500, Some(5))
        .await
        .expect("freeze credit");
    assert_eq!(v, 1500, "入账后余额（货币单位）含冻结 1500");
    assert_eq!(
        wallet.available_i64(user, "__unset__").await.expect("avail"),
        1000,
        "冻结入账不改变可用（购买力不变，只是记账）"
    );

    assert_eq!(
        wallet.thaw_frozen(user).await.expect("thaw early"),
        0,
        "未到期的冻结不搬"
    );
    assert_eq!(
        wallet.available_i64(user, "__unset__").await.expect("avail2"),
        1000,
        "提前 thaw 后可用不变"
    );

    sqlx::query(
        "UPDATE affiliate_rewards SET frozen_until = now() - interval '1 hour'
         WHERE inviter_key = $1 AND frozen_until IS NOT NULL",
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("force due");
    assert_eq!(
        wallet.thaw_frozen(user).await.expect("thaw"),
        500,
        "到期解冻全额搬回可用"
    );
    assert_eq!(
        wallet.available_i64(user, "__unset__").await.expect("avail3"),
        1500,
        "解冻后 frozen=0，amount 1500 全可用"
    );
    assert_eq!(
        wallet.thaw_frozen(user).await.expect("thaw again"),
        0,
        "幂等：解冻过的行不再被搬（frozen_until 已清）"
    );
    cleanup(&pool, user, "FREE").await;
}

/// 冻结不重复计：直接置 amount=1500、frozen_amount=500（模拟冻结中状态）。
/// 预期：available_i64 = 1000——若可用含冻结，入账瞬间余额虚增；
/// 扣 cost 1200 只实扣 1000 且 fully=false，扣后 (amount, frozen) = (500, 500)
/// ——扣减 CAS 走 `amount - frozen_amount >= n`，冻结的钱网关绝花不到。
#[tokio::test]
async fn frozen_does_not_double_count() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    set_balance(&pool, user, 1500).await;
    sqlx::query(
        "UPDATE user_balances SET frozen_amount = 500
         WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("freeze");

    assert_eq!(
        wallet.available_i64(user, "__unset__").await.expect("avail"),
        1000,
        "available = amount - frozen = 1000（不是 1500）"
    );
    let (deducted, fully) = wallet.deduct_by_cost_group(user, 1200, None).await.expect("deduct");
    assert_eq!(deducted, 1000, "只能扣非冻结的 1000");
    assert!(!fully, "冻结部分不可扣，扣不足应 fully=false");

    let (amount, frozen): (i64, i64) = sqlx::query_as(
        "SELECT amount, frozen_amount FROM user_balances
         WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        (amount, frozen),
        (500, 500),
        "扣减只消耗可用部分，冻结分毫未动"
    );
    cleanup(&pool, user, "FREE").await;
}

/// 组倍率参与扣费折算（与 available 同口径）：FREE 1000，{"vip":0.8}，
/// vip 组扣 cost 800：有效折算率 = 1×0.8，可用内部单位恰 800 → 扣干净
/// fully=true，货币消耗 800/0.8 = 1000（向上取整），余额归零。
/// 对照：None 组同余额扣 800 只剩 200（倍率不参与，行为 = 0011 之前）。
/// 语义选择说明：倍率既缩可见余额也放大单位消耗——若只影响 available
/// 判断而扣费仍按原 rate 计，同一余额「按 0.8 判可用、按 1.0 真扣」两套口径
/// 撕裂，vip 用户能多花 25% 的钱，等于倍率形同虚设。
#[tokio::test]
async fn deduct_with_group_rate() {
    let Some((_cur, wallet, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    set_balance(&pool, user, 1000).await;
    set_group_rates(&pool, r#"{"vip": 0.8}"#).await;

    let (deducted, fully) = wallet
        .deduct_by_cost_group(user, 800, Some("vip"))
        .await
        .expect("deduct vip");
    assert_eq!(deducted, 800, "vip 可用内部单位 = 1000×0.8 = 800");
    assert!(fully, "cost 800 恰等于可用应扣干净");
    let bal: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal, 0, "800 内部单位 / 0.8 = 1000 货币单位，余额归零");

    // 对照：复位余额后不带组，同 cost 只消耗 800 货币单位（倍率不生效）。
    set_balance(&pool, user, 1000).await;
    let (d2, f2) = wallet.deduct_by_cost_group(user, 800, None).await.expect("deduct none");
    assert_eq!(d2, 800);
    assert!(f2);
    let bal2: i64 = sqlx::query_scalar(
        "SELECT amount FROM user_balances WHERE user_key = $1 AND currency_code = 'FREE'",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bal2, 200, "不带组倍率 1.0，扣 800 单位剩 200");

    set_group_rates(&pool, "{}").await;
    cleanup(&pool, user, "FREE").await;
}
