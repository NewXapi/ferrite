//! OpenAI Responses codec 的行为测试。
//!
//! 重点有三：输入三种形态的解析（Responses 的 `input` 可以是字符串、消息数组、
//! 或 items 数组）、扁平 tools 形状（与 Chat 的 `function` 子对象不同），以及
//! 流式事件的**生命周期顺序**——Responses 客户端解析器严格依赖
//! `response.created` → `output_item.added` → delta → `response.completed`，
//! 顺序错了或边界帧缺失会直接报错。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::format_codec::FormatCodec;
use gateway_protocol_bridge::ir::{ContentBlock, Role, StopReason, StreamEvent};
use gateway_protocol_bridge::responses::ResponsesCodec;
use serde_json::{Value, json};

/// 把编码器输出拆成 (事件类型, 负载) 序列，便于按顺序断言。
fn frames(frames: &[Bytes]) -> Vec<(String, Value)> {
    let mut out = Vec::new();
    for frame in frames {
        let text = String::from_utf8_lossy(frame);
        let mut kind = None;
        let mut payload = None;
        for line in text.lines() {
            if let Some(e) = line.strip_prefix("event: ") {
                kind = Some(e.to_string());
            }
            if let Some(d) = line.strip_prefix("data: ") {
                payload = serde_json::from_str(d).ok();
            }
        }
        if let (Some(k), Some(p)) = (kind, payload) {
            out.push((k, p));
        }
    }
    out
}

fn event_kinds(raw: &[Bytes]) -> Vec<String> {
    frames(raw).into_iter().map(|(k, _)| k).collect()
}

/// 某事件类型首次出现的下标。
fn first_index(kinds: &[String], kind: &str) -> Option<usize> {
    kinds.iter().position(|k| k == kind)
}

// ---------- 输入三种形态 ----------

// 形态一：裸字符串 input 解成单条 user 消息。
#[test]
fn responses_string_input_becomes_single_user_message() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-5",
        "input": "hello there"
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.messages.len(), 1);
    assert_eq!(ir.messages[0].role, Role::User);
    assert!(
        matches!(&ir.messages[0].content[0], ContentBlock::Text { text } if text == "hello there"),
        "字符串 input 应成为文本块，实际: {:?}",
        ir.messages[0].content
    );
}

// 形态二：消息数组（role + content 为 input_text part 数组）。
#[test]
fn responses_message_array_input_parses_parts() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-5",
        "instructions": "be terse",
        "input": [
            {"role": "user", "content": [{"type": "input_text", "text": "ping"}]},
            {"role": "assistant", "content": [{"type": "output_text", "text": "pong"}]}
        ]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.system.len(), 1, "instructions 应成为 system");
    assert_eq!(ir.system[0].text, "be terse");
    assert_eq!(ir.messages.len(), 2);
    assert_eq!(ir.messages[0].role, Role::User);
    assert_eq!(ir.messages[1].role, Role::Assistant);
    assert!(
        matches!(&ir.messages[1].content[0], ContentBlock::Text { text } if text == "pong"),
        "output_text part 也要解析，实际: {:?}",
        ir.messages[1].content
    );
}

// 形态三：items 数组（function_call / function_call_output）。
// 这两者必须映射成 ToolUse / ToolResult 并保住 call_id 关联——丢了关联，
// 多轮工具调用就无法把结果对回发起它的那次调用。
#[test]
fn responses_item_array_maps_function_call_and_output() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-5",
        "input": [
            {"role": "user", "content": "search for ferrite"},
            {"type": "function_call", "call_id": "call_1", "name": "search",
             "arguments": "{\"q\":\"ferrite\"}"},
            {"type": "function_call_output", "call_id": "call_1", "output": "3 hits"}
        ]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.messages.len(), 3);

    let tool_use = ir.messages[1]
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => {
                Some((id.clone(), name.clone(), input.clone()))
            }
            _ => None,
        })
        .expect("function_call 应成为 ToolUse");
    assert_eq!(tool_use.0, "call_1");
    assert_eq!(tool_use.1, "search");
    assert_eq!(tool_use.2["q"], "ferrite", "arguments 字符串要解析成 JSON");

    assert_eq!(ir.messages[2].role, Role::Tool);
    assert!(
        ir.messages[2].content.iter().any(
            |b| matches!(b, ContentBlock::ToolResult { tool_use_id, content }
                if tool_use_id == "call_1" && content == "3 hits")
        ),
        "function_call_output 应成为关联 call_1 的 ToolResult，实际: {:?}",
        ir.messages[2].content
    );
}

// Responses 的 tools 是扁平的（name/parameters 在顶层），不像 Chat 包在 function 里。
// 用 Chat 的形状去解析会静默拿到零个工具。
#[test]
fn responses_flat_tool_shape_is_parsed() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-5",
        "input": "hi",
        "tools": [{
            "type": "function",
            "name": "search",
            "description": "find things",
            "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
        }]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.tools.len(), 1, "扁平 tools 必须解析出工具");
    assert_eq!(ir.tools[0].name, "search");
    assert_eq!(ir.tools[0].description.as_deref(), Some("find things"));
    assert_eq!(
        ir.tools[0].input_schema["properties"]["q"]["type"], "string",
        "parameters 必须进 input_schema"
    );

    // 回写也必须是扁平形状。
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(out["tools"][0]["name"], "search");
    assert!(
        out["tools"][0].get("function").is_none(),
        "Responses 的 tools 不该包 function 子对象，实际: {}",
        out["tools"][0]
    );
}

// instructions 回写为 instructions，不回插成 system 消息。
#[test]
fn responses_encode_request_uses_instructions_and_input() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-5",
        "instructions": "be brief",
        "input": "hello",
        "max_output_tokens": 128
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();

    assert_eq!(out["instructions"], "be brief");
    assert!(
        out.get("messages").is_none(),
        "不得回写成 Chat 的 messages 字段，实际: {out}"
    );
    assert_eq!(out["input"][0]["role"], "user");
    assert_eq!(out["max_output_tokens"], 128, "max_output_tokens 要保住");
}

// ---------- 非流式响应 ----------

// output 里的 function_call 的 arguments 是 JSON 字符串，必须解析成对象。
// 不解析的话下游 codec 拿到的是一坨字符串，转成别的格式会变成转义过的文本。
#[test]
fn responses_decode_response_parses_function_call_arguments() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "id": "resp_1",
        "object": "response",
        "status": "completed",
        "model": "gpt-5",
        "output": [
            {"type": "message", "content": [{"type": "output_text", "text": "calling"}]},
            {"type": "function_call", "call_id": "call_7", "name": "search",
             "arguments": "{\"q\":\"x\"}"}
        ],
        "usage": {"input_tokens": 10, "output_tokens": 3,
                  "input_tokens_details": {"cached_tokens": 2}}
    }))
    .unwrap();

    let resp = codec.decode_response(Bytes::from(body)).expect("decode");
    assert_eq!(resp.id, "resp_1");
    assert_eq!(resp.stop_reason, StopReason::ToolUse);
    assert_eq!(resp.usage.prompt_tokens, 10);
    assert_eq!(resp.usage.completion_tokens, 3);
    assert_eq!(resp.usage.cached_tokens, Some(2));

    assert!(
        resp.outputs
            .iter()
            .any(|b| matches!(b, ContentBlock::Text { text } if text == "calling")),
        "message item 的 output_text 要成为 Text，实际: {:?}",
        resp.outputs
    );

    let use_block = resp
        .outputs
        .iter()
        .find_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => {
                Some((id.clone(), name.clone(), input.clone()))
            }
            _ => None,
        })
        .expect("function_call 应成为 ToolUse");
    assert_eq!(use_block.0, "call_7");
    assert_eq!(use_block.1, "search");
    assert_eq!(
        use_block.2["q"], "x",
        "arguments 必须是解析后的对象，实际: {:?}",
        use_block.2
    );
}

// status=incomplete 表达截断，应映射成 Length 而不是 Stop。
#[test]
fn responses_incomplete_status_maps_to_length() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({
        "id": "resp_2",
        "status": "incomplete",
        "incomplete_details": {"reason": "max_output_tokens"},
        "output": [{"type": "message", "content": [{"type": "output_text", "text": "partial"}]}],
        "usage": {"input_tokens": 1, "output_tokens": 2}
    }))
    .unwrap();

    let resp = codec.decode_response(Bytes::from(body)).expect("decode");
    assert_eq!(
        resp.stop_reason,
        StopReason::Length,
        "incomplete 必须是 Length，否则上游会当成正常结束"
    );
}

// ---------- 流式解码 ----------

// output_item.added(function_call) 携带 call_id/name——只在这里出现。
#[test]
fn responses_decode_event_reads_function_call_start() {
    let codec = ResponsesCodec::new();
    let frame = json!({
        "type": "response.output_item.added",
        "output_index": 1,
        "item": {"type": "function_call", "id": "fc_1", "call_id": "call_1",
                 "name": "search", "arguments": "", "status": "in_progress"}
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");
    let start = events
        .iter()
        .find_map(|e| match e {
            StreamEvent::ToolCallStart { index, id, name } => {
                Some((*index, id.clone(), name.clone()))
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("应产出 ToolCallStart，实际: {events:?}"));

    assert_eq!(start.0, 1, "index 应取 output_index");
    assert_eq!(start.1, "call_1");
    assert_eq!(start.2, "search");
}

// 生命周期控制帧不产出 IR 事件，但也不能报错。
#[test]
fn responses_decode_event_ignores_lifecycle_noise() {
    let codec = ResponsesCodec::new();
    for frame in [
        "{\"type\":\"response.content_part.added\"}",
        "{\"type\":\"response.output_item.done\"}",
        "{\"type\":\"response.output_text.done\"}",
        "not json",
        "",
    ] {
        let events = codec.decode_event(frame).expect("不得报错");
        assert!(events.is_empty(), "{frame} 应产出空事件，实际: {events:?}");
    }
}

// response.failed 是终态错误，必须转成 Error 而不是静默忽略。
#[test]
fn responses_decode_event_reports_failure() {
    let codec = ResponsesCodec::new();
    let frame = json!({
        "type": "response.failed",
        "response": {"id": "resp_x", "status": "failed",
                     "error": {"code": "server_error", "message": "boom"}}
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::Error { message } if message == "boom")),
        "response.failed 必须成为 Error，实际: {events:?}"
    );
}

// ---------- 流式编码：事件生命周期 ----------

// **硬反例**：第一个内容帧之前必须先有 `response.created`，且文本 delta 之前必须
// 先有 `response.output_item.added`。缺任何一个，Responses 客户端解析器直接报错。
#[test]
fn responses_encoder_emits_lifecycle_prelude_before_text() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    let out = enc
        .encode_event(&StreamEvent::TextDelta {
            index: 0,
            text: "hi".into(),
        })
        .expect("encode");

    let kinds = event_kinds(&out);
    let created = first_index(&kinds, "response.created");
    let item_added = first_index(&kinds, "response.output_item.added");
    let delta = first_index(&kinds, "response.output_text.delta");

    let created = created.unwrap_or_else(|| panic!("缺 response.created，实际: {kinds:?}"));
    let item_added = item_added.unwrap_or_else(|| panic!("缺 output_item.added，实际: {kinds:?}"));
    let delta = delta.unwrap_or_else(|| panic!("缺 output_text.delta，实际: {kinds:?}"));

    assert!(
        created < item_added,
        "created 必须早于 item.added，实际: {kinds:?}"
    );
    assert!(
        item_added < delta,
        "item.added 必须早于文本 delta，实际: {kinds:?}"
    );

    // created 只发一次。
    let mut all = out;
    all.extend(
        enc.encode_event(&StreamEvent::TextDelta {
            index: 0,
            text: " again".into(),
        })
        .unwrap(),
    );
    let created_count = event_kinds(&all)
        .iter()
        .filter(|k| *k == "response.created")
        .count();
    assert_eq!(created_count, 1, "response.created 只应发一次");
}

// tool 调用要产出 added(function_call) → arguments.delta，并在收尾时 completed。
#[test]
fn responses_encoder_emits_tool_lifecycle() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    let mut out = Vec::new();
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallStart {
            index: 1,
            id: "call_1".into(),
            name: "search".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 1,
            arguments_delta: "{\"q\":".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 1,
            arguments_delta: "\"y\"}".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::MessageStop {
            reason: StopReason::ToolUse,
        })
        .unwrap(),
    );

    let pairs = frames(&out);
    let added = pairs
        .iter()
        .find(|(k, _)| k == "response.output_item.added")
        .expect("应有 output_item.added");
    assert_eq!(added.1["item"]["type"], "function_call");
    assert_eq!(added.1["item"]["call_id"], "call_1");
    assert_eq!(added.1["item"]["name"], "search");

    let deltas: Vec<&Value> = pairs
        .iter()
        .filter(|(k, _)| k == "response.function_call_arguments.delta")
        .map(|(_, v)| v)
        .collect();
    assert_eq!(deltas.len(), 2, "两段参数碎片各一帧");
    let recombined: String = deltas
        .iter()
        .map(|d| d["delta"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(recombined, "{\"q\":\"y\"}");

    // 收尾：参数 done + completed，且 completed 的 output 里 arguments 是完整的。
    let kinds = event_kinds(&out);
    assert!(
        kinds
            .iter()
            .any(|k| k == "response.function_call_arguments.done"),
        "tool 参数要有 done 帧，实际: {kinds:?}"
    );
    let completed = pairs
        .iter()
        .find(|(k, _)| k == "response.completed")
        .expect("应有 response.completed");
    let items = completed.1["response"]["output"]
        .as_array()
        .expect("completed 的 output 必须是数组");
    assert!(
        items.iter().any(|i| i["type"] == "function_call"
            && i["arguments"] == "{\"q\":\"y\"}"
            && i["call_id"] == "call_1"),
        "completed 快照里的 function_call 必须带完整参数，实际: {items:?}"
    );
}

// 截断（Length）必须以 response.incomplete 收尾，而不是声称 completed。
// 否则客户端会以为拿到了完整回答。
#[test]
fn responses_encoder_truncation_ends_incomplete() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    enc.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "partial".into(),
    })
    .unwrap();
    let out = enc
        .encode_event(&StreamEvent::MessageStop {
            reason: StopReason::Length,
        })
        .unwrap();

    let kinds = event_kinds(&out);
    assert!(
        kinds.iter().any(|k| k == "response.incomplete"),
        "截断必须是 incomplete，实际: {kinds:?}"
    );
    assert!(
        !kinds.iter().any(|k| k == "response.completed"),
        "截断不得同时报 completed，实际: {kinds:?}"
    );
}

// 断流时 finish 必须补收尾；重复调用幂等。
#[test]
fn responses_encoder_finish_backfills_and_is_idempotent() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    enc.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "x".into(),
    })
    .unwrap();

    let out = enc.finish().expect("finish");
    let kinds = event_kinds(&out);
    assert!(
        kinds.iter().any(|k| k == "response.completed"),
        "finish 必须补 response.completed，实际: {kinds:?}"
    );

    let again = enc.finish().expect("再次 finish");
    assert!(again.is_empty(), "重复 finish 不该再吐帧，实际: {again:?}");
}

// 失败是终态：不得在 response.failed 之后再补 response.completed。
#[test]
fn responses_encoder_error_is_terminal() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    let out = enc
        .encode_event(&StreamEvent::Error {
            message: "boom".into(),
        })
        .unwrap();
    assert!(event_kinds(&out).iter().any(|k| k == "response.failed"));

    let after = enc.finish().expect("finish");
    assert!(
        !event_kinds(&after)
            .iter()
            .any(|k| k == "response.completed"),
        "失败后不得再报 completed，实际: {after:?}"
    );
}

// 文本与工具项的 output_index 必须互不撞号（两者共用一条自增序列）。
// 撞号会让客户端把工具项当成文本项的一部分，静默拼错。
#[test]
fn responses_encoder_text_and_tool_indices_do_not_collide() {
    let codec = ResponsesCodec::new();
    let mut enc = codec.stream_encoder();

    let mut out = Vec::new();
    out.extend(
        enc.encode_event(&StreamEvent::TextDelta {
            index: 0,
            text: "hi".into(),
        })
        .unwrap(),
    );
    out.extend(
        enc.encode_event(&StreamEvent::ToolCallStart {
            index: 1,
            id: "call_1".into(),
            name: "search".into(),
        })
        .unwrap(),
    );

    let added_indices: Vec<u64> = frames(&out)
        .iter()
        .filter(|(k, _)| k == "response.output_item.added")
        .filter_map(|(_, v)| v["output_index"].as_u64())
        .collect();

    assert_eq!(added_indices.len(), 2, "文本与工具各开一个 item");
    assert_ne!(
        added_indices[0], added_indices[1],
        "文本项与工具项的 output_index 不得相同，实际: {added_indices:?}"
    );
}

#[test]
fn responses_codec_reports_responses_format() {
    assert_eq!(ResponsesCodec::new().format(), Protocol::OpenAIResp);
}

// 两跳：IR 缺 input 时也必须是合法数组（不能产出 null 让上游 400）。
#[test]
fn responses_encode_request_always_emits_input_array() {
    let codec = ResponsesCodec::new();
    let body = serde_json::to_vec(&json!({"model": "gpt-5", "input": "hi"})).unwrap();
    let mut ir = codec.decode_request(Bytes::from(body)).expect("decode");
    ir.messages.clear();

    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(out["input"], json!([]), "input 必须是数组");
}
