//! `wrap_reasoning` / `inject_reasoning` 集成测试。

use harness_prompt::{
    AgentModelContentPart, AgentModelMessage, AgentModelRole, ReasoningTemplate, inject_reasoning,
    truncate_history, wrap_reasoning,
};

fn user_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::User, text)
}

fn message_text(msg: &AgentModelMessage) -> &str {
    match &msg.parts[0] {
        AgentModelContentPart::Text { text } => text,
        _ => panic!("expected text part"),
    }
}

#[test]
fn default_template_is_think_xml() {
    let t = ReasoningTemplate::default();
    assert_eq!(t.prefix, "<think>");
    assert_eq!(t.suffix, "</think>");
    assert_eq!(t.separator, "\n");
    assert_eq!(t, ReasoningTemplate::think_xml());
}

#[test]
fn wrap_reasoning_uses_template() {
    let t = ReasoningTemplate {
        prefix: "<r>".into(),
        suffix: "</r>".into(),
        separator: "".into(),
    };
    assert_eq!(wrap_reasoning("hello", &t), "<r>hello</r>");
}

#[test]
fn wrap_reasoning_preserves_empty_text() {
    // 包装层不做空过滤；过滤由 inject_reasoning 负责。
    let t = ReasoningTemplate::default();
    assert_eq!(wrap_reasoning("", &t), "<think></think>");
}

#[test]
fn inject_reasoning_appends_each_wrapped_as_assistant_message() {
    let msgs = vec![user_msg("hi")];
    let reasoning = vec!["first".to_string(), "second".to_string()];
    let t = ReasoningTemplate::default();

    let out = inject_reasoning(&msgs, &reasoning, &t, None);

    assert_eq!(out.len(), 3);
    // 原消息保持不变
    assert_eq!(out[0].role, AgentModelRole::User);
    assert_eq!(message_text(&out[0]), "hi");
    // 注入的两条都是 system，包装过
    assert_eq!(out[1].role, AgentModelRole::Assistant);
    assert_eq!(message_text(&out[1]), "<think>first</think>");
    assert_eq!(out[2].role, AgentModelRole::Assistant);
    assert_eq!(message_text(&out[2]), "<think>second</think>");
}

#[test]
fn inject_reasoning_does_not_mutate_input_vec() {
    let msgs = vec![user_msg("hi")];
    let original_len = msgs.len();
    let original_first_text = message_text(&msgs[0]).to_string();

    let _ = inject_reasoning(
        &msgs,
        &["x".to_string()],
        &ReasoningTemplate::default(),
        None,
    );

    assert_eq!(msgs.len(), original_len);
    assert_eq!(message_text(&msgs[0]), original_first_text);
}

#[test]
fn inject_reasoning_skips_empty_reasoning() {
    let msgs = vec![user_msg("hi")];
    let reasoning = vec![
        "".to_string(),
        "   ".to_string(),
        "\n\t".to_string(),
        "keep".to_string(),
        "".to_string(),
    ];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), None);

    // 原 1 条 + 仅一条保留的 reasoning
    assert_eq!(out.len(), 2);
    assert_eq!(message_text(&out[1]), "<think>keep</think>");
}

#[test]
fn inject_reasoning_truncates_to_max_additions() {
    let msgs = vec![user_msg("hi")];
    let reasoning: Vec<String> = ["r1", "r2", "r3", "r4", "r5"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    // max_additions=2：新→旧，前两条
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), Some(2));
    assert_eq!(out.len(), 3);
    assert_eq!(message_text(&out[1]), "<think>r1</think>");
    assert_eq!(message_text(&out[2]), "<think>r2</think>");

    // None 表示不限条数
    let out_all = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), None);
    assert_eq!(out_all.len(), 6);

    // Some(n) 大于实际数量 → 全保留
    let out_big = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), Some(99));
    assert_eq!(out_big.len(), 6);
}

#[test]
fn inject_reasoning_preserves_order_newest_first() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let reasoning = vec!["new".to_string(), "old".to_string()];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), None);

    assert_eq!(message_text(&out[0]), "<think>new</think>");
    assert_eq!(message_text(&out[1]), "<think>old</think>");
}

#[test]
fn inject_reasoning_empty_messages_returns_only_injected() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let out = inject_reasoning(
        &msgs,
        &["only".to_string()],
        &ReasoningTemplate::default(),
        None,
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].role, AgentModelRole::Assistant);
    assert_eq!(message_text(&out[0]), "<think>only</think>");
}

#[test]
fn inject_reasoning_empty_inputs_returns_empty_vec() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let reasoning: Vec<String> = vec![];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), None);
    assert!(out.is_empty());
}

#[test]
fn inject_reasoning_zero_limit_injects_nothing() {
    let msgs = vec![user_msg("hi")];
    let reasoning = vec!["r1".to_string(), "r2".to_string()];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), Some(0));
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].role, AgentModelRole::User);
}

#[test]
fn empty_reasoning_does_not_consume_limit() {
    let msgs = vec![user_msg("hi")];
    let reasoning = vec!["".to_string(), "r1".to_string(), "r2".to_string()];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), Some(2));
    assert_eq!(out.len(), 3);
    assert_eq!(message_text(&out[1]), "<think>r1</think>");
    assert_eq!(message_text(&out[2]), "<think>r2</think>");
}

#[test]
fn injected_reasoning_is_not_exempt_from_truncation_budget() {
    // 真实系统提示留在 messages 首位；注入块必须仍受 budget 约束，且不被移到队首。
    let system = AgentModelMessage::text(AgentModelRole::System, "sys");
    let msgs = vec![system, user_msg("hi")];
    let reasoning = vec!["x".repeat(500)];
    let injected = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), None);
    assert_eq!(injected.len(), 3);
    assert_eq!(injected[2].role, AgentModelRole::Assistant);

    let count = |m: &AgentModelMessage| m.text_payload().len() as u32;
    let out = truncate_history(&injected, 20, count);
    assert_eq!(out[0].role, AgentModelRole::System);
    assert!(
        out.iter()
            .all(|m| !message_text(m).contains(&"x".repeat(500))),
        "oversized injected reasoning must be dropped by budget"
    );
}

#[test]
fn injection_keeps_tool_call_and_result_adjacent() {
    let call = AgentModelMessage::assistant_tool_call(
        "c1",
        "builtin:read",
        "read_file",
        serde_json::json!({"path": "/x"}),
    );
    let result = AgentModelMessage::tool_result("c1", "builtin:read", "data");
    let msgs = vec![user_msg("ask"), call, result];
    let out = inject_reasoning(
        &msgs,
        &["r".to_string()],
        &ReasoningTemplate::default(),
        None,
    );

    assert!(matches!(
        out[1].parts[0],
        AgentModelContentPart::ToolCall { .. }
    ));
    assert!(matches!(
        out[2].parts[0],
        AgentModelContentPart::ToolResult { .. }
    ));
    assert_eq!(out[3].role, AgentModelRole::Assistant);
}
