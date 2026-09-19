//! 各协议错误形状的行为测试。
//!
//! 核心断言：**四种入站协议必须产出四种不同的错误体**。旧实现把
//! `ProtocolKind::OpenAIResp` 与 `OpenAI` 归并到同一分支，Responses 客户端会拿到
//! Chat 形状的错误——本文件的 `responses_error_shape_differs_from_chat` 专门盯这条。
//!
//! 全部用例同步：形状函数（`to_*_shape`）是纯函数直接返回 JSON；`map_error` 只断言
//! 状态码与内容类型，不读 body（读 body 需 async runtime，本 crate 无 tokio 依赖）。

use contract::error::NormalizedError;
use gateway_pipeline::ctx::ProtocolKind;
use gateway_pipeline::error::StageError;
use gateway_protocol_bridge::error_mapping::{
    map_error, to_anthropic_shape, to_gemini_shape, to_openai_shape, to_responses_shape,
};

fn sample() -> NormalizedError {
    NormalizedError {
        code: "unauthenticated",
        message: "bad token".into(),
        status: 401,
        retryable: false,
        channel_scoped: false,
    }
}

// 四种协议各自的错误体形状：每家 SDK 读的字段名都不一样，混用会让客户端
// 把错误当成功解析（或在解析阶段就崩）。
#[test]
fn each_protocol_has_its_own_error_shape() {
    // Chat：error.{code,message,type}
    let chat = to_openai_shape(&sample());
    assert_eq!(chat["error"]["message"], "bad token");
    assert_eq!(
        chat["error"]["code"], "unauthenticated",
        "Chat 的 code 是内部错误码，实际: {chat}"
    );
    assert!(
        chat["error"]["type"]
            .as_str()
            .is_some_and(|t| !t.is_empty()),
        "Chat 错误必须有 type 分类，实际: {chat}"
    );

    // Responses：同为 error.{code,message,type}，但 code 是 SDK 侧缺省（null）。
    let resp = to_responses_shape(&sample());
    assert_eq!(resp["error"]["message"], "bad token");
    assert_eq!(
        resp["error"]["code"],
        serde_json::Value::Null,
        "Responses 的 code 应为空（SDK 侧缺省），实际: {resp}"
    );

    // Anthropic：顶层 type=error，内层只有 type+message（没有 code）。
    let anth = to_anthropic_shape(&sample());
    assert_eq!(anth["type"], "error");
    assert_eq!(anth["error"]["message"], "bad token");
    assert_eq!(
        anth["error"]["type"], "authentication_error",
        "Anthropic 的分类名与 OpenAI 不同，实际: {anth}"
    );
    assert!(
        anth["error"].get("code").is_none(),
        "Anthropic 错误体不该有 code 字段，实际: {anth}"
    );

    // Gemini：error.{code,message,status}，status 是 Google 的枚举名。
    let gem = to_gemini_shape(&sample());
    assert_eq!(gem["error"]["status"], "UNAUTHENTICATED");
    assert_eq!(gem["error"]["message"], "bad token");
}

// **WP7 核心回归**：Responses 的错误体不得与 Chat 完全相同。
// 旧实现 `ProtocolKind::OpenAI | ProtocolKind::OpenAIResp` 共用一个分支，
// 两者字节级一致——Responses SDK 拿到的分类语义是错的。
#[test]
fn responses_error_shape_differs_from_chat() {
    let chat = to_openai_shape(&sample());
    let resp = to_responses_shape(&sample());

    assert_ne!(
        chat, resp,
        "Responses 错误体必须与 Chat 区分开（旧实现两者归并），实际都等于: {chat}"
    );
}

// 错误分类按 code 词根派生：auth/quota/rate 三类各自映射，未匹配落兜底。
#[test]
fn error_type_classification_follows_code() {
    let mk = |code: &'static str| NormalizedError {
        code,
        message: "m".into(),
        status: 400,
        retryable: false,
        channel_scoped: false,
    };

    assert_eq!(
        to_openai_shape(&mk("rate_limited"))["error"]["type"],
        "rate_limit_error"
    );
    assert_eq!(
        to_openai_shape(&mk("quota_exhausted"))["error"]["type"],
        "insufficient_quota"
    );
    assert_eq!(
        to_openai_shape(&mk("unauthenticated"))["error"]["type"],
        "invalid_api_key"
    );
    assert_eq!(
        to_openai_shape(&mk("something_else"))["error"]["type"],
        "api_error"
    );

    // Anthropic 用另一套命名。
    assert_eq!(
        to_anthropic_shape(&mk("rate_limited"))["error"]["type"],
        "rate_limit_error"
    );
    assert_eq!(
        to_anthropic_shape(&mk("quota_exhausted"))["error"]["type"],
        "billing_error"
    );
}

// 状态码映射：四种协议必须一致（形状不同但码同源）。
#[test]
fn status_codes_are_shared_across_protocols() {
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
            // StageError 不是 Clone：按用例逐个重建等价错误。
            let fresh = match &err {
                StageError::Unauthenticated(m) => StageError::Unauthenticated(m.clone()),
                StageError::Forbidden(m) => StageError::Forbidden(m.clone()),
                StageError::NoRoute => StageError::NoRoute,
                StageError::RateLimited => StageError::RateLimited,
                other => panic!("用例只覆盖上面四类，实际: {other}"),
            };
            let resp = map_error(fresh, target);
            assert_eq!(
                resp.status().as_u16(),
                expected,
                "{target:?} 的状态码必须与错误语义一致"
            );
        }
    }
}

// Gemini 的状态名映射表：Google 的 SDK 按这个枚举名分类。
#[test]
fn gemini_status_names_follow_http_status() {
    let mk = |status: u16| NormalizedError {
        code: "c",
        message: "m".into(),
        status,
        retryable: false,
        channel_scoped: false,
    };

    assert_eq!(
        to_gemini_shape(&mk(400))["error"]["status"],
        "INVALID_ARGUMENT"
    );
    assert_eq!(
        to_gemini_shape(&mk(403))["error"]["status"],
        "PERMISSION_DENIED"
    );
    assert_eq!(to_gemini_shape(&mk(404))["error"]["status"], "NOT_FOUND");
    assert_eq!(
        to_gemini_shape(&mk(429))["error"]["status"],
        "RESOURCE_EXHAUSTED"
    );
    assert_eq!(to_gemini_shape(&mk(503))["error"]["status"], "INTERNAL");
}

// 错误响应必须带 application/json 内容类型：客户端 SDK 按它决定要不要解析体。
#[test]
fn error_response_is_json_content_type() {
    let resp = map_error(StageError::RateLimited, ProtocolKind::OpenAI);
    assert_eq!(
        resp.headers()
            .get(http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("application/json")
    );
}
