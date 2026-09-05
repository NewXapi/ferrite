//! `truncate_history` 集成测试。
//!
//! ponytail: 单元测试全部下沉到 tests/ —— Cargo 对 tests/ 目录默认只在
//! `cargo test` 编译，写 `#[cfg(test)]` 在这里反而是 boilerplate 错误。

use harness_prompt::{
    AgentModelContentPart, AgentModelMessage, AgentModelRole, DEFAULT_SUMMARY_PREFIX,
    summarize_history, truncate_history, truncate_history_with_dropped,
};
use serde_json::json;

fn user_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::User, text)
}

fn assistant_text_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::Assistant, text)
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

/// 文本字符数当 token 估算（测试用；不反映真实 tokenizer）。
/// ponytail: 与 src 旧 helper 等价，复刻实现以避免依赖私有函数。
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

fn is_tool_payload(m: &AgentModelMessage) -> bool {
    matches!(m.role, AgentModelRole::Tool)
        || (matches!(m.role, AgentModelRole::Assistant)
            && m.parts
                .iter()
                .any(|p| matches!(p, AgentModelContentPart::ToolCall { .. })))
}

// ===== 基础边界 =====

#[test]
fn empty_messages_returns_empty_vec() {
    let out = truncate_history(&[], 100, char_count);
    assert!(out.is_empty());
}

#[test]
fn empty_input_returns_empty() {
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
fn zero_budget_returns_empty() {
    let msgs = vec![user_msg("hello")];
    let out = truncate_history(&msgs, 0, char_count);
    assert!(out.is_empty());
}

#[test]
fn full_history_within_budget() {
    let msgs = vec![
        sys_msg("system"),
        user_msg("hi"),
        assistant_text_msg("hello"),
    ];
    let out = truncate_history(&msgs, 10_000, char_count);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].role, AgentModelRole::System);
}

#[test]
fn first_system_message_is_always_kept_even_if_over_budget() {
    let msgs = vec![sys_msg(&"x".repeat(200)), user_msg("hi")];
    let out = truncate_history(&msgs, 5, char_count);
    // sys 必须保留；user "hi" 也装得下（2 chars）
    assert!(out[0].role.is_system());
    assert_eq!(out.len(), 2);
}

#[test]
fn system_message_survives_zero_budget_for_system() {
    // 即便 budget 极小，system 也要保留
    let msgs = vec![sys_msg(&"x".repeat(500)), user_msg("hi")];
    let out = truncate_history(&msgs, 5, char_count);
    assert!(out[0].role.is_system());
}

// ===== budget 耗尽 / 单条超预算 =====

#[test]
fn drops_oldest_messages_when_budget_exhausted() {
    let msgs = vec![
        sys_msg("sys"),
        user_msg(&"a".repeat(50)),
        user_msg(&"b".repeat(50)),
        user_msg(&"c".repeat(50)),
        user_msg(&"d".repeat(50)),
    ];
    // 预算 60：sys(3) + 1 条最近 user(50) = 53 ≤ 60；第 2 条 50 → 103 > 60 → break
    let out = truncate_history(&msgs, 60, char_count);
    assert_eq!(out.len(), 2);
    assert!(out[0].role.is_system());
    assert_eq!(out[1].text_payload(), "d".repeat(50));
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
fn single_message_breaking_budget_stops_iteration() {
    let msgs = vec![
        sys_msg("sys"),
        user_msg("small"),
        user_msg(&"x".repeat(200)),
    ];
    let out = truncate_history(&msgs, 50, char_count);
    // sys 免费保留；reverse 扫到 x*200(200) > 50 → break（连 small 都不保留，
    // 因为 small 在 x*200 之前；break 在 x*200 处触发）
    assert_eq!(out.len(), 1);
    assert!(out[0].role.is_system());
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

// ===== tool group 原子性 =====

#[test]
fn tool_call_group_kept_atomically() {
    let msgs = vec![
        sys_msg("sys"),
        user_msg("ask"),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", "file contents"),
        user_msg("thanks"),
    ];
    let out = truncate_history(&msgs, 10_000, char_count);
    assert_eq!(out.len(), 5);
    assert!(matches!(
        out[2].parts[0],
        AgentModelContentPart::ToolCall { .. }
    ));
    assert!(matches!(
        out[3].parts[0],
        AgentModelContentPart::ToolResult { .. }
    ));
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
    let has_call = out
        .iter()
        .any(|m| matches!(m.parts[0], AgentModelContentPart::ToolCall { .. }));
    let has_result = out
        .iter()
        .any(|m| matches!(m.parts[0], AgentModelContentPart::ToolResult { .. }));
    assert_eq!(has_call, has_result, "tool call/result must be atomic");
}

#[test]
fn tool_group_dropped_atomically_when_doesnt_fit() {
    // sys + 极大旧 user + tool group + 新 user
    // 预算只够 sys + 新 user，tool group + 旧 user 都放不下
    let msgs = vec![
        sys_msg("sys"),
        user_msg(&"x".repeat(100)),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", &"y".repeat(50)),
        user_msg("recent"),
    ];
    let out = truncate_history(&msgs, 50, char_count);
    assert_eq!(out.len(), 2);
    assert!(out[0].role.is_system());
    assert_eq!(out[1].text_payload(), "recent");
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
    assert!(out.iter().any(|m| m.text_payload() == "recent"));
    assert!(out.iter().any(|m| m.text_payload() == "older"));
}

#[test]
fn tool_group_kept_when_fits_and_skipped_when_group_cost_exceeds() {
    // 把 tool group 弄得很贵，让它单独超预算；同时验证 fit 时保留
    let msgs = vec![
        sys_msg("sys"),
        user_msg("old1"),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", &"z".repeat(1000)),
        user_msg("recent"),
    ];
    let out = truncate_history(&msgs, 50, char_count);
    let tool_count = out.iter().filter(|m| is_tool_payload(m)).count();
    assert_eq!(tool_count, 0, "tool group must be dropped atomically");
    assert!(out[0].role.is_system());
    assert!(out.iter().any(|m| m.text_payload() == "recent"));
    assert!(out.iter().any(|m| m.text_payload() == "old1"));
}

#[test]
fn tool_group_kept_when_affordable_and_older_messages_follow() {
    // group 装得下 → 保留；继续看更早的非工具消息
    let msgs = vec![
        sys_msg("sys"),
        user_msg("older"),
        tool_call_msg("c1", "builtin:read"),
        tool_result_msg("c1", "builtin:read", "data"),
        user_msg("recent"),
    ];
    // group cost ~13+4=17, recent=6, older=6, sys=3 → 总 32 ≤ 50 全保留
    let out = truncate_history(&msgs, 50, char_count);
    assert_eq!(out.len(), 5);
    assert!(matches!(
        out[2].parts[0],
        AgentModelContentPart::ToolCall { .. }
    ));
    assert!(matches!(
        out[3].parts[0],
        AgentModelContentPart::ToolResult { .. }
    ));
}

#[test]
fn orphan_tool_result_kept_when_fits() {
    let msgs = vec![
        sys_msg("sys"),
        tool_result_msg("orphan", "builtin:read", "result"),
        user_msg("hi"),
    ];
    let out = truncate_history(&msgs, 10_000, char_count);
    assert_eq!(out.len(), 3);
}

// ===== 相邻独立 tool 回合（review blocker） =====

#[test]
fn two_adjacent_tool_groups_split_separately() {
    // callA/resultA/callB/resultB 各自成组；预算只够 B 的 group（callB+resultB）。
    // 旧实现会把四个连成一大组，整组丢掉；新实现保留 B 组（最新一组）。
    let msgs = vec![
        sys_msg("sys"),
        user_msg(&"ask".repeat(100)),
        tool_call_msg("a", "builtin:read"),
        tool_result_msg("a", "builtin:read", "AAAA"),
        tool_call_msg("b", "builtin:read"),
        tool_result_msg("b", "builtin:read", "BBBB"),
    ];
    // group A cost: call(arg = {"path":"/x"})≈13 + result("AAAA")=4 → 17
    // group B cost: 13 + 4 = 17
    // user cost = "ask" * 100 = 300
    // 预算 22：只够 sys + group B（17+3=20 ≤ 22）；user 和 group A 都装不下
    let out = truncate_history(&msgs, 22, char_count);
    assert_eq!(out.len(), 3, "sys + groupB only (user too expensive)");
    assert!(out[0].role.is_system());
    let calls: Vec<_> = out
        .iter()
        .filter_map(|m| match &m.parts[0] {
            AgentModelContentPart::ToolCall { call_id, .. } => Some(call_id.clone()),
            _ => None,
        })
        .collect();
    let results: Vec<_> = out
        .iter()
        .filter_map(|m| match &m.parts[0] {
            AgentModelContentPart::ToolResult { call_id, .. } => Some(call_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        calls,
        vec!["b".to_string()],
        "must keep only group B's call"
    );
    assert_eq!(
        results,
        vec!["b".to_string()],
        "must keep only group B's result"
    );
}

#[test]
fn two_tool_groups_drop_only_older_when_budget_only_fits_newer() {
    // 预算足够最新 group + system，但旧 group 装不下 → 保留最新 group，
    // 旧 group 整组丢掉（不能半保留）。
    let msgs = vec![
        sys_msg("sys"),
        tool_call_msg("a", "builtin:read"),
        tool_result_msg("a", "builtin:read", &"X".repeat(200)),
        tool_call_msg("b", "builtin:read"),
        tool_result_msg("b", "builtin:read", "small"),
    ];
    // group A cost ≈ 213, group B cost ≈ 13+5=18
    // 预算 25：sys(3) + B(18) = 21 ≤ 25；A 不装 → 跳
    let out = truncate_history(&msgs, 25, char_count);
    let tool_count = out.iter().filter(|m| is_tool_payload(m)).count();
    assert_eq!(tool_count, 2, "exactly one group kept (call + result)");
    let kept_ids: Vec<_> = out
        .iter()
        .filter_map(|m| match &m.parts[0] {
            AgentModelContentPart::ToolCall { call_id, .. } => Some(call_id.clone()),
            AgentModelContentPart::ToolResult { call_id, .. } => Some(call_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(kept_ids, vec!["b".to_string(), "b".to_string()]);
    assert!(out[0].role.is_system());
}

// ---------------------------------------------------------------------------
// truncate_history_with_dropped / summarize_history（摘要记忆 H3）
// ---------------------------------------------------------------------------

/// dropped 应包含所有被预算丢弃的消息、按原始顺序、且永不含首条 system。
#[test]
fn with_dropped_reports_dropped_in_original_order() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::System, "sys"),
        AgentModelMessage::text(AgentModelRole::User, "old1"), // 4
        AgentModelMessage::text(AgentModelRole::Assistant, "old2"), // 4
        AgentModelMessage::text(AgentModelRole::User, "new"),  // 3
    ];
    // 预算 3：只装得下 "new"；old1/old2 被 drop
    let (kept, dropped) = truncate_history_with_dropped(&msgs, 3, char_count);
    assert_eq!(kept.len(), 2); // sys + new
    let dropped_text: Vec<_> = dropped.iter().map(|m| m.text_payload()).collect();
    assert_eq!(dropped_text, vec!["old1", "old2"]);
}

/// 预算充足 → dropped 为空，kept 与输入等价。
#[test]
fn with_dropped_empty_when_everything_fits() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::User, "a"),
        AgentModelMessage::text(AgentModelRole::Assistant, "b"),
    ];
    let (kept, dropped) = truncate_history_with_dropped(&msgs, 100, char_count);
    assert!(dropped.is_empty());
    assert_eq!(kept.len(), 2);
}

/// 摘要：有丢弃 → summarize 被调用（入参=dropped），摘要 system 注入开头
/// System 段之后；无丢弃 → summarize 不被调用。
#[test]
fn summarize_history_injects_after_system_block() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::System, "sys"),
        AgentModelMessage::text(AgentModelRole::User, "old1"),
        AgentModelMessage::text(AgentModelRole::Assistant, "old2"),
        AgentModelMessage::text(AgentModelRole::User, "new"),
    ];
    let calls = std::cell::RefCell::new(0u32);
    let result = summarize_history(&msgs, 3, char_count, |dropped| {
        *calls.borrow_mut() += 1;
        format!("compressed {} messages", dropped.len())
    });
    assert_eq!(*calls.borrow(), 1);
    // 布局：[sys, summary(sys), new]
    assert_eq!(result.len(), 3);
    assert!(result[1].role.is_system());
    let summary_text = result[1].text_payload();
    assert!(summary_text.contains(DEFAULT_SUMMARY_PREFIX));
    assert!(summary_text.contains("compressed 2 messages"));
    assert_eq!(result[2].text_payload(), "new");
}

/// 无 system 头时摘要置于最前。
#[test]
fn summarize_history_without_system_head_goes_first() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::User, "old"),
        AgentModelMessage::text(AgentModelRole::User, "new"),
    ];
    let result = summarize_history(&msgs, 3, char_count, |d| format!("s{}", d.len()));
    // dropped=[old]（3 字节超剩余 0），布局：[summary(sys), new]
    assert_eq!(result.len(), 2);
    assert!(result[0].role.is_system());
    assert!(result[0].text_payload().contains("s1"));
    assert_eq!(result[1].text_payload(), "new");
}

/// summarize 返回空串 → 不注入（退化为普通截断）。
#[test]
fn summarize_history_empty_summary_skips_injection() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::User, "old"),
        AgentModelMessage::text(AgentModelRole::User, "new"),
    ];
    let result = summarize_history(&msgs, 3, char_count, |_| String::new());
    assert_eq!(result.len(), 1); // kept only（new 装入，old 被丢）
}

/// 无丢弃 → summarize 不被调用、结果等价截断。
#[test]
fn summarize_history_no_drop_never_calls_summarizer() {
    let msgs = vec![AgentModelMessage::text(AgentModelRole::User, "a")];
    let calls = std::cell::RefCell::new(0u32);
    let result = summarize_history(&msgs, 100, char_count, |_| {
        *calls.borrow_mut() += 1;
        "should not happen".to_string()
    });
    assert_eq!(*calls.borrow(), 0);
    assert_eq!(result.len(), 1);
}

/// 零预算：首条 system 仍免费保留（不进 dropped），其余全部计入 dropped。
/// 「system 永不被丢」不变量对零预算同样成立（ocr bug·high 修复的回归测试）。
#[test]
fn zero_budget_keeps_first_system_out_of_dropped() {
    let msgs = vec![
        AgentModelMessage::text(AgentModelRole::System, "sys"),
        AgentModelMessage::text(AgentModelRole::User, "u1"),
        AgentModelMessage::text(AgentModelRole::Assistant, "a1"),
    ];
    let (kept, dropped) = truncate_history_with_dropped(&msgs, 0, char_count);
    assert_eq!(kept.len(), 1);
    assert!(kept[0].role.is_system());
    let dropped_text: Vec<_> = dropped.iter().map(|m| m.text_payload()).collect();
    assert_eq!(dropped_text, vec!["u1", "a1"]);

    // 无 system 头时零预算 = 全部 dropped
    let msgs = vec![AgentModelMessage::text(AgentModelRole::User, "u1")];
    let (kept, dropped) = truncate_history_with_dropped(&msgs, 0, char_count);
    assert!(kept.is_empty());
    assert_eq!(dropped.len(), 1);
}
