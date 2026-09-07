//! R3 后端校验：含 `_ferrite_agent_prompt_marker` 的 payload 必须已物化。
//!
//! 测试策略：直接调用 `router` 并用 `tower::ServiceExt::oneshot` 发请求，
//! 上游指向不可达端口即可（marker 校验发生在上游调用之前）。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use bytes::Bytes;
use tavern_generate::{GenerateConfig, GenerateState, router};

fn make_state(upstream: String) -> GenerateState {
    GenerateState {
        dirs: tavern_storage::DataRoot::new("/tmp/nonexistent").user("test"),
        config: GenerateConfig { upstream },
        http: reqwest::Client::new(),
    }
}

/// 含 marker + 消息 content 残留 `{{char}}` → 400，body 含 "unfinalized" 相关错误文本。
#[tokio::test]
async fn marker_with_unmaterialized_content_rejected() {
    let router = router(make_state("http://127.0.0.1:1".into()));

    let payload = serde_json::json!({
        "_ferrite_agent_prompt_marker": "",
        "messages": [
            { "role": "user", "content": "Hello {{char}}!" }
        ]
    });

    let resp = tower::ServiceExt::oneshot(
        router,
        Request::builder()
            .method("POST")
            .uri("/generate")
            .header("content-type", "application/json")
            .body(Body::from(Bytes::from(payload.to_string())))
            .unwrap(),
    )
    .await
    .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        body_str.contains("unfinalized") || body_str.contains("{{"),
        "body should mention unfinalized: {body_str}"
    );
}

/// 含 marker（空串）+ 干净消息 → 不是 400（会因 secret 缺失或上游不可达失败，
/// 但断言 status != 400 且 body 不含 unfinalized）。
#[tokio::test]
async fn marker_clean_content_not_rejected_by_validation() {
    let router = router(make_state("http://127.0.0.1:1".into()));

    let payload = serde_json::json!({
        "_ferrite_agent_prompt_marker": "",
        "messages": [
            { "role": "user", "content": "Hello there!" }
        ]
    });

    let resp = tower::ServiceExt::oneshot(
        router,
        Request::builder()
            .method("POST")
            .uri("/generate")
            .header("content-type", "application/json")
            .body(Body::from(Bytes::from(payload.to_string())))
            .unwrap(),
    )
    .await
    .unwrap();

    // 不是 400：marker 校验通过，失败来自上游不可达 / secret 缺失
    assert_ne!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "clean payload should not trigger marker rejection"
    );

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        !body_str.contains("unfinalized"),
        "body should not mention unfinalized: {body_str}"
    );
}

/// 无 marker + `{{char}}` 残留 → 不因校验被拒（断言不是该校验的 400）。
#[tokio::test]
async fn no_marker_with_unmaterialized_content_not_rejected() {
    let router = router(make_state("http://127.0.0.1:1".into()));

    let payload = serde_json::json!({
        "messages": [
            { "role": "user", "content": "Hello {{char}}!" }
        ]
    });

    let resp = tower::ServiceExt::oneshot(
        router,
        Request::builder()
            .method("POST")
            .uri("/generate")
            .header("content-type", "application/json")
            .body(Body::from(Bytes::from(payload.to_string())))
            .unwrap(),
    )
    .await
    .unwrap();

    // 不是 marker 校验的 400：无 marker 字段 → 校验跳过
    // 失败来自上游不可达 / secret 缺失
    assert_ne!(
        resp.status(),
        StatusCode::BAD_REQUEST,
        "payload without marker should not trigger marker rejection"
    );

    let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert!(
        !body_str.contains("unfinalized"),
        "body should not mention unfinalized: {body_str}"
    );
}
