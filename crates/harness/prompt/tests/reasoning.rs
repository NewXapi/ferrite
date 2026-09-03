//! `wrap_reasoning` / `inject_reasoning` 集成测试。

use harness_prompt::{
    AgentModelContentPart, AgentModelMessage, AgentModelRole, ReasoningTemplate, inject_reasoning,
    wrap_reasoning,
};

fn user_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::User, text)
}

fn system_text(msg: &AgentModelMessage) -> &str {
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
fn inject_reasoning_appends_each_wrapped_as_system_message() {
    let msgs = vec![user_msg("hi")];
    let reasoning = vec!["first".to_string(), "second".to_string()];
    let t = ReasoningTemplate::default();

    let out = inject_reasoning(&msgs, &reasoning, &t, 0);

    assert_eq!(out.len(), 3);
    // 原消息保持不变
    assert_eq!(out[0].role, AgentModelRole::User);
    assert_eq!(system_text(&out[0]), "hi");
    // 注入的两条都是 system，包装过
    assert_eq!(out[1].role, AgentModelRole::System);
    assert_eq!(system_text(&out[1]), "<think>first</think>");
    assert_eq!(out[2].role, AgentModelRole::System);
    assert_eq!(system_text(&out[2]), "<think>second</think>");
}

#[test]
fn inject_reasoning_does_not_mutate_input_vec() {
    let msgs = vec![user_msg("hi")];
    let original_len = msgs.len();
    let original_first_text = system_text(&msgs[0]).to_string();

    let _ = inject_reasoning(&msgs, &["x".to_string()], &ReasoningTemplate::default(), 0);

    assert_eq!(msgs.len(), original_len);
    assert_eq!(system_text(&msgs[0]), original_first_text);
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
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 0);

    // 原 1 条 + 仅一条保留的 reasoning
    assert_eq!(out.len(), 2);
    assert_eq!(system_text(&out[1]), "<think>keep</think>");
}

#[test]
fn inject_reasoning_truncates_to_max_additions() {
    let msgs = vec![user_msg("hi")];
    let reasoning: Vec<String> = ["r1", "r2", "r3", "r4", "r5"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    // max_additions=2：新→旧，前两条
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 2);
    assert_eq!(out.len(), 3);
    assert_eq!(system_text(&out[1]), "<think>r1</think>");
    assert_eq!(system_text(&out[2]), "<think>r2</think>");

    // max_additions=0 表示全部
    let out_all = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 0);
    assert_eq!(out_all.len(), 6);

    // max_additions 大于实际数量 → 全保留
    let out_big = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 99);
    assert_eq!(out_big.len(), 6);
}

#[test]
fn inject_reasoning_preserves_order_newest_first() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let reasoning = vec!["new".to_string(), "old".to_string()];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 0);

    assert_eq!(system_text(&out[0]), "<think>new</think>");
    assert_eq!(system_text(&out[1]), "<think>old</think>");
}

#[test]
fn inject_reasoning_empty_messages_returns_only_injected() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let out = inject_reasoning(
        &msgs,
        &["only".to_string()],
        &ReasoningTemplate::default(),
        0,
    );

    assert_eq!(out.len(), 1);
    assert_eq!(out[0].role, AgentModelRole::System);
    assert_eq!(system_text(&out[0]), "<think>only</think>");
}

#[test]
fn inject_reasoning_empty_inputs_returns_empty_vec() {
    let msgs: Vec<AgentModelMessage> = vec![];
    let reasoning: Vec<String> = vec![];
    let out = inject_reasoning(&msgs, &reasoning, &ReasoningTemplate::default(), 0);
    assert!(out.is_empty());
}
