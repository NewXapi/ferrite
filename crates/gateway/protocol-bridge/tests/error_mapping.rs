//! 各协议错误形状的行为测试。
//!
//! 核心断言：**四种入站协议必须产出四种不同的错误体**。旧实现把
//! `ProtocolKind::OpenAIResp` 与 `OpenAI` 归并到同一分支，Responses 客户端会拿到
//! Chat 形状的错误——本文件的 `responses_error_shape_differs_from_chat` 专门盯这条。

use axum::body::Body;
use gateway_pipeline::ctx::ProtocolKind;
use gateway_pipeline::error::StageError;
use gateway_protocol_bridge::error_mapping::map_error;

/// 读干响应体。
async fn body_json(resp: http::Response<Body>) -> serde_json::Value {
    let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body");
    serde_json::from_slice(&bytes).expect("错误体应是 JSON")
}

fn unauthorized() -> StageError {
    StageError::Unauthenticated("bad token".into())
}

// 四种协议各自的错误体形状：每家 SDK 读的字段名都不一样，混用会让客户端
// 把错误当成功解析（或在解析阶段就崩）。
#[tokio::test]
async fn each_protocol_has_its_own_error_shape() {
    // Chat：error.{code,message,type}
    let resp = map_error(unauthorized(), ProtocolKind::OpenAI);
    assert_eq!(resp.status(), 401);
    let v = body_json(resp).await;
    assert_eq!(v["error"]["message"], "bad token");
    assert!(
        v["error"]["type"].as_str().is_some_and(|t| !t.is_empty()),
        "Chat 错误必须有 type 分类，实际: {v}"
    );

    // Responses：同样是 error.{code,message,type}，但 code 可空。
    let resp = map_error(unauthorized(), ProtocolKind::OpenAIResp);
    assert_eq!(resp.status(), 401);
    let v = body_json(resp).await;
    assert_eq!(v["error"]["message"], "bad token");
    assert!(
        v["error"].get("type").is_some(),
        "Responses 错误必须有 type，实际: {v}"
    );

    // Anthropic：顶层 type=error，内层只有 type+message（没有 code）。
    let resp = map_error(unauthorized(), ProtocolKind::Anthropic);
    assert_eq!(resp.status(), 401);
    let v = body_json(resp).await;
    assert_eq!(v["type"], "error");
    assert_eq!(v["error"]["message"], "bad token");
    assert_eq!(
        v["error"]["type"], "authentication_error",
        "Anthropic 的错误分类名与 OpenAI 不同，实际: {v}"
    );

    // Gemini：error.{code,message,status}，status 是 Google 的枚举名。
    let resp = map_error(unauthorized(), ProtocolKind::Gemini);
    assert_eq!(resp.status(), 401);
    let v = body_json(resp).await;
    assert_eq!(v["error"]["status"], "UNAUTHENTICATED");
    assert_eq!(v["error"]["message"], "bad token");
}

// **WP7 核心回归**：Responses 的错误体不得与 Chat 完全相同。
// 旧实现 `ProtocolKind::OpenAI | ProtocolKind::OpenAIResp` 共用一个分支，
// 两者字节级一致——Responses SDK 拿到的分类语义是错的。
#[tokio::test]
async fn responses_error_shape_differs_from_chat() {
    let chat = body_json(map_error(unauthorized(), ProtocolKind::OpenAI)).await;
    let resp = body_json(map_error(unauthorized(), ProtocolKind::OpenAIResp)).await;

    assert_ne!(
        chat, resp,
        "Responses 错误体必须与 Chat 区分开（旧实现两者归并），实际都等于: {chat}"
    );
    assert_eq!(
        resp["error"]["code"],
        serde_json::Value::Null,
        "Responses 的 error.code 应为空（它表达的是 SDK 侧 code 缺省），实际: {resp}"
    );
}

// 状态码映射：StageError → HTTP 状态码，四种协议必须一致（形状不同但码同源）。
#[tokio::test]
async fn status_codes_are_shared_across_protocols() {
    let cases: [(StageError, u16); 4] = [
        (StageError::Unauthenticated("x".into()), 401),
        (StageError::Forbidden("x".into()), 403),
        (StageError::NoRoute, 404),
        (StageError::RateLimited, 429),
    ];

    for (err, expected) in cases {
        for target in [
            ProtocolKind::OpenAI,
            ProtocolKind::OpenAIResp,
            ProtocolKind::Anthropic,
            ProtocolKind::Gemini,
        ] {
            let resp = map_error(err.clone(), target);
            assert_eq!(
                resp.status().as_u16(),
                expected,
                "{target:?} 的状态码必须与错误语义一致"
            );
        }
    }
}

// 错误体必须是合法 JSON 且带 application/json 内容类型：
// 客户端 SDK 按内容类型决定要不要解析体。
#[tokio::test]
async fn error_response_is_json_content_type() {
    let resp = map_error(StageError::RateLimited, ProtocolKind::OpenAI);
    assert_eq!(
        resp.headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/json")
    );
}
