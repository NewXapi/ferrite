//! E2E: 建渠道 → 建令牌 → /v1/chat/completions → 用量落库。

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::body::Body;
use axum::http::StatusCode;
use bytes::Bytes;
use contract::error::NormalizedError;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use tower::ServiceExt;

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

fn pg_pool() -> sqlx::PgPool {
    let url = std::env::var("FERRITE_E2E_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite_e2e".into());
    sqlx::PgPool::connect_lazy(&url).expect("connect_lazy")
}

/// 必须先建 app（触发 admin-router 的 ensure_table 建表），再插数据。
async fn build_test_app(pool: &sqlx::PgPool) -> axum::Router {
    let egress = Arc::new(MockEgress {
        chunks: sse_chunks(),
    });
    api::build_app_with_egress(pool.clone(), egress)
        .await
        .expect("build_app_with_egress")
}

async fn insert_test_user(pool: &sqlx::PgPool) -> uuid::Uuid {
    let user_key = uuid::Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, password_hash, role, status)
           VALUES ($1, $2, 'hash', 1, 1) ON CONFLICT DO NOTHING"#,
    )
    .bind(user_key)
    .bind(format!("user_{}", &user_key.to_string()[..8]))
    .execute(pool)
    .await
    .unwrap();
    user_key
}

async fn insert_channel(pool: &sqlx::PgPool) {
    sqlx::query(
        r#"INSERT INTO api_channels (key, name, channel_type, base_url, keys, models, group_name, status)
           VALUES ($1, 'ch', 'openai', 'http://mock', '["sk"]', '["gpt-4o"]', 'default', 1)"#,
    )
    .bind(uuid::Uuid::new_v4())
    .execute(pool)
    .await
    .unwrap();
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
    let pool = pg_pool();
    // 建 app → 建表
    let _app = build_test_app(&pool).await;

    // 插数据
    let user_key = insert_test_user(&pool).await;
    insert_channel(&pool).await;
    let (_token_key, plaintext) = insert_token(&pool, user_key).await;

    // 重新建 app（加载新插入的快照）
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({"model":"gpt-4o","stream":true,"messages":[{"role":"user","content":"hi"}]});
    let req = http::Request::builder()
        .method("POST")
        .uri("/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", plaintext))
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap();

    let resp = ServiceExt::oneshot(app, req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "got {}", resp.status());

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

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM api_tokens WHERE key = $1")
        .bind(_token_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(used >= 15, "used_quota: {}", used);
}

#[tokio::test]
async fn e2e_unauthorized_without_token_returns_401() {
    let pool = pg_pool();
    let app = build_test_app(&pool).await;

    let body = serde_json::json!({"model":"gpt-4o","stream":true,"messages":[]});
    let req = http::Request::builder()
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
    let pool = pg_pool();
    let app = build_test_app(&pool).await;

    let req = http::Request::builder()
        .method("GET")
        .uri("/api/token")
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        ServiceExt::oneshot(app, req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
}
