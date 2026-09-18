//! IR 模块的行为测试：round-trip 无损、逃逸口、skip_serializing_if、tagged 形状。

use gateway_protocol_bridge::ir::{
    ContentBlock, LlmRequest, Message, Role, SamplingParams, StopReason, StreamEvent, TextBlock,
    ToolChoice, ToolDef,
};
use serde_json::{Map, Value, json};

/// 构造一个覆盖全部 ContentBlock 变体的请求，作为 round-trip 的载荷。
fn full_request() -> LlmRequest {
    LlmRequest {
        model: "gpt-x".into(),
        messages: vec![Message {
            role: Role::Assistant,
            content: vec![
                ContentBlock::Text { text: "hi".into() },
                ContentBlock::ToolUse {
                    id: "tu_1".into(),
                    name: "search".into(),
                    input: json!({"q": "ferrite"}),
                },
                ContentBlock::ToolResult {
                    tool_use_id: "tu_1".into(),
                    content: "result".into(),
                },
                ContentBlock::Thinking { text: "hmm".into() },
                ContentBlock::Refusal { text: "no".into() },
                ContentBlock::Image {
                    media_type: "image/png".into(),
                    data: "AAAB".into(),
                },
                // 未知 block 类型：逃逸口必须原样吞下并吐回
                ContentBlock::Unknown {
                    raw: json!({"type": "brand_new_block", "foo": "bar"}),
                },
            ],
        }],
        system: vec![TextBlock { text: "sys".into() }],
        tools: vec![ToolDef {
            name: "search".into(),
            description: Some("desc".into()),
            input_schema: json!({"type": "object"}),
            extra: Map::new(),
        }],
        tool_choice: Some(ToolChoice::Auto),
        sampling: Some(SamplingParams {
            temperature: Some(0.7),
            top_p: Some(0.9),
            max_tokens: Some(128),
            extra: Map::new(),
        }),
        stream: true,
        extra: Map::new(),
    }
}

// 1. Round-trip 无损：所有 ContentBlock 变体（含 Unknown 逃逸口）序列化→反序列化后必须语义等价
//    （assert_eq 走 Value 相等，键顺序不敏感；Unknown 的键顺序经 BTreeMap 会被规范化，数据不丢即可）。
//    这是 IR 作为中转枢纽的硬契约：任何 codec 经 IR 中转都不能丢信息。
#[test]
fn ir_roundtrip_is_lossless_for_all_content_blocks() {
    let req = full_request();
    let json_str = serde_json::to_string(&req).expect("serialize");
    let back: LlmRequest = serde_json::from_str(&json_str).expect("deserialize");
    assert_eq!(req, back);
}

// 2. 未知顶层字段保留：JSON 里不在 schema 中的键必须进 extra，再序列化时原样出现。
//    若 extra 逃逸口失效，新厂商字段会在第一跳就被静默丢弃，导致响应无法按原格式回写。
#[test]
fn ir_unknown_top_level_fields_survive_via_extra() {
    let raw = r#"{"model":"x","messages":[],"unknown_key":1}"#;
    let req: LlmRequest = serde_json::from_str(raw).expect("deserialize");
    assert_eq!(req.extra.get("unknown_key"), Some(&json!(1)));

    let again = serde_json::to_string(&req).expect("serialize");
    assert!(
        again.contains(r#""unknown_key":1"#),
        "unknown_key must round-trip, got: {again}"
    );
}

// 3. 未知 block 类型进 Unknown：解析端遇到没建过模的 type 时，整个对象落 raw，
//    序列化时原样吐出（不重新打 tag）。audio/file 等仍未建模的多模态块就是经这条路无损通过的。
#[test]
fn ir_unknown_block_type_falls_into_unknown() {
    let raw = r#"{"type":"brand_new_block","foo":"bar"}"#;
    let block: ContentBlock = serde_json::from_str(raw).expect("deserialize");
    match block {
        ContentBlock::Unknown { ref raw } => {
            assert_eq!(raw, &json!({"type": "brand_new_block", "foo": "bar"}));
        }
        other => panic!("expected Unknown, got {other:?}"),
    }

    // 序列化必须把原始对象完整吐回（键顺序无关——serde_json 默认按 BTreeMap 排序键，
    // 语义等价即无损）：type 与 foo 都在，且不被改写成 "unknown"
    let again: Value =
        serde_json::from_str(&serde_json::to_string(&block).expect("serialize")).expect("re-parse");
    assert_eq!(again, json!({"type": "brand_new_block", "foo": "bar"}));
}

// 3b. 已建模的 image block 必须分派到 Image 变体，而不是静默落 Unknown。
//     防的 bug：给 ContentBlock 加了 Image 变体，却忘了在反序列化的已知类型分派列表里
//     加 "image" —— 结果图片仍整体落 raw，编译器不报错、round-trip 也不报错
//     （Unknown 同样无损），但下游 codec 再也无法把它认成图片，跨格式图片转换静默退化。
#[test]
fn ir_image_block_dispatches_to_image_variant() {
    let raw = r#"{"type":"image","media_type":"image/png","data":"AAAB"}"#;
    let block: ContentBlock = serde_json::from_str(raw).expect("deserialize");
    match block {
        ContentBlock::Image {
            ref media_type,
            ref data,
        } => {
            assert_eq!(media_type, "image/png");
            assert_eq!(data, "AAAB");
        }
        other => panic!("expected Image, got {other:?}"),
    }

    // 序列化必须回到固定的 tagged 形状（字段顺序无关，语义等价即可）
    let again: Value =
        serde_json::from_str(&serde_json::to_string(&block).expect("serialize")).expect("re-parse");
    assert_eq!(
        again,
        json!({"type": "image", "media_type": "image/png", "data": "AAAB"})
    );
}

// 4. skip_serializing_if 生效：空集合不应产出噪声键。
//    若空请求带 "messages":[]，下游按格式回写时会插入 schema 里不存在的数组，污染响应。
#[test]
fn ir_empty_collections_are_not_serialized() {
    let req = LlmRequest {
        model: "x".into(),
        messages: vec![],
        system: vec![],
        tools: vec![],
        tool_choice: None,
        sampling: None,
        stream: false,
        extra: Map::new(),
    };
    let j: Value = serde_json::to_value(&req).expect("serialize");
    for key in [
        "messages",
        "system",
        "tools",
        "tool_choice",
        "sampling",
        "temperature",
        "top_p",
        "max_tokens",
    ] {
        assert!(
            j.get(key).is_none(),
            "empty field {key} should be absent: {j}"
        );
    }
}

// 5. StreamEvent tagged 形状：tag = "type" + snake_case，index 字段在位。
//    index 是 G1（tool 碎片装配）修复的关键：codec 按 index 归属分片，形状错会导致整条流错位。
#[test]
fn ir_stream_event_tagged_shape() {
    let delta = StreamEvent::TextDelta {
        index: 1,
        text: "hi".into(),
    };
    assert_eq!(
        serde_json::to_value(&delta).expect("serialize"),
        json!({"type":"text_delta","index":1,"text":"hi"})
    );

    let start = StreamEvent::MessageStart {
        id: "msg_1".into(),
        model: "m".into(),
    };
    assert_eq!(
        serde_json::to_value(&start).expect("serialize"),
        json!({"type":"message_start","id":"msg_1","model":"m"})
    );
}

// 附带：StopReason 的 Other 逃逸口与 snake_case 形状（非 tagged，枚举直接序列化成字符串/对象）。
#[test]
fn ir_stop_reason_snake_case_and_other_escape() {
    assert_eq!(
        serde_json::to_value(StopReason::ContentFilter).expect("serialize"),
        json!("content_filter")
    );
    assert_eq!(
        serde_json::to_value(StopReason::ToolUse).expect("serialize"),
        json!("tool_use")
    );
    // ponytail: Other(String) 在非 tagged 枚举里序列化成 {"other": "..."}，作为未归类原因的兜底
    assert_eq!(
        serde_json::to_value(StopReason::Other("spam".into())).expect("serialize"),
        json!({"other": "spam"})
    );
}

// 附带：Role 非 tagged snake_case，避免 codec 手写字符串映射。
#[test]
fn ir_role_snake_case() {
    assert_eq!(
        serde_json::to_value(Role::Assistant).expect("serialize"),
        json!("assistant")
    );
}
