//! `truncate_history` 集成测试。
//!
//! 覆盖：空输入、budget 边界、system 永保留、tool-call/result 原子性。

use harness_prompt::{AgentModelContentPart, AgentModelMessage, AgentModelRole, truncate_history};
use serde_json::json;

fn user_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::User, text)
}

fn sys_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::System, text)
}

fn tool_call_msg(call_id: &str, tool_id: &str) -> AgentModelMessage {
    AgentModelMessage::assistant_tool_call(call_id, tool_id, "read_file", json!({"path": "/x"}))
}

fn tool_result_msg(call_id: &str, tool_id: &str, content: &str) -> AgentModelMessage {
    AgentModelMessage::tool_result(call_id, tool_id, content)
}

/// 文本字符数当 token 估算。
fn char_count(m: &AgentModelMessage) -> u32 {
    m.parts
        .iter()
        .map(|p| match p {
            AgentModelContentPart::Text { text } => text.len() as u32,
            AgentModelContentPart::ToolCall { arguments, .. } => arguments.to_string().len() as u32,
            AgentModelContentPart::ToolResult { content, .. } => content.len() as u32,
        })
        .sum::<u32>()
        .max(1)
}

#[test]
fn empty_messages_returns_empty_vec() {
    let out = truncate_history(&[], 100, char_count);
    assert!(out.is_empty());
}

#[test]
fn zero_budget_returns_empty_vec() {
    let msgs = vec![user_msg("hello")];
    let out = truncate_history(&msgs, 0, char_count);
    assert!(out.is_empty());
}

#[test]
fn system_message_survives_zero_budget_for_system() {
    // 即便 budget 极小，system 也要保留
    let msgs = vec![sys_msg(&"x".repeat(500)), user_msg("hi")];
    let out = truncate_history(&msgs, 5, char_count);
    assert!(out[0].role.is_system());
}

#[test]
fn budget_exhaustion_drops_oldest_messages_in_reverse() {
    let msgs = vec![
        sys_msg("sys"),
        user_msg(&"a".repeat(50)),
        user_msg(&"b".repeat(50)),
        user_msg(&"c".repeat(50)),
        user_msg(&"d".repeat(50)),
    ];
    let out = truncate_history(&msgs, 60, char_count);
    // 期望：sys + 1 条最近（"d"*50）；再前面会超预算 break
    assert_eq!(out.len(), 2);
    assert!(out[0].role.is_system());
    assert_eq!(out[1].text_payload(), "d".repeat(50));
}

#[test]
fn tool_call_and_result_dropped_atomically() {
    let msgs = vec![
        sys_msg("sys"),
        user_msg("hi"),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", "result content here"),
        user_msg("thanks"),
    ];
    let out = truncate_history(&msgs, 10_000, char_count);
    assert_eq!(out.len(), 5);
    // 必须 call + result 同时保留或同时丢弃
    let has_call = out
        .iter()
        .any(|m| matches!(m.parts[0], AgentModelContentPart::ToolCall { .. }));
    let has_result = out
        .iter()
        .any(|m| matches!(m.parts[0], AgentModelContentPart::ToolResult { .. }));
    assert_eq!(has_call, has_result, "tool call/result must be atomic");
}

#[test]
fn tool_group_skipped_atomically_when_over_budget() {
    // 让 tool group 装不下；老 user 应该被独立保留
    let msgs = vec![
        sys_msg("sys"),
        user_msg("older"),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", &"z".repeat(500)),
        user_msg("recent"),
    ];
    let out = truncate_history(&msgs, 100, char_count);
    let tool_count = out
        .iter()
        .filter(|m| {
            matches!(m.parts[0], AgentModelContentPart::ToolCall { .. })
                || matches!(m.parts[0], AgentModelContentPart::ToolResult { .. })
        })
        .count();
    assert_eq!(tool_count, 0, "tool group must be dropped atomically");
    // 老 user 应该被保留（它比 group 老、cost 小）
    assert!(out.iter().any(|m| m.text_payload() == "recent"));
    assert!(out.iter().any(|m| m.text_payload() == "older"));
}

#[test]
fn long_history_caps_to_recent_messages() {
    // 100 条 user，每条 10 字符；budget 装得下 ~10 条
    let mut msgs = vec![sys_msg("sys")];
    for i in 0..100 {
        msgs.push(user_msg(&format!("{:0>10}", i)));
    }
    let out = truncate_history(&msgs, 100, char_count);
    // 期望装得下最近的几条（具体数字取决于 cost 函数）
    assert!(out.len() > 1);
    assert!(out.len() < 50);
    assert!(out[0].role.is_system());
}
