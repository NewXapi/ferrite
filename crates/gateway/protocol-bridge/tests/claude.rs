//! Claude codec 的行为测试：请求/响应双向 round-trip、流式事件的 G1 三个具体缺陷、
//! 以及流编码器的 block 边界补齐。
//!
//! 重点在 `decode_event` 与 `stream_encoder`：旧实现（`adaptor::convert_claude_event_to_openai`）
//! 在这里有三个真实缺陷，本文件的用例专门盯着它们——若有人把旧行为改回来，这些用例必然失败。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::claude::ClaudeCodec;
use gateway_protocol_bridge::format_codec::FormatCodec;
use gateway_protocol_bridge::ir::{
    ContentBlock, LlmResponse, Role, StopReason, StreamEvent, Usage,
};
use serde_json::{Value, json};

/// 把 `data:` 帧负载（以及可能的 `event:` 行）拆成 JSON 值，便于断言。
fn frames_to_values(frames: &[Bytes]) -> Vec<Value> {
    let mut out = Vec::new();
    for frame in frames {
        let text = String::from_utf8_lossy(frame);
        for line in text.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                out.push(serde_json::from_str(data).expect("frame data 应为合法 JSON"));
            }
        }
    }
    out
}

/// 取出所有帧里 `type` 等于给定值的那些。
fn frames_of_type(frames: &[Bytes], kind: &str) -> Vec<Value> {
    frames_to_values(frames)
        .into_iter()
        .filter(|v| v.get("type").and_then(Value::as_str) == Some(kind))
        .collect()
}

// ---------- 请求方向 ----------

// 请求双向 round-trip：tool_use / tool_result / thinking 三种 block 经 IR 往返后
// 内容不丢。防的是「新 codec 只搬 text，把结构化 block 当未知丢掉」。
#[test]
fn claude_request_roundtrip_preserves_tool_and_thinking_blocks() {
    let codec = ClaudeCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "claude-sonnet",
        "max_tokens": 512,
        "temperature": 0.3,
        "system": "be brief",
        "messages": [
            {"role": "user", "content": "hi"},
            {"role": "assistant", "content": [
                {"type": "text", "text": "calling"},
                {"type": "tool_use", "id": "tu_1", "name": "search", "input": {"q": "ferrite"}},
                {"type": "thinking", "thinking": "reasoning here"}
            ]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "tu_1", "content": "found 3"}
            ]}
        ]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.model, "claude-sonnet");
    assert_eq!(ir.system.len(), 1);
    assert_eq!(ir.system[0].text, "be brief");
    assert_eq!(
        ir.sampling.as_ref().and_then(|s| s.max_tokens),
        Some(512),
        "max_tokens 必须进 sampling"
    );

    let assistant = &ir.messages[1];
    assert_eq!(assistant.role, Role::Assistant);
    assert!(
        assistant
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { name, .. } if name == "search")),
        "tool_use 必须成为 ToolUse block，实际: {:?}",
        assistant.content
    );
    assert!(
        assistant
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::Thinking { text } if text == "reasoning here")),
        "thinking 必须成为 Thinking block，实际: {:?}",
        assistant.content
    );

    let user = &ir.messages[2];
    assert!(
        user.content.iter().any(
            |b| matches!(b, ContentBlock::ToolResult { tool_use_id, content }
                if tool_use_id == "tu_1" && content == "found 3")
        ),
        "tool_result 必须成为 ToolResult block，实际: {:?}",
        user.content
    );

    // 回到 Claude 形状：关键结构必须在。
    let back = codec.encode_request(&ir).expect("encode");
    let out: Value = serde_json::from_slice(&back).unwrap();
    assert_eq!(out["model"], "claude-sonnet");
    assert_eq!(out["max_tokens"], 512);
    assert_eq!(out["system"][0]["text"], "be brief");
    assert_eq!(out["messages"][1]["content"][1]["type"], "tool_use");
    assert_eq!(out["messages"][2]["content"][0]["tool_use_id"], "tu_1");
}

// max_tokens 缺失时必须补默认值：Claude 的 max_tokens 是必填项，漏了就 400。
// 防的是「IR 没有 max_tokens 就原样发出去」。
#[test]
fn claude_encode_request_fills_required_max_tokens() {
    let codec = ClaudeCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "claude-sonnet",
        "messages": [{"role": "user", "content": "hi"}]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert!(ir.sampling.is_none(), "无采样参数时应为 None");

    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert!(
        out["max_tokens"].as_u64().is_some_and(|v| v > 0),
        "Claude 必填的 max_tokens 必须补默认值，实际: {out}"
    );
}

// ---------- G1 核心回归 ----------

/// 构造一条包含 tool 调用的完整 Claude SSE 序列（逐帧的 data 负载）。
fn claude_tool_stream_frames() -> Vec<String> {
    vec![
        json!({
            "type": "message_start",
            "message": {
                "id": "msg_1", "type": "message", "role": "assistant", "model": "claude-sonnet",
                "content": [], "usage": {"input_tokens": 11, "output_tokens": 0}
            }
        })
        .to_string(),
        json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": {"type": "tool_use", "id": "tu_1", "name": "search", "input": {}}
        })
        .to_string(),
        json!({
            "type": "content_block_delta",
            "index": 1,
            "delta": {"type": "input_json_delta", "partial_json": "{\"q\":"}
        })
        .to_string(),
        json!({
            "type": "content_block_delta",
            "index": 1,
            "delta": {"type": "input_json_delta", "partial_json": "\"ferrite\"}"}
        })
        .to_string(),
        json!({"type": "content_block_stop", "index": 1}).to_string(),
        json!({
            "type": "message_delta",
            "delta": {"stop_reason": "tool_use"},
            "usage": {"output_tokens": 7}
        })
        .to_string(),
        json!({"type": "message_stop"}).to_string(),
    ]
}

// G1-a：`content_block_start`(tool_use) 必须产出 ToolCallStart，带上 id 与 name。
// 旧实现没有这个 match 分支，事件被静默丢弃，客户端永远拿不到工具身份。
#[test]
fn claude_decode_event_emits_tool_call_start_from_content_block_start() {
    let codec = ClaudeCodec::new();
    let events = codec
        .decode_event(&claude_tool_stream_frames()[1])
        .expect("decode content_block_start");

    let start = events
        .iter()
        .find(|e| matches!(e, StreamEvent::ToolCallStart { .. }))
        .unwrap_or_else(|| {
            panic!("content_block_start(tool_use) 必须产出 ToolCallStart，实际: {events:?}")
        });

    match start {
        StreamEvent::ToolCallStart { index, id, name } => {
            assert_eq!(*index, 1, "index 必须取事件自带值");
            assert_eq!(id, "tu_1", "tool id 必须带出来");
            assert_eq!(name, "search", "tool name 必须带出来");
        }
        other => panic!("期望 ToolCallStart，实际: {other:?}"),
    }
}

// G1-b：tool 碎片的 index 必须取事件自带值，不能硬编码 0。
// 旧实现写死 index 0，并行工具调用会全部塌到同一个索引上互相覆盖。
#[test]
fn claude_tool_fragments_keep_their_own_index() {
    let codec = ClaudeCodec::new();
    let frames = claude_tool_stream_frames();

    let mut indices = Vec::new();
    let mut args = String::new();
    for frame in &frames {
        for ev in codec.decode_event(frame).expect("decode") {
            if let StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } = ev
            {
                indices.push(index);
                args.push_str(&arguments_delta);
            }
        }
    }

    assert_eq!(
        indices,
        vec![1, 1],
        "两段碎片的 index 都必须是 1（旧实现硬编码 0）"
    );
    let parsed: Value = serde_json::from_str(&args).expect("拼起来的参数应是合法 JSON");
    assert_eq!(parsed["q"], "ferrite", "碎片必须拼回完整 JSON");
}

// G1-b 加强：两个并行工具调用的分片不得互相污染。
// 这是硬编码 index 0 的致命场景——两个调用会串成同一份参数。
#[test]
fn claude_parallel_tool_calls_do_not_cross_contaminate() {
    let codec = ClaudeCodec::new();
    let frames = [
        json!({"type": "content_block_start", "index": 1,
               "content_block": {"type": "tool_use", "id": "a", "name": "first"}}),
        json!({"type": "content_block_start", "index": 2,
               "content_block": {"type": "tool_use", "id": "b", "name": "second"}}),
        json!({"type": "content_block_delta", "index": 1,
               "delta": {"type": "input_json_delta", "partial_json": "{\"k\":1}"}}),
        json!({"type": "content_block_delta", "index": 2,
               "delta": {"type": "input_json_delta", "partial_json": "{\"k\":2}"}}),
    ];

    let mut by_index: std::collections::BTreeMap<u32, String> = std::collections::BTreeMap::new();
    for frame in &frames {
        for ev in codec.decode_event(&frame.to_string()).expect("decode") {
            if let StreamEvent::ToolCallDelta {
                index,
                arguments_delta,
            } = ev
            {
                by_index
                    .entry(index)
                    .or_default()
                    .push_str(&arguments_delta);
            }
        }
    }

    assert_eq!(by_index.get(&1).map(String::as_str), Some("{\"k\":1}"));
    assert_eq!(by_index.get(&2).map(String::as_str), Some("{\"k\":2}"));
}

// G1-c：message_start 的 input usage 不得丢失。
// 旧实现整块丢弃，计量因此少了 prompt 侧 token。
#[test]
fn claude_message_start_usage_is_preserved() {
    let codec = ClaudeCodec::new();
    let events = codec
        .decode_event(&claude_tool_stream_frames()[0])
        .expect("decode message_start");

    assert!(
        matches!(events.first(), Some(StreamEvent::MessageStart { id, .. }) if id == "msg_1"),
        "首事件应为 MessageStart，实际: {events:?}"
    );

    let usage = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::MessageDelta { usage: Some(u) } => Some(u.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("message_start 的 input usage 必须保留，实际: {events:?}"));

    assert_eq!(
        usage.prompt_tokens, 11,
        "input_tokens 必须映射到 prompt_tokens"
    );
}

// 未知/控制帧不得让整条流失败：SSE 流里存在 ping 之类的非内容帧。
// 防的是「一帧解析不了就 Err，整条流断掉」。
#[test]
fn claude_non_content_frames_are_ignored() {
    let codec = ClaudeCodec::new();
    for frame in [
        "",
        "not json at all",
        "{\"type\":\"ping\"}",
        "{\"no_type\":1}",
    ] {
        let events = codec.decode_event(frame).expect("不得报错");
        assert!(events.is_empty(), "非内容帧应产出空事件，实际: {events:?}");
    }
}

// ---------- 流编码器 ----------

// 编码器必须补齐 Claude 的 block 边界帧：IR 事件流只有带 index 的 delta，
// 而 Claude 协议要求 start/delta/stop 成套。缺了 start，客户端解析器直接报错。
#[test]
fn claude_encoder_wraps_tool_deltas_with_block_boundaries() {
    let codec = ClaudeCodec::new();
    let mut enc = codec.stream_encoder();

    let mut frames = Vec::new();
    frames.extend(
        enc.encode_event(&StreamEvent::MessageStart {
            id: "msg_1".into(),
            model: "claude-sonnet".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallStart {
            index: 1,
            id: "tu_1".into(),
            name: "search".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 1,
            arguments_delta: "{\"q\":".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 1,
            arguments_delta: "\"x\"}".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::MessageStop {
            reason: StopReason::ToolUse,
        })
        .unwrap(),
    );

    let starts = frames_of_type(&frames, "content_block_start");
    assert_eq!(starts.len(), 1, "tool block 只应开一次，实际: {starts:?}");
    assert_eq!(starts[0]["index"], 1);
    assert_eq!(starts[0]["content_block"]["type"], "tool_use");
    assert_eq!(starts[0]["content_block"]["id"], "tu_1");
    assert_eq!(starts[0]["content_block"]["name"], "search");

    let deltas = frames_of_type(&frames, "content_block_delta");
    assert_eq!(deltas.len(), 2, "两段参数碎片应各成一帧");
    let recombined: String = deltas
        .iter()
        .map(|d| d["delta"]["partial_json"].as_str().unwrap())
        .collect();
    assert_eq!(recombined, "{\"q\":\"x\"}");
    for d in &deltas {
        assert_eq!(d["index"], 1, "delta 的 index 必须与 block 一致");
        assert_eq!(d["delta"]["type"], "input_json_delta");
    }

    let stops = frames_of_type(&frames, "content_block_stop");
    assert_eq!(stops.len(), 1, "block 必须关闭，实际: {stops:?}");
    assert_eq!(
        frames_of_type(&frames, "message_delta")[0]["delta"]["stop_reason"],
        "tool_use",
        "StopReason::ToolUse 必须回成 Claude 的 tool_use"
    );
    assert_eq!(frames_of_type(&frames, "message_stop").len(), 1);
}

// 只收到 tool 碎片而没收到 ToolCallStart 时，编码器仍须开出 block。
// 防的是「上游没给 id/name 就整条 tool 调用被静默吞掉」。
#[test]
fn claude_encoder_opens_block_even_without_tool_start() {
    let codec = ClaudeCodec::new();
    let mut enc = codec.stream_encoder();

    let frames = enc
        .encode_event(&StreamEvent::ToolCallDelta {
            index: 1,
            arguments_delta: "{}".into(),
        })
        .expect("encode");

    let starts = frames_of_type(&frames, "content_block_start");
    assert_eq!(
        starts.len(),
        1,
        "缺 id/name 也必须开 tool_use block，实际: {starts:?}"
    );
    assert_eq!(starts[0]["content_block"]["type"], "tool_use");
    assert_eq!(
        frames_of_type(&frames, "content_block_delta").len(),
        1,
        "碎片本身也要吐出去"
    );
}

// finish() 必须收尾：补 start、关 block、发 message_stop；重复调用不得重复吐。
// 防的是「流提前结束，客户端拿到没有 message_stop 的半截流」。
#[test]
fn claude_encoder_finish_closes_open_stream_idempotently() {
    let codec = ClaudeCodec::new();
    let mut enc = codec.stream_encoder();

    // 内容事件会自行补出 message_start（Claude 要求任何内容帧前先有 start）。
    let head = enc
        .encode_event(&StreamEvent::TextDelta {
            index: 0,
            text: "hello".into(),
        })
        .unwrap();
    assert_eq!(
        frames_of_type(&head, "message_start").len(),
        1,
        "首个内容事件必须先补 message_start"
    );

    // finish 只负责收尾：关掉仍开启的 block 并终止消息，不得重发 message_start。
    let out = enc.finish().expect("finish");
    assert_eq!(
        frames_of_type(&out, "message_start").len(),
        0,
        "message_start 已发过，finish 不得重发"
    );
    assert_eq!(
        frames_of_type(&out, "content_block_stop").len(),
        1,
        "开启的 text block 必须关闭"
    );
    assert_eq!(frames_of_type(&out, "message_stop").len(), 1);

    let again = enc.finish().expect("finish 幂等");
    assert!(
        again.is_empty(),
        "重复 finish 不应重复吐帧，实际: {again:?}"
    );
}

// ---------- 响应方向 ----------

// 非流式响应：content 数组里的 tool_use 不得被丢掉（旧实现只提取 text）。
#[test]
fn claude_decode_response_keeps_tool_use_blocks() {
    let codec = ClaudeCodec::new();
    let body = serde_json::to_vec(&json!({
        "id": "msg_9",
        "type": "message",
        "role": "assistant",
        "model": "claude-sonnet",
        "content": [
            {"type": "text", "text": "using a tool"},
            {"type": "tool_use", "id": "tu_9", "name": "search", "input": {"q": "x"}}
        ],
        "stop_reason": "tool_use",
        "usage": {"input_tokens": 5, "output_tokens": 9}
    }))
    .unwrap();

    let resp = codec.decode_response(Bytes::from(body)).expect("decode");
    assert_eq!(resp.id, "msg_9");
    assert_eq!(resp.stop_reason, StopReason::ToolUse);
    assert_eq!(resp.usage.prompt_tokens, 5);
    assert_eq!(resp.usage.completion_tokens, 9);
    assert!(
        resp.outputs
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { id, .. } if id == "tu_9")),
        "非流式响应里的 tool_use 不能丢，实际: {:?}",
        resp.outputs
    );
}

// 响应方向 round-trip：IR → Claude 体的 usage 字段用 Claude 的命名。
#[test]
fn claude_encode_response_uses_claude_field_names() {
    let codec = ClaudeCodec::new();
    let resp = LlmResponse {
        id: "msg_1".into(),
        model: "claude-sonnet".into(),
        outputs: vec![ContentBlock::Text { text: "hi".into() }],
        usage: Usage {
            prompt_tokens: 3,
            completion_tokens: 4,
            cached_tokens: None,
        },
        stop_reason: StopReason::Stop,
    };

    let out: Value = serde_json::from_slice(&codec.encode_response(&resp).unwrap()).unwrap();
    assert_eq!(out["type"], "message");
    assert_eq!(out["role"], "assistant");
    assert_eq!(out["usage"]["input_tokens"], 3);
    assert_eq!(out["usage"]["output_tokens"], 4);
    assert_eq!(out["stop_reason"], "end_turn");
}

// ---------- 格式标识 ----------

#[test]
fn claude_codec_reports_claude_format() {
    assert_eq!(ClaudeCodec::new().format(), Protocol::Claude);
}
