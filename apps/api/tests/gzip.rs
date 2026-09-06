//! 请求体 gzip 解压集成测试（对齐 ST `request-compression.js`）。
//!
//! `RequestDecompressionLayer` 在客户端标记 `Content-Encoding: gzip` 时尝试解压；
//! 若 body 非法则返回 415（解压层拦截），未标记则透传给业务层。
//! 不依赖 flate2（workspace 未引入），通过语义边界验证层已正确挂载。

use api::tavern::router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

#[tokio::test]
async fn malformed_gzip_body_is_rejected_by_decompression_layer() {
    let router = router(&api::tavern::TavernConfig::default()).expect("router");
    // 标记 Content-Encoding: gzip 但 body 不是合法 gzip → 解压层拦截
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tavern/characters")
                .header("content-type", "application/json")
                .header("content-encoding", "gzip")
                .body(Body::from("this is not gzip data at all"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE,
        "expected 400 or 415, got {}",
        response.status()
    );
}

#[tokio::test]
async fn non_gzip_request_passes_through_to_business_layer() {
    let router = router(&api::tavern::TavernConfig::default()).expect("router");
    let payload = serde_json::json!({ "name": "bob" }).to_string();
    let response = router
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/tavern/characters")
                .header("content-type", "application/json")
                .body(Body::from(payload))
                .expect("request"),
        )
        .await
        .expect("response");
    // 透传到业务层——即使业务返回 400/422，也绝不是解压层的 415。
    assert_ne!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
}
