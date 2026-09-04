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
        .register(
            &username,
            "hunter2hunter",
            Some(&format!("{username}@x.com")),
        )
        .await
        .expect("register");
    assert_eq!(view.username, username);
    assert_eq!(view.role, 1);

    // wrong password
    let bad = svc.login(&username, "nope", "ua", "127.0.0.1").await;
    assert!(matches!(bad, Err(auth::AuthError::InvalidCredentials)));

    // good password → token + refresh
    let login = svc
        .login(&username, "hunter2hunter", "ua", "127.0.0.1")
        .await
        .expect("login");
    assert_eq!(login.user.username, username);
    assert!(!login.access_token.is_empty());
    let raw_refresh = login.refresh_token.clone();
    assert!(raw_refresh.contains('.'));

    // self
    let self_view = svc.self_by_access(&login.access_token).await.expect("self");
    assert_eq!(self_view.key, view.key);

    // refresh rotation
    let r = svc
        .refresh(&raw_refresh, "ua", "127.0.0.1")
        .await
        .expect("refresh");
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

    svc.register(&username, "hunter2hunter", None)
        .await
        .expect("first");
    let dup = svc.register(&username, "hunter2hunter", None).await;
    assert!(matches!(dup, Err(auth::AuthError::UsernameTaken)));
}

#[tokio::test]
#[ignore]
async fn login_nonexistent_user_rejected() {
    // 防枚举路径: 用户不存在也要走 dummy argon2 verify, 不能 panic/500
    let svc = make_service().await;
    let bad = svc
        .login("no_such_user_zz", "whatever123", "ua", "127.0.0.1")
        .await;
    assert!(matches!(bad, Err(auth::AuthError::InvalidCredentials)));
}

#[tokio::test]
#[ignore]
async fn auth_version_bump_invalidates_refresh() {
    // 改密 (auth_version++) 后, 已发的 refresh 必须失效
    let svc = make_service().await;
    let username = unique_user("carol");
    svc.register(&username, "hunter2hunter", None)
        .await
        .expect("register");
    let login = svc
        .login(&username, "hunter2hunter", "ua", "ip")
        .await
        .expect("login");

    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pool2");
    sqlx::query("UPDATE auth_users SET auth_version = auth_version + 1 WHERE username = $1")
        .bind(&username)
        .execute(&pool)
        .await
        .expect("bump");

    // 旧 access JWT: auth_version 不匹配 → InvalidToken
    let self_after = svc.self_by_access(&login.access_token).await;
    assert!(matches!(self_after, Err(auth::AuthError::InvalidToken)));

    // 旧 refresh: auth_version 不匹配 → InvalidToken
    let r = svc.refresh(&login.refresh_token, "ua", "ip").await;
    assert!(matches!(r, Err(auth::AuthError::InvalidToken)));
}

#[tokio::test]
#[ignore]
async fn duplicate_email_rejected() {
    let svc = make_service().await;
    let email = format!("{}@x.com", unique_user("shared"));
    let u1 = unique_user("dave");
    let u2 = unique_user("eve");

    svc.register(&u1, "hunter2hunter", Some(&email))
        .await
        .expect("u1");
    let dup = svc.register(&u2, "hunter2hunter", Some(&email)).await;
    assert!(matches!(dup, Err(auth::AuthError::EmailTaken)));
}

#[tokio::test]
#[ignore]
async fn malformed_refresh_tokens_rejected() {
    let svc = make_service().await;
    for bad in [
        "",
        "no-dot",
        "not-a-uuid.00",
        "00000000-0000-0000-0000-000000000000.zzz",
        "00000000-0000-0000-0000-000000000000.00",
    ] {
        let r = svc.refresh(bad, "ua", "ip").await;
        assert!(
            matches!(r, Err(auth::AuthError::InvalidToken)),
            "expected InvalidToken for {bad:?}"
        );
    }
}

#[tokio::test]
#[ignore]
async fn weak_credentials_rejected() {
    let svc = make_service().await;
    let u = unique_user("frank");
    assert!(matches!(
        svc.register(&u, "short", None).await,
        Err(auth::AuthError::BadRequest(_))
    ));
    assert!(matches!(
        svc.register("", "hunter2hunter", None).await,
        Err(auth::AuthError::BadRequest(_))
    ));
}

#[tokio::test]
#[ignore]
async fn disabled_user_cannot_login_or_refresh() {
    let svc = make_service().await;
    let username = unique_user("gina");
    svc.register(&username, "hunter2hunter", None)
        .await
        .expect("register");
    let login = svc
        .login(&username, "hunter2hunter", "ua", "ip")
        .await
        .expect("login");

    let pool = sqlx::PgPool::connect(&db_url()).await.expect("pool2");
    sqlx::query("UPDATE auth_users SET status = 2 WHERE username = $1")
        .bind(&username)
        .execute(&pool)
        .await
        .expect("disable");

    let bad_login = svc.login(&username, "hunter2hunter", "ua", "ip").await;
    assert!(matches!(bad_login, Err(auth::AuthError::UserDisabled)));

    let bad_refresh = svc.refresh(&login.refresh_token, "ua", "ip").await;
    assert!(matches!(bad_refresh, Err(auth::AuthError::UserDisabled)));
}

#[tokio::test]
#[ignore]
async fn update_self_password_bumps_version() {
    let svc = make_service().await;
    let username = unique_user("hank");
    svc.register(&username, "hunter2hunter", None)
        .await
        .expect("register");
    let login = svc
        .login(&username, "hunter2hunter", "ua", "ip")
        .await
        .expect("login");
    let key = uuid::Uuid::parse_str(&login.user.key).unwrap();

    // 错的原密码
    let bad = svc
        .update_self(key, None, Some("wrong-pass"), Some("newpassword1"))
        .await;
    assert!(matches!(bad, Err(auth::AuthError::InvalidCredentials)));

    // 对的原密码 → 改密成功
    svc.update_self(
        key,
        Some("Hank"),
        Some("hunter2hunter"),
        Some("newpassword1"),
    )
    .await
    .expect("update");

    // 旧 access 失效
    assert!(matches!(
        svc.self_by_access(&login.access_token).await,
        Err(auth::AuthError::InvalidToken)
    ));
    // 旧 refresh 失效
    assert!(matches!(
        svc.refresh(&login.refresh_token, "ua", "ip").await,
        Err(auth::AuthError::InvalidToken)
    ));
    // 新密码可登录, display_name 生效
    let relogin = svc
        .login(&username, "newpassword1", "ua", "ip")
        .await
        .expect("relogin");
    assert_eq!(relogin.user.display_name, "Hank");
}

#[tokio::test]
#[ignore]
async fn admin_user_management_flow() {
    let svc = make_service().await;
    let admin_name = unique_user("root");
    let user_name = unique_user("mallory");

    let admin = svc
        .register(&admin_name, "hunter2hunter", None)
        .await
        .unwrap();
    let user = svc
        .register(&user_name, "hunter2hunter", None)
        .await
        .unwrap();
    let admin_key = uuid::Uuid::parse_str(&admin.key).unwrap();
    let user_key = uuid::Uuid::parse_str(&user.key).unwrap();

    // 提权 admin → role 100
    let promoted = svc
        .manage_user(admin_key, "set_role", Some("100"))
        .await
        .unwrap();
    assert_eq!(promoted.role, 100);

    // (admin 调 list 需要走路由层鉴权; service 层直接测 list + manage)
    let (users, total) = svc.list_users(Some(&user_name), 1, 20).await.unwrap();
    assert!(total >= 1);
    assert!(users.iter().any(|u| u.key == user.key));

    // 禁用
    let disabled = svc.manage_user(user_key, "disable", None).await.unwrap();
    assert_eq!(disabled.status, 2);
    let disabled_login = svc.login(&user_name, "hunter2hunter", "ua", "ip").await;
    assert!(matches!(disabled_login, Err(auth::AuthError::UserDisabled)));

    // 调额度
    let charged = svc
        .manage_user(user_key, "adjust_quota", Some("500000"))
        .await
        .unwrap();
    assert_eq!(charged.quota, 500000);

    // admin 重置密码 → 旧 refresh 全失效
    let user_login = {
        svc.manage_user(user_key, "enable", None).await.unwrap();
        svc.login(&user_name, "hunter2hunter", "ua", "ip")
            .await
            .unwrap()
    };
    svc.manage_user(user_key, "reset_password", Some("brandnew99"))
        .await
        .unwrap();
    assert!(matches!(
        svc.refresh(&user_login.refresh_token, "ua", "ip").await,
        Err(auth::AuthError::InvalidToken)
    ));
    let after_reset = svc
        .login(&user_name, "brandnew99", "ua", "ip")
        .await
        .unwrap();
    assert!(after_reset.user.auth_version > user.auth_version);

    // 删除
    svc.delete_user(user_key).await.unwrap();
    assert!(matches!(
        svc.login(&user_name, "brandnew99", "ua", "ip").await,
        Err(auth::AuthError::InvalidCredentials)
    ));
}
