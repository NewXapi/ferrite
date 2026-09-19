//! OpenAI Chat Completions codec 的行为测试。
//!
//! 两份图：一是双向 round-trip 的无损性（IR 作为中枢的硬契约），二是流式输出的
//! 形状约束——id 全流稳定、usage 帧的 choices 非空、tool 分片保住各自 index。
//! 后三条都对应仓里已记录的反例（旧 Gemini 侧每块现生成 uuid、usage 帧 `choices: []`、
//! tool index 硬编码 0）。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::Protocol;
use gateway_protocol_bridge::format_codec::FormatCodec;
use gateway_protocol_bridge::ir::{
    ContentBlock, LlmRequest, LlmResponse, Role, SamplingParams, StopReason, StreamEvent, Usage,
};
use gateway_protocol_bridge::openai::OpenAiCodec;
use serde_json::{Map, Value, json};

/// 把编码器输出的帧拆成 JSON（跳过 `[DONE]`）。
fn frames_to_values(frames: &[Bytes]) -> Vec<Value> {
    frames
        .iter()
        .filter_map(|f| {
            let text = String::from_utf8_lossy(f);
            let line = text.lines().find(|l| l.starts_with("data: "))?;
            let data = line.strip_prefix("data: ")?;
            if data == "[DONE]" {
                return None;
            }
            serde_json::from_str(data).ok()
        })
        .collect()
}

fn has_done(frames: &[Bytes]) -> bool {
    frames
        .iter()
        .any(|f| String::from_utf8_lossy(f).contains("[DONE]"))
}

fn mk_request(model: &str) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        messages: vec![],
        system: vec![],
        tools: vec![],
        tool_choice: None,
        sampling: None,
        stream: false,
        extra: Map::new(),
    }
}

// ---------- 请求方向 ----------

// 请求 round-trip：messages / tools / tool_choice / 采样参数经 IR 往返后不丢。
// 防的是「新 codec 只搬 model+messages，工具与会话参数被吃掉」。
#[test]
fn openai_request_roundtrip_preserves_tools_and_sampling() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "temperature": 0.5,
        "top_p": 0.9,
        "max_tokens": 256,
        "messages": [
            {"role": "system", "content": "be terse"},
            {"role": "user", "content": "hi"}
        ],
        "tools": [{
            "type": "function",
            "function": {
                "name": "search",
                "description": "find things",
                "parameters": {"type": "object", "properties": {"q": {"type": "string"}}}
            }
        }],
        "tool_choice": "auto"
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert_eq!(ir.model, "gpt-x");
    assert_eq!(ir.system.len(), 1, "system 消息必须提到 IR 的 system");
    assert_eq!(ir.system[0].text, "be terse");
    assert_eq!(ir.messages.len(), 1, "system 不应留在 messages 里");
    assert_eq!(ir.tools.len(), 1);
    assert_eq!(ir.tools[0].name, "search");
    assert_eq!(ir.tools[0].input_schema["type"], "object");
    let s = ir.sampling.as_ref().expect("采样参数");
    assert_eq!(s.temperature, Some(0.5));
    assert_eq!(s.top_p, Some(0.9));
    assert_eq!(s.max_tokens, Some(256));

    // 回到 Chat 形状：system 回插成首条消息，tools 回到 function 包装里。
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(out["model"], "gpt-x");
    assert_eq!(out["messages"][0]["role"], "system");
    assert_eq!(out["messages"][0]["content"], "be terse");
    assert_eq!(out["tools"][0]["function"]["name"], "search");
    assert_eq!(out["tool_choice"], "auto");
    assert_eq!(out["temperature"], 0.5);
}

// 未知顶层字段不得丢：厂商扩展参数要能原样带到上游。
#[test]
fn openai_request_keeps_unknown_fields() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "messages": [{"role": "user", "content": "hi"}],
        "vendor_specific_flag": {"nested": true}
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(
        out["vendor_specific_flag"]["nested"], true,
        "未知字段必须 round-trip，实际: {out}"
    );
}

// 流式请求必须注入 stream_options.include_usage（否则上游不回 usage，计费缺数据）；
// 客户端已显式给出 stream_options 时不得覆盖。
#[test]
fn openai_stream_request_injects_include_usage_without_clobbering() {
    let codec = OpenAiCodec::new();

    let ir = mk_request("gpt-x");
    let mut ir = LlmRequest { stream: true, ..ir };
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(
        out["stream_options"]["include_usage"], true,
        "流式请求应注入 include_usage"
    );

    // 客户端显式设置的 stream_options 优先。
    ir.extra
        .insert("stream_options".into(), json!({"include_usage": false}));
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    assert_eq!(
        out["stream_options"]["include_usage"], false,
        "客户端显式设置不得被覆盖"
    );
}

// tool 消息的 tool_call_id 要变成 ToolResult 关联，而不是丢失关联的纯文本。
#[test]
fn openai_tool_message_maps_to_tool_result_block() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "messages": [
            {"role": "assistant", "content": null, "tool_calls": [
                {"id": "call_1", "type": "function",
                 "function": {"name": "search", "arguments": "{\"q\":\"ferrite\"}"}}
            ]},
            {"role": "tool", "tool_call_id": "call_1", "content": "3 results"}
        ]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");

    let assistant = &ir.messages[0];
    assert_eq!(assistant.role, Role::Assistant);
    let tool_use = assistant
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::ToolUse { id, name, input } => {
                Some((id.clone(), name.clone(), input.clone()))
            }
            _ => None,
        })
        .expect("assistant 的 tool_calls 必须成为 ToolUse");
    assert_eq!(tool_use.0, "call_1");
    assert_eq!(tool_use.1, "search");
    assert_eq!(
        tool_use.2["q"], "ferrite",
        "arguments 字符串必须解析成 JSON"
    );

    let tool_msg = &ir.messages[1];
    assert_eq!(tool_msg.role, Role::Tool);
    assert!(
        tool_msg.content.iter().any(
            |b| matches!(b, ContentBlock::ToolResult { tool_use_id, content }
                if tool_use_id == "call_1" && content == "3 results")
        ),
        "tool 消息必须成为关联 call_1 的 ToolResult，实际: {:?}",
        tool_msg.content
    );
}

// data URL 图片必须成为 Image 块（不是 Unknown），跨格式图片转换才能落地。
#[test]
fn openai_data_url_image_becomes_image_block() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "messages": [{"role": "user", "content": [
            {"type": "text", "text": "look"},
            {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAB"}}
        ]}]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    let image = ir.messages[0]
        .content
        .iter()
        .find_map(|b| match b {
            ContentBlock::Image { media_type, data } => Some((media_type.clone(), data.clone())),
            _ => None,
        })
        .expect("data URL 图片必须成为 Image 块");

    assert_eq!(image.0, "image/png");
    assert_eq!(image.1, "AAAB", "base64 负载要去掉 data URL 前缀");
}

// 远程 URL 图片无法用 base64 形状表达，必须原样保留而不是丢弃。
#[test]
fn openai_remote_url_image_survives_as_unknown() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "messages": [{"role": "user", "content": [
            {"type": "image_url", "image_url": {"url": "https://example.com/a.png"}}
        ]}]
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    assert!(
        ir.messages[0]
            .content
            .iter()
            .any(|b| matches!(b, ContentBlock::Unknown { raw }
                if raw["image_url"]["url"] == "https://example.com/a.png")),
        "远程图片 URL 必须经逃逸口保留，实际: {:?}",
        ir.messages[0].content
    );
}

// ---------- 响应方向 ----------

#[test]
fn openai_decode_response_maps_text_tools_and_usage() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "id": "chatcmpl-1",
        "model": "gpt-x",
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": "using a tool",
                "tool_calls": [{"id": "call_9", "type": "function",
                    "function": {"name": "search", "arguments": "{\"q\":\"z\"}"}}]
            },
            "finish_reason": "tool_calls"
        }],
        "usage": {"prompt_tokens": 12, "completion_tokens": 4,
                  "prompt_tokens_details": {"cached_tokens": 5}}
    }))
    .unwrap();

    let resp = codec.decode_response(Bytes::from(body)).expect("decode");
    assert_eq!(resp.id, "chatcmpl-1");
    assert_eq!(resp.stop_reason, StopReason::ToolUse);
    assert_eq!(resp.usage.prompt_tokens, 12);
    assert_eq!(resp.usage.completion_tokens, 4);
    assert_eq!(resp.usage.cached_tokens, Some(5));
    assert!(
        resp.outputs
            .iter()
            .any(|b| matches!(b, ContentBlock::ToolUse { id, .. } if id == "call_9")),
        "响应里的 tool_calls 必须成为 ToolUse，实际: {:?}",
        resp.outputs
    );
}

#[test]
fn openai_encode_response_uses_chat_shape() {
    let codec = OpenAiCodec::new();
    let resp = LlmResponse {
        id: "chatcmpl-2".into(),
        model: "gpt-x".into(),
        outputs: vec![
            ContentBlock::Text { text: "hi".into() },
            ContentBlock::ToolUse {
                id: "call_9".into(),
                name: "search".into(),
                input: json!({"q": "z"}),
            },
        ],
        usage: Usage {
            prompt_tokens: 1,
            completion_tokens: 2,
            cached_tokens: None,
        },
        stop_reason: StopReason::ToolUse,
    };

    let out: Value = serde_json::from_slice(&codec.encode_response(&resp).unwrap()).unwrap();
    assert_eq!(out["object"], "chat.completion");
    assert_eq!(
        out["choices"][0]["message"]["tool_calls"][0]["id"],
        "call_9"
    );
    assert_eq!(out["choices"][0]["finish_reason"], "tool_calls");
    assert_eq!(out["usage"]["total_tokens"], 3);
}

// ---------- 流式解码 ----------

// 一帧里既有 choices 又有 usage（include_usage 的末帧形状）时，两类事件都要产出。
// 防的是「只取 choices，流式 usage 整块丢掉」——计费会缺一整个流的数据。
#[test]
fn openai_decode_event_emits_usage_alongside_choices() {
    let codec = OpenAiCodec::new();
    let frame = json!({
        "id": "chatcmpl-1",
        "choices": [{"index": 0, "delta": {"content": "hi"}, "finish_reason": "stop"}],
        "usage": {"prompt_tokens": 7, "completion_tokens": 2}
    })
    .to_string();

    let events = codec.decode_event(&frame).expect("decode");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::TextDelta { text, .. } if text == "hi")),
        "文本增量要产出，实际: {events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, StreamEvent::MessageStop { .. })),
        "finish_reason 要产出 MessageStop，实际: {events:?}"
    );
    assert!(
        events.iter().any(
            |e| matches!(e, StreamEvent::MessageDelta { usage: Some(u) } if u.prompt_tokens == 7)
        ),
        "同一帧的 usage 也必须产出，实际: {events:?}"
    );
}

// `[DONE]` 是控制帧，不能当内容，也不能报错。
#[test]
fn openai_decode_event_ignores_done_and_garbage() {
    let codec = OpenAiCodec::new();
    for frame in ["[DONE]", "", "   ", "not json"] {
        let events = codec.decode_event(frame).expect("不得报错");
        assert!(
            events.is_empty(),
            "{frame:?} 应产出空事件，实际: {events:?}"
        );
    }
}

// tool 分片的 index 必须取 chunk 自带值。硬编码会让并行调用串在一起。
#[test]
fn openai_decode_event_keeps_tool_call_indices() {
    let codec = OpenAiCodec::new();
    let frames = [
        json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 0, "id": "c0", "type": "function", "function": {"name": "a", "arguments": ""}}
        ]}, "finish_reason": null}]}),
        json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 1, "id": "c1", "type": "function", "function": {"name": "b", "arguments": ""}}
        ]}, "finish_reason": null}]}),
        json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 1, "function": {"arguments": "{\"k\":"}}
        ]}, "finish_reason": null}]}),
        json!({"choices": [{"index": 0, "delta": {"tool_calls": [
            {"index": 1, "function": {"arguments": "1}"}}
        ]}, "finish_reason": null}]}),
    ];

    let mut starts = Vec::new();
    let mut args_by_index: std::collections::BTreeMap<u32, String> =
        std::collections::BTreeMap::new();
    for f in &frames {
        for ev in codec.decode_event(&f.to_string()).expect("decode") {
            match ev {
                StreamEvent::ToolCallStart { index, name, .. } => starts.push((index, name)),
                StreamEvent::ToolCallDelta {
                    index,
                    arguments_delta,
                } => args_by_index
                    .entry(index)
                    .or_default()
                    .push_str(&arguments_delta),
                _ => {}
            }
        }
    }

    assert_eq!(starts, vec![(0, "a".to_string()), (1, "b".to_string())]);
    assert_eq!(args_by_index.get(&1).map(String::as_str), Some("{\"k\":1}"));
    assert!(
        !args_by_index.contains_key(&0),
        "index 0 的碎片不该出现（它没有 delta）"
    );
}

// ---------- 流式编码 ----------

// **硬反例**：chunk id 全流稳定。旧 Gemini 侧每块现生成 uuid，客户端与流式聚合
// 会把同一次补全当成多次。这里把每帧的 id 收起来断言全等。
#[test]
fn openai_encoder_keeps_chunk_id_stable_across_stream() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    let mut frames = Vec::new();
    frames.extend(
        enc.encode_event(&StreamEvent::MessageStart {
            id: "chatcmpl-fixed".into(),
            model: "gpt-x".into(),
        })
        .unwrap(),
    );
    for text in ["a", "b", "c"] {
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
        "全流 id 必须一致，实际: {ids:?}"
    );
}

// usage 帧的 choices 必须非空：`choices: []` 会被多数客户端拒绝。
#[test]
fn openai_encoder_usage_chunk_has_non_empty_choices() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    let frames = enc
        .encode_event(&StreamEvent::MessageDelta {
            usage: Some(Usage {
                prompt_tokens: 5,
                completion_tokens: 6,
                cached_tokens: None,
            }),
        })
        .unwrap();

    let values = frames_to_values(&frames);
    let usage_frame = values
        .iter()
        .find(|v| v.get("usage").is_some())
        .expect("应有一帧带 usage");

    let choices = usage_frame["choices"]
        .as_array()
        .expect("choices 必须是数组");
    assert!(
        !choices.is_empty(),
        "usage 帧的 choices 不得为空数组，实际: {usage_frame}"
    );
    assert_eq!(usage_frame["usage"]["total_tokens"], 11);
}

// 流必须以 `data: [DONE]` 收尾；重复 finish 不得重复吐。
#[test]
fn openai_encoder_terminates_with_done_once() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    let frames = enc
        .encode_event(&StreamEvent::MessageStop {
            reason: StopReason::Stop,
        })
        .unwrap();
    assert!(has_done(&frames), "MessageStop 后应有 [DONE]");

    let again = enc.finish().unwrap();
    assert!(
        !has_done(&again),
        "已发过 [DONE] 不该再发，实际: {:?}",
        String::from_utf8_lossy(&again.concat())
    );
}

// 流没收到 MessageStop 就断开时，finish 必须补 [DONE]，否则客户端一直挂着。
#[test]
fn openai_encoder_finish_backfills_done() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    enc.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "partial".into(),
    })
    .unwrap();

    let out = enc.finish().unwrap();
    assert!(has_done(&out), "finish 必须补 [DONE]");
}

// tool 碎片在输出侧要装配成 tool_calls 分片，且 index 保持。
#[test]
fn openai_encoder_emits_tool_call_fragments_with_index() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    let mut frames = Vec::new();
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallStart {
            index: 2,
            id: "call_2".into(),
            name: "search".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 2,
            arguments_delta: "{\"q\":".into(),
        })
        .unwrap(),
    );
    frames.extend(
        enc.encode_event(&StreamEvent::ToolCallDelta {
            index: 2,
            arguments_delta: "\"y\"}".into(),
        })
        .unwrap(),
    );

    let values = frames_to_values(&frames);
    let call_frames: Vec<&Value> = values
        .iter()
        .filter(|v| v["choices"][0]["delta"].get("tool_calls").is_some())
        .collect();

    assert_eq!(call_frames.len(), 3, "start + 两段参数");
    assert_eq!(
        call_frames[0]["choices"][0]["delta"]["tool_calls"][0]["id"],
        "call_2"
    );
    assert_eq!(
        call_frames[0]["choices"][0]["delta"]["tool_calls"][0]["function"]["name"],
        "search"
    );

    let args: String = call_frames[1..]
        .iter()
        .map(|v| {
            v["choices"][0]["delta"]["tool_calls"][0]["function"]["arguments"]
                .as_str()
                .unwrap_or_default()
                .to_string()
        })
        .collect();
    assert_eq!(args, "{\"q\":\"y\"}");

    for v in &call_frames {
        assert_eq!(
            v["choices"][0]["delta"]["tool_calls"][0]["index"], 2,
            "tool index 必须保持为 2，实际: {v}"
        );
    }
}

// 只有碎片、没有 start 时也必须能输出（补一个无 id 的头帧），而不是整条丢掉。
#[test]
fn openai_encoder_emits_tool_delta_without_start() {
    let codec = OpenAiCodec::new();
    let mut enc = codec.stream_encoder();

    let frames = enc
        .encode_event(&StreamEvent::ToolCallDelta {
            index: 0,
            arguments_delta: "{}".into(),
        })
        .unwrap();

    let values = frames_to_values(&frames);
    assert!(
        values
            .iter()
            .any(|v| v["choices"][0]["delta"].get("tool_calls").is_some()),
        "缺 ToolCallStart 也要能输出 tool 分片，实际: {values:?}"
    );
}

#[test]
fn openai_codec_reports_openai_format() {
    assert_eq!(OpenAiCodec::new().format(), Protocol::OpenAi);
}

// 两跳不变量：Chat 体 → IR → Chat 体，核心字段仍在（回归旧中枢路径）。
#[test]
fn openai_two_hop_roundtrip_is_stable() {
    let codec = OpenAiCodec::new();
    let body = serde_json::to_vec(&json!({
        "model": "gpt-x",
        "messages": [{"role": "user", "content": "ping"}],
        "temperature": 0.2
    }))
    .unwrap();

    let ir = codec.decode_request(Bytes::from(body)).expect("decode");
    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();

    assert_eq!(out["model"], "gpt-x");
    assert_eq!(out["messages"][0]["content"], "ping");
    assert_eq!(out["temperature"], 0.2);
    // 未设 stream 时不得凭空注入 stream_options（会让非流式请求也被上游当流式处理）。
    assert!(
        out.get("stream_options").is_none(),
        "非流式请求不该注入 stream_options，实际: {out}"
    );
}

// IR 里没有 system 时不得产出空的 system 消息（会污染下游 schema）。
#[test]
fn openai_encode_request_omits_absent_system() {
    let codec = OpenAiCodec::new();
    let mut ir = mk_request("gpt-x");
    ir.messages = vec![gateway_protocol_bridge::ir::Message {
        role: Role::User,
        content: vec![ContentBlock::Text { text: "hi".into() }],
    }];
    ir.sampling = Some(SamplingParams {
        temperature: None,
        top_p: None,
        max_tokens: None,
        extra: Map::new(),
    });

    let out: Value = serde_json::from_slice(&codec.encode_request(&ir).unwrap()).unwrap();
    let messages = out["messages"].as_array().unwrap();
    assert_eq!(
        messages.len(),
        1,
        "不该凭空多出 system 消息，实际: {messages:?}"
    );
    assert_eq!(messages[0]["role"], "user");
}
