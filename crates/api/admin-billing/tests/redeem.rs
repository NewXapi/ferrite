//! billing::redeem 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test redeem -- --ignored`

use billing::RedeemService;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svc() -> (RedeemService, sqlx::PgPool) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    billing::ensure_table(&pool).await.expect("ddl");
    (RedeemService::new(pool.clone()), pool)
}

/// 建一个测试用户（直接 SQL，独立于 auth 集成测试的清理策略）。
async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("redeem_user_{}", key.simple()))
    .bind("redeem test")
    .execute(pool)
    .await
    .expect("insert user");
    key
}

/// 生成 → 兑换 → 余额增加；并发核销同一码只有一个成功。
#[tokio::test]
#[ignore]
async fn redeem_generate_and_redeem_flow() {
    let (svc, pool) = make_svc().await;
    let user_key = make_user(&pool).await;

    // 生成 3 张 500 额度的码
    let codes = svc.generate(500, 3).await.expect("generate");
    assert_eq!(codes.len(), 3);
    assert!(codes.iter().all(|c| c.starts_with("fx-")));

    // 核销第一张 → 返回 500
    let got = svc.redeem(&codes[0], user_key).await.expect("redeem");
    assert_eq!(got, 500);

    // 重复核销同一张 → NotFound（已用）
    let again = svc.redeem(&codes[0], user_key).await;
    assert!(matches!(again, Err(auth::error::AuthError::NotFound(_))));

    // 用户 quota = 500
    let (balance,): (i64,) =
        sqlx::query_as("SELECT quota FROM auth_users WHERE key = $1")
            .bind(user_key)
            .fetch_one(&pool)
            .await
            .expect("fetch user");
    assert_eq!(balance, 500);
}

/// 并发核销同一码 → 只有一个成功（CAS 语义）。
#[tokio::test]
#[ignore]
async fn redeem_concurrent_single_winner() {
    let (svc, pool) = make_svc().await;
    let user_key = make_user(&pool).await;
    let codes = svc.generate(100, 1).await.expect("generate");
    let code = codes[0].clone();

    let svc1 = std::sync::Arc::new(billing::RedeemService::new(pool.clone()));
    let svc2 = std::sync::Arc::clone(&svc1);
    let (r1, r2) = tokio::join!(
        svc1.redeem(&code, user_key),
        svc2.redeem(&code, user_key),
    );
    // 恰好一个成功
    assert!(r1.is_ok() != r2.is_ok(), "exactly one redeem must win");
}

/// 非法输入：quota<=0 拒绝、空码拒绝。
#[tokio::test]
#[ignore]
async fn redeem_validation_rejected() {
    let (svc, _pool) = make_svc().await;
    assert!(svc.generate(0, 1).await.is_err());
    assert!(svc.generate(-5, 1).await.is_err());
    assert!(svc.redeem("", Uuid::new_v4()).await.is_err());
}

/// admin 列表分页 + 禁用。
#[tokio::test]
#[ignore]
async fn redeem_list_and_disable() {
    let (svc, pool) = make_svc().await;
    let _ = make_user(&pool).await;
    let codes = svc.generate(50, 2).await.expect("generate");

    let (items, _total) = svc.list(Some(1), 1, 50).await.expect("list");
    assert!(items.len() >= 2);

    // 禁用一张（按 preview 找 key）
    let key = items
        .iter()
        .find(|i| codes.iter().any(|c| i.code_preview == format!("fx-{}****{}", &c[3..7], &c[c.len()-4..])))
        .map(|i| i.key.clone())
        .expect("find generated key");
    svc.disable(uuid::Uuid::parse_str(&key).unwrap()).await.expect("disable");

    // 禁用后兑换 → NotFound
    let plain = codes.iter().find(|c| items.iter().any(|i| i.code_preview == format!("fx-{}****{}", &c[3..7], &c[c.len()-4..]))).unwrap();
    let r = svc.redeem(plain, Uuid::new_v4()).await;
    assert!(matches!(r, Err(auth::error::AuthError::NotFound(_))));
}
