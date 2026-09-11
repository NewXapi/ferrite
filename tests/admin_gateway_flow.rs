//! E2E: 建渠道 → 建令牌 → /v1/chat/completions → 用量落库。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use contract::error::NormalizedError;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use serde_json::Value;
use tower::ServiceExt;

/// e2e 专用测试口令：仅存在于本地/CI 一次性数据库，命名常量避免裸字面量。
const TEST_PASSWORD: &str = "test_password_123";

struct MockEgress {
    chunks: Vec<Bytes>,
}

impl Egress for MockEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<Box<dyn Future<Output = Result<ForwardedResponse, NormalizedError>> + Send + 'a>> {
        let chunks = self.chunks.clone();
        Box::pin(async move {
            let stream =
                futures_util::stream::iter(chunks.into_iter().map(Ok::<Bytes, std::io::Error>));
            Ok(ForwardedResponse::from_stream(
                200,
                "text/event-stream",
                stream,
            ))
        })
    }
}

fn sse_chunks() -> Vec<Bytes> {
    vec![
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"hello world!\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"!\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n"),
        Bytes::from_static(b"data: [DONE]\n\n"),
    ]
}

/// 统一"读 body + 解析 JSON"样板（仅用于 envelope/JSON 响应；SSE 流走原始字节）。
async fn response_to_json(resp: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// 连不上 PG 时返回 None，调用方跳过测试（CI 无 postgres service）。
/// 需要真链路验证时设 FERRITE_E2E_DATABASE_URL 或起本地 5433 PG。
async fn pg_pool() -> Option<sqlx::PgPool> {
    let url = std::env::var("FERRITE_E2E_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite_e2e".into());
    let pool = sqlx::PgPool::connect_lazy(&url).ok()?;
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&pool)
        .await
    {
        Ok(_) => Some(pool),
        Err(e) => {
            eprintln!("skipping e2e: postgres unreachable at {url}: {e}");
            None
        }
    }
}

/// 三个测试并行跑，各自建 app 会并发执行 CREATE TABLE/TYPE 撞 PG catalog
/// 唯一约束（pg_type_typname_nsp_index）。用全局 mutex 串行化建表段。
static DDL_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 必须先建 app（触发 admin-router 的 ensure_table 建表），再插数据。
async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    let egress = Arc::new(MockEgress {
        chunks: sse_chunks(),
    });
    let _guard = DDL_LOCK.lock().await;
    api::build_app_with_egress(pool.clone(), egress)
        .await
        .expect("build_app_with_egress")
}

/// 插入可登录用户：password_hash 必须是真实 argon2 PHC，否则 login 走
/// password::verify 恒失败、reload 测试拿不到 access JWT。
async fn insert_test_user(pool: &sqlx::PgPool) -> uuid::Uuid {
    let user_key = uuid::Uuid::new_v4();
    let phc =
        auth::password::hash(TEST_PASSWORD).expect("failed to hash test password with argon2");
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, password_hash, role, status)
           VALUES ($1, $2, $3, 1, 1) ON CONFLICT DO NOTHING"#,
    )
    .bind(user_key)
    .bind(format!("user_{}", &user_key.to_string()[..8]))
    .bind(&phc)
    .execute(pool)
    .await
    .unwrap();
    user_key
}

/// 插入渠道；返回 (key_uuid, name)，供用量归因断言比对。
async fn insert_channel(pool: &sqlx::PgPool) -> (uuid::Uuid, String) {
    let key = uuid::Uuid::new_v4();
    let name = format!("ch_{}", &key.to_string()[..8]);
    sqlx::query(
        r#"INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, groups, status)
           VALUES ($1, $2, 'openai', 'http://mock', '["sk"]', '["gpt-4o"]', '{default}', 1)"#,
    )
    .bind(key)
    .bind(&name)
    .execute(pool)
    .await
    .unwrap();
    (key, name)
}

/// 创建 token；返回 (key_uuid, plaintext)
async fn insert_token(pool: &sqlx::PgPool, user_key: uuid::Uuid) -> (uuid::Uuid, String) {
    let key = uuid::Uuid::new_v4();
    let plaintext = format!("sk-{}", uuid::Uuid::new_v4().to_string().replace('-', ""));
    let key_hash = {
        use sha2::{Digest, Sha256};
        let mut h = Sha256::new();
        h.update(&plaintext);
        hex::encode(h.finalize())
    };
    sqlx::query(
        r#"INSERT INTO api_tokens (key, user_key, name, key_hash, key_preview, quota, used_quota, status)
           VALUES ($1, $2, 'tk', $3, $4, 1000000, 0, 1)"#,
    )
    .bind(key).bind(user_key).bind(&key_hash).bind(&plaintext[..8])
    .execute(pool).await.unwrap();
    (key, plaintext)
}

#[tokio::test]
async fn e2e_create_channel_token_call_v1_records_usage() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let _app = build_test_app(&pool).await;

    let user_key = insert_test_user(&pool).await;
    let (channel_key, channel_name) = insert_channel(&pool).await;
    let (_token_key, plaintext) = insert_token(&pool, user_key).await;

    let app = build_test_app(&pool).await;

    let body = serde_json::json!({"model":"gpt-4o","stream":true,"messages":[{"role":"user","content":"hi"}]});
    let req = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", plaintext))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "got {}", resp.status());

    // /v1 响应是 SSE 流（data: 行），不是 JSON —— 只做原文包含断言
    let body_bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&body_bytes).contains("usage"));

    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM usage_logs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count >= 1, "usage_logs rows: {}", count);

    // 渠道归因回归闸：最新一行必须是本请求的，且带着 Dispatch 实际命中的渠道
    // （pipeline 响应 extensions → usage 中间件 → usage_logs）。历史上这两列
    // 恒为 NULL/空（中间件在 pipeline 外拿不到选中路由），渠道维度分析全废。
    let (row_channel_key, row_channel_name): (Option<uuid::Uuid>, String) =
        sqlx::query_as("SELECT channel_key, channel_name FROM usage_logs ORDER BY id DESC LIMIT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        row_channel_key,
        Some(channel_key),
        "usage_logs 最新一行应携带本请求命中渠道的 UUID"
    );
    assert_eq!(
        row_channel_name, channel_name,
        "usage_logs 最新一行应携带本请求命中渠道的名称"
    );

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM api_tokens WHERE key = $1")
        .bind(_token_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(used >= 15, "used_quota: {}", used);
}

#[tokio::test]
async fn e2e_unauthorized_without_token_returns_401() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({"model":"gpt-4o","stream":true,"messages":[]});
    let req = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    assert_eq!(
        ServiceExt::oneshot(app, req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn e2e_admin_api_mounted() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;

    let req = Request::builder()
        .method("GET")
        .uri("/api/token")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        ServiceExt::oneshot(app, req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn e2e_reload_picks_up_new_token_without_restart() {
    let Some(pool) = pg_pool().await else {
        return;
    };

    // 时序关键：app 必须在插 token 之前构建（快照不含新 token），此后全程用
    // 同一实例 —— 401 → 插库 → reload → 200 才真正验证热更，而非重建快照。
    let app = build_test_app(&pool).await;

    let user_key = insert_test_user(&pool).await;
    sqlx::query("UPDATE auth_users SET role = 10 WHERE key = $1")
        .bind(user_key)
        .execute(&pool)
        .await
        .unwrap();

    let login_body = serde_json::json!({
        "username": format!("user_{}", &user_key.to_string()[..8]),
        "password": TEST_PASSWORD
    });
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/user/login")
        // login handler 用 ConnectInfo 提取 client ip；oneshot 裸 Router 不自带，
        // 必须手动塞 extension，否则提取拒绝 → 500 空 body。
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4242".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
        .unwrap();
    // login 走同一 app 实例（时序见上）
    let login_resp = ServiceExt::oneshot(app.clone(), login_req).await.unwrap();
    let login_json = response_to_json(login_resp).await;
    // 登录失败时 access_token 缺失，把响应体打出来定位（用户名/密码/hash 不匹配等）。
    // LoginResult serde rename_all = camelCase → "accessToken"
    let access_token = login_json["accessToken"].as_str().unwrap_or_else(|| {
        panic!("login response missing accessToken: {login_json}");
    });

    let (_token_key, plaintext) = insert_token(&pool, user_key).await;
    // 注意：这里不再重建 app —— 重建会重载快照使 reload 测试失去意义

    let body = serde_json::json!({"model":"gpt-4o","stream":true,"messages":[]});
    let req_before = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", plaintext))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp_before = ServiceExt::oneshot(app.clone(), req_before).await.unwrap();
    assert_eq!(
        resp_before.status(),
        StatusCode::UNAUTHORIZED,
        "reload 前 token 应被拒绝"
    );

    let reload_req = Request::builder()
        .method("POST")
        .uri("/api/gateway/reload")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    let reload_resp = ServiceExt::oneshot(app.clone(), reload_req).await.unwrap();
    assert_eq!(
        reload_resp.status(),
        StatusCode::OK,
        "reload 端点应返回 200"
    );
    let reload_json = response_to_json(reload_resp).await;
    assert_eq!(reload_json["success"], true);
    // 计数字段存在且 >=1：reload 确实从 PG 读到了新插的 token/user/channel
    assert!(
        reload_json["data"]["route_units"].as_u64().unwrap() >= 1,
        "route_units 计数应 >= 1"
    );
    assert!(
        reload_json["data"]["channels"].as_u64().unwrap() >= 1,
        "channels 计数应 >= 1"
    );

    let req_after = Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", plaintext))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();
    let resp_after = ServiceExt::oneshot(app, req_after).await.unwrap();
    assert_eq!(
        resp_after.status(),
        StatusCode::OK,
        "reload 后 token 应被接受"
    );
    let after_bytes = axum::body::to_bytes(resp_after.into_body(), usize::MAX)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&after_bytes).contains("usage"));

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM usage_logs")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(count >= 1, "usage_logs rows: {}", count);
}

/// reload 鉴权负路径：reload 是管理面敏感操作，鉴权必须先于任何查库/热更拒绝。
/// - 无 Authorization 头 → bearer_user 报 InvalidToken → 401（不能 200/501）
/// - role=1 普通用户持有合法 access JWT → admin 守卫 → 403（不能 200）
/// 与核心 reload 测试同文件同 pg_pool skip 模式（app 组装本身需要 PG 迁移）。
#[tokio::test]
async fn e2e_reload_without_admin_bearer_is_rejected() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;

    // 无 Authorization 头 → 401
    let anon_req = Request::builder()
        .method("POST")
        .uri("/api/gateway/reload")
        .body(Body::empty())
        .unwrap();
    let anon_resp = ServiceExt::oneshot(app.clone(), anon_req).await.unwrap();
    assert_eq!(
        anon_resp.status(),
        StatusCode::UNAUTHORIZED,
        "无 token 调 reload 应 401 而非 200"
    );

    // role=1（insert_test_user 默认角色，未做 UPDATE role=10）+ 合法 JWT → 403
    let user_key = insert_test_user(&pool).await;
    let login_body = serde_json::json!({
        "username": format!("user_{}", &user_key.to_string()[..8]),
        "password": TEST_PASSWORD
    });
    let login_req = Request::builder()
        .method("POST")
        .uri("/api/user/login")
        // login handler 用 ConnectInfo 提取 client ip；oneshot 裸 Router 不自带，
        // 必须手动塞 extension（同 e2e_reload_picks_up_new_token_without_restart）
        .extension(axum::extract::ConnectInfo(
            "127.0.0.1:4242".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&login_body).unwrap()))
        .unwrap();
    let login_json =
        response_to_json(ServiceExt::oneshot(app.clone(), login_req).await.unwrap()).await;
    let access_token = login_json["accessToken"].as_str().unwrap_or_else(|| {
        panic!("login response missing accessToken: {login_json}");
    });

    let reload_req = Request::builder()
        .method("POST")
        .uri("/api/gateway/reload")
        .header("Authorization", format!("Bearer {}", access_token))
        .body(Body::empty())
        .unwrap();
    let reload_resp = ServiceExt::oneshot(app, reload_req).await.unwrap();
    assert_eq!(
        reload_resp.status(),
        StatusCode::FORBIDDEN,
        "role=1 用户调 reload 应 403 而非 200"
    );
}

#[tokio::test]
async fn e2e_unmatched_paths_return_404() {
    let Some(pool) = pg_pool().await else {
        return;
    };
    let app = build_test_app(&pool).await;

    let health_req = Request::builder()
        .method("GET")
        .uri("/healthz")
        .body(Body::empty())
        .unwrap();
    let health_resp = ServiceExt::oneshot(app.clone(), health_req).await.unwrap();
    assert_eq!(health_resp.status(), StatusCode::OK, "/healthz 应返回 200");

    let v1beta_req = Request::builder()
        .method("POST")
        .uri("/v1beta/models/gemini-pro:generateContent")
        .header("Content-Type", "application/json")
        .body(Body::empty())
        .unwrap();
    let v1beta_resp = ServiceExt::oneshot(app.clone(), v1beta_req).await.unwrap();
    assert_eq!(
        v1beta_resp.status(),
        StatusCode::UNAUTHORIZED,
        "/v1beta 应进入 pipeline"
    );

    let unmatched_req = Request::builder()
        .method("GET")
        .uri("/api/does-not-exist")
        .body(Body::empty())
        .unwrap();
    let unmatched_resp = ServiceExt::oneshot(app.clone(), unmatched_req)
        .await
        .unwrap();
    assert_eq!(
        unmatched_resp.status(),
        StatusCode::NOT_FOUND,
        "非 /v1/* /v1beta*/healthz 路径应被 guard 404"
    );
    // 这里断言的是纯文本 body（guard 的 404 不走 envelope），保持原始字节比较
    let unmatched_text = axum::body::to_bytes(unmatched_resp.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&unmatched_text), "not found");
}
