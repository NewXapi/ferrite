//! Gemini codec 的行为测试。
//!
//! 除双向 round-trip 外，重点盯旧实现（`adaptor::convert_gemini_event_to_openai`）
//! 的三个具体缺陷：finish_reason 原样透传、chunk id 每块现生成 uuid、usage 帧的
//! `choices` 是空数组。三个用例都写成「旧行为必然失败」的形式。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::format_codec::FormatCodec;
use gateway_protocol_bridge::gemini::GeminiCodec;
use gateway_protocol_bridge::ir::{ContentBlock, Role, StopReason, StreamEvent};
use serde_json::{Value, json};

/// 把编码器输出拆成 JSON 序列。
fn frames_to_values(frames: &[Bytes]) -> Vec<Value> {
    frames
        .iter()
        .filter_map(|f| {
            let text = String::from_utf8_lossy(f);
            let line = text.lines().find(|l| l.starts_with("data: "))?;
            serde_json::from_str(line.strip_prefix("data: ")?).ok()
        })
        .collect()
}

// ---------- 请求方向 ----------

// 请求双向 round-trip：contents / systemInstruction / generationConfig / tools 不丢。
#[test]
fn gemini_request_roundtrip_preserves_structure() {
    let codec = GeminiCodec::new();
    let body = serde_json::to_vec(&json!({
        "systemInstruction": {"parts": [{"text": "be brief"}]},
        "contents": [
            {"role": "user", "parts": [{"text": "hi"}]},
            {"role": "model", "parts": [{"text": "hello"}]}
        ],
        "generationConfig": {
            "temperature": 0.4,
            "topP": 0.9,
            "maxOutputTokens": 128,
            "stopSequences": ["###"]
        },
        "tools": [{"functionDeclarations": [{
            "name": "search",
            "description": "find things",
            "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
        }]}]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.system.len(), 1);
    assert_eq!(ir.system[0].text, "be brief");
    assert_eq!(ir.messages.len(), 2);
    assert_eq!(ir.messages[0].role, Role::User);
    assert_eq!(
        ir.messages[1].role,
        Role::Assistant,
        "model 应映射为 assistant"
    );

    assert_eq!(ir.tools.len(), 1, "functionDeclarations 必须摊平成 tools");
    assert_eq!(ir.tools[0].name, "search");
    assert_eq!(
        ir.tools[0].input_schema["properties"]["q"]["type"], "string",
        "Gemini 的 parameters 必须进 input_schema"
    );

    let s = ir
        .sampling
        .as_ref()
        .expect("generationConfig 应产生采样参数");
    assert_eq!(s.temperature, Some(0.4));
    assert_eq!(s.top_p, Some(0.9));
    assert_eq!(s.max_tokens, Some(128));
    assert_eq!(s.extra.get("stop"), Some(&json!(["###"])));

    // 回写：字段名要回到 Gemini 的命名。
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(out["systemInstruction"]["parts"][0]["text"], "be brief");
    assert_eq!(out["contents"][0]["role"], "user");
    assert_eq!(out["contents"][1]["role"], "model");
    assert_eq!(out["generationConfig"]["maxOutputTokens"], 128);
    assert_eq!(out["generationConfig"]["topP"], 0.9);
    assert_eq!(out["generationConfig"]["stopSequences"][0], "###");
    assert_eq!(
        out["tools"][0]["functionDeclarations"][0]["parameters"]["type"], "object",
        "回写必须用 Gemini 的 parameters 字段名"
    );
}

// functionCall 与 inline_data 都要能解出结构化 block，不能退化成纯文本。
#[test]
fn gemini_request_decodes_function_call_and_image_parts() {
    let codec = GeminiCodec::new();
    let body = serde_json::to_vec(&json!({
        "contents": [{"role": "model", "parts": [
            {"functionCall": {"name": "search", "args": {"q": "x"}}},
            {"inline_data": {"mime_type": "image/png", "data": "AAAB"}}
        ]}]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    let content = &ir.messages[0].content;

    assert!(
        content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { name, input, .. }
                if name == "search" && input["q"] == "x")),
        "functionCall 应成为 ToolUse，实际: {content:?}"
    );
    assert!(
        content
            .iter()
            .any(|b| matches!(b, ContentBlock::Image { media_type, data }
                if media_type == "image/png" && data == "AAAB")),
        "inline_data 应成为 Image，实际: {content:?}"
    );
}

// ---------- 缺陷一：finish_reason 映射 ----------

// **硬反例**：Gemini 的 finishReason 是大写枚举，必须映射成 IR 的语义值，
// 不能原样透传（旧实现直接赋值，客户端拿到 "STOP" 这种 OpenAI 形状里不存在的取值）。
#[test]
fn gemini_finish_reason_is_mapped_not_passed_through() {
    let codec = GeminiCodec::new();

    let cases = [
        ("STOP", StopReason::Stop),
        ("MAX_TOKENS", StopReason::Length),
        ("SAFETY", StopReason::ContentFilter),
        ("RECITATION", StopReason::ContentFilter),
        ("MALFORMED_FUNCTION_CALL", StopReason::ToolUse),
    ];

    for (raw, expected) in cases {
        let body = serde_json::to_vec(&json!({
            "candidates": [{
                "content": {"role": "model", "parts": [{"text": "x"}]},
                "finishReason": raw
            }],
            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
        }))
        .unwrap();

        let resp = codec.decode_response(Bytes::from(body)).expect("decode");
        assert_eq!(
            resp.stop_reason, expected,
            "{raw} 必须映射成 {expected:?}，不能原样透传"
        );
        assert_ne!(
            resp.stop_reason,
            StopReason::Other(raw.to_string()),
            "{raw} 不得落进 Other 逃逸口（说明映射表漏了这一项）"
        );
    }
}

// 流式路径同样要映射：旧实现的流式分支也把 finish_reason 原样塞出去。
#[test]
fn gemini_stream_finish_reason_is_mapped() {
    let codec = GeminiCodec::new();
    let frame = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [{"text": "done"}]},
            "finishReason": "MAX_TOKENS"
        }]
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");
    let stop = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::MessageStop { reason } => Some(reason.clone()),
            _ => None,
        })
        .expect("应产出 MessageStop");

    assert_eq!(stop, StopReason::Length, "流式 finishReason 也必须映射");
}

// ---------- 缺陷二：id 稳定 ----------

// **硬反例**：chunk id 全流复用。旧实现每块 `Uuid::new_v4()`，同一次补全会被
// 客户端与流式计费聚合当成多次响应。这里断言所有帧的 id 全等。
#[test]
fn gemini_encoder_keeps_id_stable_across_stream() {
    let codec = GeminiCodec::new();
    let mut enc = codec.stream_encoder();

    let mut frames = Vec::new();
    frames.extend(
        enc.encode_event(&StreamEvent::MessageStart {
            id: "chatcmpl-fixed".into(),
            model: "gemini-3".into(),
        })
        .unwrap(),
    );
    for text in ["a", "b", "c", "d"] {
        frames.extend(
            enc.encode_event(&StreamEvent::TextDelta {
                index: 0,
                text: text.into(),
            })
            .unwrap(),
        );
    }
    frames.extend(
        enc.encode_event(&StreamEvent::MessageStop {
            reason: StopReason::Stop,
        })
        .unwrap(),
    );

    let ids: Vec<String> = frames_to_values(&frames)
        .iter()
        .filter_map(|v| v.get("id").and_then(Value::as_str).map(str::to_string))
        .collect();

    assert!(ids.len() >= 4, "应有多帧带 id，实际: {ids:?}");
    assert!(
        ids.iter().all(|id| id == "chatcmpl-fixed"),
        "全流 id 必须一致（旧实现每块 uuid 必失败），实际: {ids:?}"
    );
}

// ---------- 缺陷三：usage 帧形状 ----------

// **硬反例**：usage 帧不得是 `choices: []`（旧实现如此，多数客户端拒绝）。
#[test]
fn gemini_usage_frame_carries_usage_metadata() {
    let codec = GeminiCodec::new();
    let mut enc = codec.stream_encoder();

    let frames = enc
        .encode_event(&StreamEvent::MessageDelta {
            usage: Some(gateway_protocol_bridge::ir::Usage {
                prompt_tokens: 8,
                completion_tokens: 5,
                cached_tokens: None,
            }),
        })
        .unwrap();

    let values = frames_to_values(&frames);
    let usage_frame = values
        .iter()
        .find(|v| v.get("usageMetadata").is_some())
        .expect("应有一帧带 usageMetadata");

    assert_eq!(usage_frame["usageMetadata"]["promptTokenCount"], 8);
    assert_eq!(usage_frame["usageMetadata"]["candidatesTokenCount"], 5);
    assert_eq!(usage_frame["usageMetadata"]["totalTokenCount"], 13);

    // 帧里必须带 candidates 数组（结构完整），而不是空的 choices。
    assert!(
        usage_frame["candidates"].is_array(),
        "usage 帧必须带 candidates 数组，实际: {usage_frame}"
    );
}

// ---------- 非流式响应 ----------

#[test]
fn gemini_decode_response_maps_parts_and_usage() {
    let codec = GeminiCodec::new();
    let body = serde_json::to_vec(&json!({
        "candidates": [{
            "content": {"role": "model", "parts": [
                {"text": "calling a tool"},
                {"functionCall": {"name": "search", "args": {"q": "z"}}}
            ]},
            "finishReason": "STOP"
        }],
        "usageMetadata": {
            "promptTokenCount": 6,
            "candidatesTokenCount": 3,
            "cachedContentTokenCount": 2
        }
    }))
    .unwrap();

    let resp = codec.decode_response(Bytes::from(body)).expect("decode");
    assert_eq!(resp.usage.prompt_tokens, 6);
    assert_eq!(resp.usage.completion_tokens, 3);
    assert_eq!(resp.usage.cached_tokens, Some(2));
    assert!(
        resp.outputs
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { name, .. } if name == "search")),
        "非流式响应里的 functionCall 不能丢，实际: {:?}",
        resp.outputs
    );
}

#[test]
fn gemini_encode_response_uses_gemini_field_names() {
    let codec = GeminiCodec::new();
    let resp = gateway_protocol_bridge::ir::LlmResponse {
        id: "r1".into(),
        model: "gemini-3".into(),
        outputs: vec![ContentBlock::Text { text: "hi".into() }],
        usage: gateway_protocol_bridge::ir::Usage {
            prompt_tokens: 2,
            completion_tokens: 3,
            cached_tokens: None,
        },
        stop_reason: StopReason::Stop,
    };

    let out: Value = serde_json::from_slice(&codec.encode_response(&resp).unwrap()).unwrap();
    assert_eq!(out["candidates"][0]["content"]["role"], "model");
    assert_eq!(out["candidates"][0]["finishReason"], "STOP");
    assert_eq!(out["usageMetadata"]["promptTokenCount"], 2);
    assert_eq!(out["usageMetadata"]["candidatesTokenCount"], 3);
}

// ---------- 流式解码 ----------

// 多个 part 必须占各自的 index：全塞进 0 会让下游按 index 归属的格式互相覆盖。
#[test]
fn gemini_multiple_parts_get_distinct_indices() {
    let codec = GeminiCodec::new();
    let frame = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [
                {"text": "first"},
                {"text": "second"}
            ]}
        }]
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");
    let indices: Vec<u32> = events
        .iter()
        .filter_map(|e| match e {
            StreamEvent::TextDelta { index, .. } => Some(*index),
            _ => None,
        })
        .collect();

    assert_eq!(indices.len(), 2);
    assert_ne!(indices[0], indices[1], "两个 part 的 index 不得相同");
}

// functionCall 在流式里是完整对象，应产出 start（含 name）+ 一次完整 args 增量。
#[test]
fn gemini_decode_event_emits_function_call_pair() {
    let codec = GeminiCodec::new();
    let frame = json!({
        "candidates": [{
            "content": {"role": "model", "parts": [
                {"functionCall": {"name": "search", "args": {"q": "y"}}}
            ]}
        }]
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");

    let start = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::ToolCallStart { id, name, .. } => Some((id.clone(), name.clone())),
            _ => None,
        })
        .expect("应产出 ToolCallStart");
    assert_eq!(start.1, "search");
    assert!(
        start.0.starts_with("gemini_call_"),
        "id 应是确定性合成值（不是 uuid），实际: {}",
        start.0
    );

    let args = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::ToolCallDelta {
                arguments_delta, ..
            } => Some(arguments_delta.clone()),
            _ => None,
        })
        .expect("应产出 ToolCallDelta");
    let parsed: Value = serde_json::from_str(&args).expect("args 应是 JSON");
    assert_eq!(parsed["q"], "y");
}

// 非 JSON / 控制帧不得报错。
#[test]
fn gemini_decode_event_ignores_noise() {
    let codec = GeminiCodec::new();
    for frame in ["", "[DONE]", "not json", "{\"no_candidates\":1}"] {
        let events = codec.decode_event(frame).expect("不得报错");
        assert!(
            events.is_empty(),
            "{frame:?} 应产出空事件，实际: {events:?}"
        );
    }
}

// ---------- 流式编码 ----------

// tool 参数要累积到收尾才吐完整 functionCall（Gemini 的 args 是整体对象而非增量）。
#[test]
fn gemini_encoder_accumulates_tool_args_until_stop() {
    let codec = GeminiCodec::new();
    let mut enc = codec.stream_encoder();

    let mut out = Vec::new();
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallStart {
            index: 0,
            id: "call_1".into(),
            name: "search".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 0,
            arguments_delta: "{\"q\":".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 0,
            arguments_delta: "\"w\"}".into(),
        })
        .unwrap(),
    );

    // 参数未完成前不应吐出 functionCall（半截 JSON 会让 Gemini 客户端解析失败）。
    assert!(
        !frames_to_values(&out).iter().any(|v| v["candidates"][0]
            .get("content")
            .and_then(|c| c.get("parts"))
            .and_then(Value::as_array)
            .is_some_and(|p| p.iter().any(|x| x.get("functionCall").is_some()))),
        "参数累积期间不得吐出 functionCall"
    );

    let finish = enc
        .encode_event(&StreamEvent::MessageStop {
            reason: StopReason::ToolUse,
        })
        .unwrap();

    let fc = frames_to_values(&finish)
        .into_iter()
        .find_map(|v| {
            v["candidates"][0]["content"]["parts"]
                .as_array()
                .and_then(|p| p.first().cloned())
                .filter(|p| p.get("functionCall").is_some())
        })
        .expect("收尾必须吐出完整的 functionCall");

    assert_eq!(fc["functionCall"]["name"], "search");
    assert_eq!(fc["functionCall"]["args"]["q"], "w", "参数要拼成完整对象");
}

// 断流时 finish 必须补收尾帧，重复调用幂等。
#[test]
fn gemini_encoder_finish_backfills_and_is_idempotent() {
    let codec = GeminiCodec::new();
    let mut enc = codec.stream_encoder();

    enc.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "x".into(),
    })
    .unwrap();

    let out = enc.finish().expect("finish");
    let values = frames_to_values(&out);
    assert!(
        values
            .iter()
            .any(|v| v["candidates"][0]["finishReason"] == "STOP"),
        "finish 必须补一个收尾帧，实际: {values:?}"
    );

    let again = enc.finish().expect("再次 finish");
    assert!(again.is_empty(), "重复 finish 不该再吐帧，实际: {again:?}");
}

#[test]
fn gemini_codec_reports_gemini_format() {
    assert_eq!(GeminiCodec::new().format(), Protocol::Gemini);
}
