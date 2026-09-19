//! 四格式 codec 经注册表组合的集成测试。
//!
//! `tests/format_codec.rs` 用假 codec 验证注册表的调度语义；本文件换成 `with_defaults()`
//! 装配的**真实 codec**，验证三种格式真能互转——这是两跳抽象的核心承诺
//! （N 个 codec 覆盖 N×(N-1) 个方向），也是 WP6 接线前的最后一道保险。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::format_codec::FormatRegistry;
use serde_json::{Value, json};

/// 每个格式都能表达的最小请求体（含一条用户消息）。
fn request_for(fmt: Protocol) -> Bytes {
    let v = match fmt {
        Protocol::OpenAi => json!({
            "model": "m",
            "messages": [{"role": "user", "content": "ping"}]
        }),
        Protocol::Claude => json!({
            "model": "m",
            "max_tokens": 64,
            "messages": [{"role": "user", "content": "ping"}]
        }),
        Protocol::Gemini => json!({
            "contents": [{"role": "user", "parts": [{"text": "ping"}]}]
        }),
        Protocol::OpenAIResp => json!({"model": "m", "input": "ping"}),
        Protocol::Passthrough => json!({"whatever": true}),
    };
    Bytes::from(serde_json::to_vec(&v).unwrap())
}

/// 从 Content 字段里取用户文本。Claude/OpenAI 都接受「字符串或 block 数组」两种形态，
/// 故两种都要认。
fn text_from_content(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    v.as_array().and_then(|a| {
        a.iter()
            .find_map(|b| b.get("text").and_then(Value::as_str))
            .map(str::to_string)
    })
}

/// 每个格式的请求体里，用户文本应出现在哪个位置（用于断言内容真的搬过去了）。
fn user_text_of(fmt: Protocol, v: &Value) -> Option<String> {
    match fmt {
        Protocol::OpenAi => text_from_content(&v["messages"][0]["content"]),
        Protocol::Claude => text_from_content(&v["messages"][0]["content"]),
        Protocol::Gemini => v["contents"][0]["parts"][0]["text"]
            .as_str()
            .map(str::to_string),
        Protocol::OpenAIResp => v["input"]
            .as_array()
            .and_then(|a| a.first())
            .and_then(|i| i["content"].as_array())
            .and_then(|a| a.first())
            .and_then(|p| p["text"].as_str())
            .map(str::to_string),
        Protocol::Passthrough => None,
    }
}

// 全格式两两互转：8 个有序方向（4 格式去掉自转与 Passthrough）都要能走通，
// 且产出的字节确实是目标格式能解析的形状、用户文本原样到达。
//
// 这条断言的意义：旧架构下每多一个格式就要多写 N-1 个转换器，漏一个方向就是
// 运行时 502；新架构下「漏方向」在结构上不可能——只要 codec 注册齐，两跳恒成立。
#[test]
fn all_real_codecs_interoperate_through_registry() {
    let reg = FormatRegistry::with_defaults();
    let formats = [
        Protocol::OpenAi,
        Protocol::Claude,
        Protocol::Gemini,
        Protocol::OpenAIResp,
    ];

    for src in formats {
        for dst in formats {
            if src == dst {
                continue;
            }
            let out = reg
                .translate_request(src, dst, request_for(src))
                .unwrap_or_else(|e| panic!("{src:?} → {dst:?} 应能两跳，实际失败: {e}"));

            let v: Value = serde_json::from_slice(&out)
                .unwrap_or_else(|e| panic!("{src:?} → {dst:?} 产物应是 JSON: {e}"));

            // 产物必须是目标格式的形状：用户文本出现在目标格式约定的位置。
            let text = user_text_of(dst, &v)
                .unwrap_or_else(|| panic!("{src:?} → {dst:?} 的产物里找不到用户文本，实际: {v}"));
            assert_eq!(text, "ping", "{src:?} → {dst:?} 用户文本必须原样到达");
        }
    }
}

// 混合格式的关键路径：Claude 客户端 → OpenAI 上游（本 PR 要修的核心场景之一）。
#[test]
fn claude_client_to_openai_upstream_keeps_tool_definitions() {
    let reg = FormatRegistry::with_defaults();
    let body = Bytes::from(
        serde_json::to_vec(&json!({
            "model": "claude-x",
            "max_tokens": 128,
            "messages": [{"role": "user", "content": "search something"}],
            "tools": [{
                "name": "search",
                "description": "find",
                "input_schema": {"type": "object", "properties": {"q": {"type": "string"}}}
            }]
        }))
        .unwrap(),
    );

    let out = reg
        .translate_request(Protocol::Claude, Protocol::OpenAi, body)
        .expect("Claude → OpenAI 应成功");
    let v: Value = serde_json::from_slice(&out).unwrap();

    // Chat 的工具定义包在 function 子对象里，schema 字段叫 parameters。
    assert_eq!(
        v["tools"][0]["function"]["name"], "search",
        "工具定义必须跨格式存活，实际: {v}"
    );
    assert_eq!(
        v["tools"][0]["function"]["parameters"]["properties"]["q"]["type"], "string",
        "schema 必须搬到 Chat 的 parameters 位置"
    );
    // Claude 必填的 max_tokens 要搬进 Chat 的 max_tokens。
    assert_eq!(v["max_tokens"], 128);
    // system 位置差异：Claude 有则搬成 Chat 的 system 消息（本例没 system，不该凭空多）。
    assert_eq!(v["messages"].as_array().unwrap().len(), 1);
}

// 反方向：OpenAI 客户端 → Claude 上游。Claude 必填 max_tokens，缺了上游会 400，
// 所以两跳必须补上默认值。
#[test]
fn openai_client_to_claude_upstream_fills_required_max_tokens() {
    let reg = FormatRegistry::with_defaults();
    let body = Bytes::from(
        serde_json::to_vec(&json!({
            "model": "gpt-x",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .unwrap(),
    );

    let out = reg
        .translate_request(Protocol::OpenAi, Protocol::Claude, body)
        .expect("OpenAI → Claude 应成功");
    let v: Value = serde_json::from_slice(&out).unwrap();

    assert!(
        v["max_tokens"].as_u64().is_some_and(|n| n > 0),
        "Claude 必填的 max_tokens 必须补默认值，实际: {v}"
    );
    assert_eq!(
        text_from_content(&v["messages"][0]["content"]).as_deref(),
        Some("hi")
    );
}

// Responses 客户端 → Claude 上游（组合路由的最简形态）。
#[test]
fn responses_client_to_claude_upstream_works() {
    let reg = FormatRegistry::with_defaults();
    let body = Bytes::from(
        serde_json::to_vec(&json!({
            "model": "gpt-5",
            "instructions": "be brief",
            "input": "hello"
        }))
        .unwrap(),
    );

    let out = reg
        .translate_request(Protocol::OpenAIResp, Protocol::Claude, body)
        .expect("Responses → Claude 应成功");
    let v: Value = serde_json::from_slice(&out).unwrap();

    // instructions 要落到 Claude 的顶层 system。
    let system_text = v["system"]
        .as_array()
        .and_then(|a| a.first())
        .and_then(|s| s["text"].as_str())
        .or_else(|| v["system"].as_str())
        .unwrap_or_default();
    assert_eq!(
        system_text, "be brief",
        "instructions 必须搬到 Claude 的 system"
    );

    assert_eq!(
        text_from_content(&v["messages"][0]["content"]).as_deref(),
        Some("hello")
    );
    assert!(v["max_tokens"].as_u64().is_some_and(|n| n > 0));
}

// 响应方向的两跳：Claude 上游的响应 → OpenAI 客户端形状。
#[test]
fn claude_response_translates_to_openai_shape() {
    let reg = FormatRegistry::with_defaults();
    let body = Bytes::from(
        serde_json::to_vec(&json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-x",
            "content": [{"type": "text", "text": "pong"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 4, "output_tokens": 2}
        }))
        .unwrap(),
    );

    let out = reg
        .translate_response(Protocol::Claude, Protocol::OpenAi, body)
        .expect("Claude 响应 → OpenAI 应成功");
    let v: Value = serde_json::from_slice(&out).unwrap();

    assert_eq!(v["object"], "chat.completion");
    assert_eq!(v["choices"][0]["message"]["content"], "pong");
    assert_eq!(
        v["choices"][0]["finish_reason"], "stop",
        "end_turn 必须映射成 Chat 的 stop"
    );
    assert_eq!(v["usage"]["prompt_tokens"], 4, "usage 要跨格式搬运");
}

// Passthrough 与任何格式之间都必须零转换直通：喂非法 JSON 也不该报错。
#[test]
fn passthrough_bypasses_conversion_with_real_registry() {
    let reg = FormatRegistry::with_defaults();
    let garbage = Bytes::from_static(b"\x00\xff not json");

    for dst in [
        Protocol::OpenAi,
        Protocol::Claude,
        Protocol::Gemini,
        Protocol::OpenAIResp,
    ] {
        let out = reg
            .translate_request(Protocol::Passthrough, dst, garbage.clone())
            .unwrap_or_else(|e| panic!("Passthrough → {dst:?} 应直通，实际: {e}"));
        assert_eq!(out, garbage);
    }
}
