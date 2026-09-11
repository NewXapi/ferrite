//! ops::system_info 集成与鉴权测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p ops --test system_info -- --ignored`

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use ops::system_info::{ProcessTimeTracker, SystemInfoAppState, SystemInfoService, SystemInfoView};
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use uuid::Uuid;

use auth::service::AuthService;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn setup_env() -> (sqlx::PgPool, Arc<AuthService>, Arc<SystemInfoService>) {
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

    let secret = b"system_info_test_jwt_secret_32bytes_long!".to_vec();
    let auth_svc = Arc::new(AuthService::new(pool.clone(), secret).expect("auth svc"));
    let sys_svc = Arc::new(SystemInfoService::new(
        pool.clone(),
        ProcessTimeTracker::default(),
    ));

    (pool, auth_svc, sys_svc)
}

async fn make_user(pool: &sqlx::PgPool, role: i16) -> (Uuid, String) {
    let key = Uuid::new_v4();
    let username = format!("sysinfo_user_{}", key.simple());
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', $4, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(&username)
    .bind("sysinfo test")
    .bind(role)
    .execute(pool)
    .await
    .expect("insert user");
    (key, username)
}

#[tokio::test]
#[ignore]
async fn test_system_info_metrics_flow() {
    let (_pool, _auth_svc, sys_svc) = setup_env().await;

    let info: SystemInfoView = sys_svc
        .get_system_info()
        .await
        .expect("collect system info");

    // 运行时环境验证
    assert_eq!(info.runtime.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(info.runtime.os, std::env::consts::OS);
    assert_eq!(info.runtime.arch, std::env::consts::ARCH);
    assert!(!info.runtime.hostname.is_empty());

    // Uptime 验证
    assert!(info.uptime.started_at <= chrono::Utc::now());

    // 内存数据合理性
    assert!(info.memory.system_total_bytes > 0);
    assert!(info.memory.system_used_bytes <= info.memory.system_total_bytes);

    // CPU 指标
    assert!(info.cpu.num_cpus >= 1);

    // 数据库连接池与实体计数
    assert_eq!(info.database.status, "connected");
    assert!(info.counts.users >= 0);
    assert!(info.counts.channels >= 0);
    assert!(info.counts.active_channels >= 0);
    assert!(info.counts.models >= 0);
    assert!(info.counts.tokens >= 0);
}

#[tokio::test]
#[ignore]
async fn test_system_info_admin_auth_guard() {
    let (pool, auth_svc, sys_svc) = setup_env().await;

    let state = SystemInfoAppState {
        svc: sys_svc,
        auth: auth_svc.clone(),
    };
    let app = ops::system_info_router(state);

    // 1. 无认证请求 -> 401
    let req = Request::builder()
        .uri("/api/system-info")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);

    // 2. 普通用户 (role = 1) -> 403
    let (normal_key, _) = make_user(&pool, 1).await;
    let (normal_token, _) = auth::jwt::issue(
        b"system_info_test_jwt_secret_32bytes_long!",
        &normal_key.to_string(),
        1,
        1,
        &Uuid::new_v4().to_string(),
    )
    .unwrap();

    let req = Request::builder()
        .uri("/api/system-info")
        .method("GET")
        .header("Authorization", format!("Bearer {normal_token}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);

    // 3. 管理员用户 (role = 10) -> 200
    let (admin_key, _) = make_user(&pool, 10).await;
    let (admin_token, _) = auth::jwt::issue(
        b"system_info_test_jwt_secret_32bytes_long!",
        &admin_key.to_string(),
        10,
        1,
        &Uuid::new_v4().to_string(),
    )
    .unwrap();

    let req = Request::builder()
        .uri("/api/system-info")
        .method("GET")
        .header("Authorization", format!("Bearer {admin_token}"))
        .body(Body::empty())
        .unwrap();

    let resp = app.oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let bytes = axum::body::to_bytes(resp.into_body(), 1024 * 64)
        .await
        .unwrap();
    let body_val: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body_val["database"]["status"], "connected");
}
