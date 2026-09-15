//! billing::currency 通用换算层（0014 symbol/kind/precision + convert）——需要 PG。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test currency_display -- --ignored`
//!
//! 覆盖场景（AGENTS.md 约定：每个测试说明测什么行为、为什么是这个预期）：
//! - 0014 seed：USD/CNY 以 fiat 身份存在，FREE 是 points
//! - seed_for_user 只建 points 余额行——fiat 进余额是语义污染（¥0 行），
//!   这是 0014 修的真 bug 源
//! - available_i64 排除 fiat：手动塞入的 fiat 余额行不折算进可用值
//!   （防御口径：入账路径已拦 fiat，这里是兜底证明）
//! - convert：USD 基准恒等；CNY→USD→CNY floor 误差内往返；未启用/不存在拒绝
//! - upsert_def：fiat 必须带 symbol/precision；USD 基准 rate 锁 1（改它 =
//!   全盘换算口径漂移）
//! - 入账防线：credit_topup 拒绝 fiat（currency_enabled 收窄为 points-only）
//!
//! 每个 PG 测试用独立 user_key/货币 code，结束按 key 清理（对齐 currency_wallet.rs）。

use billing::currency::BillingErr;
use billing::{CurrencyService, WalletService};
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 建 pool + 跑迁移（0014 必须应用）。PG 不可达时 skip（CI 有真 PG）。
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

async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("cd_user_{}", key.simple()))
    .bind("cd test")
    .execute(pool)
    .await
    .expect("insert user");
    key
}

async fn cleanup_user(pool: &sqlx::PgPool, user: Uuid) {
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM auth_users WHERE key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
}

/// 0014 seed：USD/CNY fiat + FREE points。为什么：这是换算层的基准事实，
/// convert 与 seed 过滤都依赖它；迁移没跑对时这里第一时间炸。
#[tokio::test]
async fn seed_fiat_currencies_present() {
    let Some((cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT code, kind FROM currency_defs WHERE code IN ('USD','CNY','FREE') ORDER BY code",
    )
    .fetch_all(&pool)
    .await
    .expect("query defs");
    let kinds: std::collections::HashMap<String, String> = rows.into_iter().collect();
    assert_eq!(
        kinds.get("USD").map(String::as_str),
        Some("fiat"),
        "USD must be fiat (base)"
    );
    assert_eq!(
        kinds.get("CNY").map(String::as_str),
        Some("fiat"),
        "CNY must be fiat"
    );
    assert_eq!(
        kinds.get("FREE").map(String::as_str),
        Some("points"),
        "FREE must be points"
    );
    let _ = cur;
}

/// seed_for_user 只建 points 余额行。为什么：fiat 行是"法币余额"语义污染，
/// 用户会在前端看到 ¥0 余额却永远扣不了它（0014 修的 bug 源）。
#[tokio::test]
async fn seed_skips_fiat() {
    let Some((cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    cur.seed_for_user(user).await.expect("seed");
    let codes: Vec<String> =
        sqlx::query_scalar("SELECT currency_code FROM user_balances WHERE user_key = $1")
            .bind(user)
            .fetch_all(&pool)
            .await
            .expect("query balances");
    assert!(
        codes.iter().all(|c| c == "FREE"),
        "only points currencies may be seeded, got {codes:?}"
    );
    cleanup_user(&pool, user).await;
}

/// available_i64 排除 fiat 行。为什么：入账路径已拦 fiat，但脏数据兜底口径
/// 必须成立——法币不是可扣费余额，混进 SUM 会凭空放大可用额度。
#[tokio::test]
async fn available_excludes_fiat() {
    let Some((cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    // FREE 100 点（rate=1 → 可用 100 内部单位）。
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'FREE', 100)",
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("seed free balance");
    // 模拟脏数据：手动塞 fiat 余额（正常路径到不了这里）。
    sqlx::query(
        "INSERT INTO user_balances (user_key, currency_code, amount) VALUES ($1, 'CNY', 10000)",
    )
    .bind(user)
    .execute(&pool)
    .await
    .expect("seed dirty fiat balance");
    let avail = cur.available_i64(user).await.expect("available");
    assert_eq!(avail, 100, "fiat rows must not count into available_i64");
    cleanup_user(&pool, user).await;
}

/// convert USD 恒等：基准货币换自己必须原样返回。为什么：基准的 rate=1，
/// 任何数值漂移都说明换算实现坏了。
#[tokio::test]
async fn convert_usd_identity() {
    let Some((cur, _wal, _pool)) = make_svcs().await else {
        return;
    };
    assert_eq!(
        cur.convert(500_000, "USD", "USD").await.expect("convert"),
        500_000
    );
}

/// convert CNY→USD→CNY 往返在 floor 误差内。为什么：换算经内部单位中转，
/// 两次 floor 各丢 <1 单位，往返误差必须 ≤2 货币单位（rate≈0.14 时内部
/// 误差 < 1 CNY 单位的量级）。
#[tokio::test]
async fn convert_roundtrip_within_floor_error() {
    let Some((cur, _wal, _pool)) = make_svcs().await else {
        return;
    };
    let usd = cur.convert(100, "CNY", "USD").await.expect("cny->usd");
    assert!(
        usd > 0,
        "positive amount must convert to positive usd, got {usd}"
    );
    let back = cur.convert(usd, "USD", "CNY").await.expect("usd->cny");
    assert!(
        (back - 100).abs() <= 2,
        "roundtrip 100 CNY -> {usd} USD -> {back} CNY drifted beyond floor tolerance"
    );
}

/// convert 拒绝不存在/未启用的货币。为什么：静默 0 会把配置错误伪装成
/// "换出 0 点"，BadRequest 才能把问题暴露给调用方。
#[tokio::test]
async fn convert_rejects_unknown_currency() {
    let Some((cur, _wal, _pool)) = make_svcs().await else {
        return;
    };
    let err = cur
        .convert(100, "NOPE", "USD")
        .await
        .expect_err("must reject");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
}

/// upsert_def 校验 fiat：symbol 必填、precision ≥ 1。为什么：没有符号的法币
/// 在前端没法展示，没有小数位的法币计价精度不够。
#[tokio::test]
async fn upsert_fiat_requires_symbol_and_precision() {
    let Some((cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let err = cur
        .upsert_def("JPN", "Japanese Yen", 0.0067, true, "", "", "fiat", 0)
        .await
        .expect_err("empty symbol must be rejected");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
    let err = cur
        .upsert_def("JPN", "Japanese Yen", 0.0067, true, "", "¥", "fiat", 0)
        .await
        .expect_err("precision 0 fiat must be rejected");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
    let _ = pool;
}

/// USD 基准 rate 锁 1。为什么：全部换算经 internal 单位中转，基准 rate 被改
/// = 全盘换算口径漂移，等价于改写所有用户余额的购买力。
#[tokio::test]
async fn upsert_usd_rate_locked() {
    let Some((cur, _wal, _pool)) = make_svcs().await else {
        return;
    };
    let err = cur
        .upsert_def("USD", "US Dollar", 2.0, true, "", "$", "fiat", 2)
        .await
        .expect_err("base rate change must be rejected");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
}

/// 入账防线：credit_topup 拒绝 fiat。为什么：currency_enabled 收窄为
/// points-only 后，任何把法币当余额入账的调用都在入口被拦。
#[tokio::test]
async fn credit_topup_rejects_fiat() {
    let Some((_cur, wal, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    let err = wal
        .credit_topup(user, "CNY", 100)
        .await
        .expect_err("fiat topup must be rejected");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
    cleanup_user(&pool, user).await;
}

/// points 货币经 upsert_def 启用时给存量用户补 0 余额行；fiat 不补。
/// 为什么：补余额行只对可扣费货币有意义，fiat 补行就是污染（同 seed 语义）。
#[tokio::test]
async fn upsert_seeds_only_points() {
    let Some((cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let user = make_user(&pool).await;
    let pts = format!(
        "TST{}",
        Uuid::new_v4().simple().to_string()[..4].to_uppercase()
    );
    cur.upsert_def(&pts, "Test Points", 1.0, true, "", "T", "points", 0)
        .await
        .expect("upsert points");
    let has_pts: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_balances WHERE user_key = $1 AND currency_code = $2)",
    )
    .bind(user)
    .bind(&pts)
    .fetch_one(&pool)
    .await
    .expect("query balance row");
    assert!(has_pts, "points currency must backfill 0 balance rows");
    // fiat：CNY 已 seed 为 enabled，用户不该有 CNY 行。
    let has_cny: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM user_balances WHERE user_key = $1 AND currency_code = 'CNY')",
    )
    .bind(user)
    .fetch_one(&pool)
    .await
    .expect("query balance row");
    assert!(!has_cny, "fiat must not be backfilled into balances");
    sqlx::query("DELETE FROM currency_defs WHERE code = $1")
        .bind(&pts)
        .execute(&pool)
        .await
        .ok();
    sqlx::query("DELETE FROM user_balances WHERE currency_code = $1")
        .bind(&pts)
        .execute(&pool)
        .await
        .ok();
    cleanup_user(&pool, user).await;
}

/// open_topup 拒绝 fiat 货币。为什么：fiat 永远无法入账（credit_topup 走
/// points-only 校验），开了就是永远 settle 不了的僵尸单——settle 失败事务
/// 回滚、订单退回 pending，可反复重试永远失败。在开单入口拦（0014 复查修复）。
#[tokio::test]
#[ignore = "needs PG; run with DATABASE_URL"]
async fn open_topup_rejects_fiat() {
    let Some((_cur, _wal, pool)) = make_svcs().await else {
        return;
    };
    let topup = billing::TopupService::new(pool.clone());
    let err = topup
        .open_topup(contract::api::billing::TopUpRequest {
            user_key: Uuid::new_v4().to_string(),
            currency: "CNY".to_string(),
            amount: 100,
        })
        .await
        .expect_err("fiat topup order must be rejected at open time");
    assert!(matches!(err, BillingErr::BadRequest(_)), "got {err:?}");
    // 无订单行落库（在入口拒绝，而非建单后卡状态机）
    let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM billing_topups WHERE currency = 'CNY'")
        .fetch_one(&pool)
        .await
        .expect("count orders");
    assert_eq!(n, 0, "no pending fiat order row may be created");
}
