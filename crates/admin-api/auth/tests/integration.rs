//! 端到端集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite cargo test -p auth --test integration -- --ignored --nocapture`
//! 集成测试默认 `#[ignore]`，需要 `--ignored` 才跑。

use std::time::Duration;

use auth::service::AuthService;
use sqlx::postgres::PgPoolOptions;
static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}


async fn make_service() -> AuthService {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    auth::ddl::run(&pool).await.expect("ddl");
    AuthService::new(pool, b"test-secret-must-be-long-enough-32!".to_vec())
}

fn unique_user(prefix: &str) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{prefix}_{ns}")
}

#[tokio::test]
#[ignore]
async fn end_to_end_login_flow() {
    let svc = make_service().await;
    let username = unique_user("alice");

    let view = svc
        .register(&username, "hunter2hunter", Some(&format!("{username}@x.com")))
        .await
        .expect("register");
    assert_eq!(view.username, username);
    assert_eq!(view.role, 1);

    // wrong password
    let bad = svc.login(&username, "nope", "ua", "127.0.0.1").await;
    assert!(matches!(bad, Err(auth::AuthError::InvalidCredentials)));

    // good password → token + refresh
    let login = svc.login(&username, "hunter2hunter", "ua", "127.0.0.1").await.expect("login");
    assert_eq!(login.user.username, username);
    assert!(!login.access_token.is_empty());
    let raw_refresh = login.refresh_token.clone();
    assert!(raw_refresh.contains('.'));

    // self
    let self_view = svc.self_by_access(&login.access_token).await.expect("self");
    assert_eq!(self_view.key, view.key);

    // refresh rotation
    let r = svc.refresh(&raw_refresh, "ua", "127.0.0.1").await.expect("refresh");
    assert!(!r.access_token.is_empty());
    assert_ne!(r.refresh_token, raw_refresh);

    // 旧 refresh 已被吊销 — 再用一次返 InvalidToken
    let second = svc.refresh(&raw_refresh, "ua", "127.0.0.1").await;
    assert!(matches!(second, Err(auth::AuthError::InvalidToken)));

    // logout 新 refresh
    svc.logout(&r.refresh_token).await.expect("logout");
    let after = svc.refresh(&r.refresh_token, "ua", "127.0.0.1").await;
    assert!(matches!(after, Err(auth::AuthError::InvalidToken)));
}

#[tokio::test]
#[ignore]
async fn duplicate_username_rejected() {
    let svc = make_service().await;
    let username = unique_user("bob");

    svc.register(&username, "hunter2hunter", None).await.expect("first");
    let dup = svc.register(&username, "hunter2hunter", None).await;
    assert!(matches!(dup, Err(auth::AuthError::UsernameTaken)));
}
