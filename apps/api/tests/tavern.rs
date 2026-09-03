//! `/tavern/*` 路由冒烟：角色卡与聊天走完整 HTTP 往返。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

fn app(data_root: &str) -> axum::Router {
    api::tavern::router(&api::tavern::TavernConfig {
        data_root: data_root.to_string(),
        upstream: "http://127.0.0.1:1".into(),
    })
    .expect("tavern router")
}

async fn call(
    router: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<&str>,
) -> (StatusCode, String) {
    let req = Request::builder().method(method).uri(uri);
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn tmp_root(tag: &str) -> String {
    let p = std::env::temp_dir().join(format!("ferrite-tavern-{}-{tag}", std::process::id()));
    std::fs::remove_dir_all(&p).ok();
    p.to_string_lossy().to_string()
}

#[tokio::test]
async fn character_create_list_get_delete() {
    let root = tmp_root("chars");
    let app = app(&root);

    let (status, _) = call(
        &app,
        "POST",
        "/tavern/characters",
        Some(r#"{"file_name":"alice","name":"Alice","description":"a test card"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);

    let (status, body) = call(&app, "GET", "/tavern/characters", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("Alice"), "list body: {body}");

    let (status, body) = call(&app, "GET", "/tavern/characters/alice", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("a test card"));

    let (status, _) = call(&app, "DELETE", "/tavern/characters/alice", None).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = call(&app, "GET", "/tavern/characters", None).await;
    assert_eq!(body, "[]");

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn chat_save_reload_and_recent() {
    let root = tmp_root("chats");
    let app = app(&root);

    let messages = r#"[
        {"name":"User","is_user":true,"send_date":"2026-09-03","mes":"hi"},
        {"name":"Alice","is_user":false,"send_date":"2026-09-03","mes":"hello"}
    ]"#;
    let (status, _) = call(&app, "PUT", "/tavern/chats/alice/c1", Some(messages)).await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = call(&app, "GET", "/tavern/chats/alice/c1", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("hello"), "reloaded: {body}");

    let (status, body) = call(&app, "GET", "/tavern/chats/alice", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("\"file_name\":\"c1\""), "recent: {body}");
    assert!(body.contains("hi"), "preview missing: {body}");

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn settings_roundtrip_keeps_unknown_fields() {
    let root = tmp_root("settings");
    let app = app(&root);

    let (status, body) = call(&app, "GET", "/tavern/settings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "{}");

    let (status, _) = call(
        &app,
        "PUT",
        "/tavern/settings",
        Some(r#"{"temperature":0.7,"future_field":{"a":1}}"#),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (_, body) = call(&app, "GET", "/tavern/settings", None).await;
    assert!(body.contains("future_field"), "dropped unknown field: {body}");

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn secrets_never_return_plaintext() {
    let root = tmp_root("secrets");
    let app = app(&root);

    let (status, _) = call(
        &app,
        "PUT",
        "/tavern/secrets/api_key_openai",
        Some(r#"{"value":"sk-do-not-leak"}"#),
    )
    .await;
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (status, body) = call(&app, "GET", "/tavern/secrets", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("api_key_openai"));
    assert!(!body.contains("sk-do-not-leak"), "leaked secret: {body}");

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn path_traversal_is_rejected() {
    let root = tmp_root("traversal");
    let app = app(&root);

    let (status, _) = call(&app, "GET", "/tavern/chats/..%2f..%2fetc/passwd", None).await;
    assert_ne!(status, StatusCode::OK);

    std::fs::remove_dir_all(&root).ok();
}
