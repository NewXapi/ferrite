//! 历史消息 token budget 裁剪。
//!
//! ## 算法（沿用 SillyTavern `openai.js populateChatHistory`）
//!
//! 1. **首条 system 消息**：从 budget 中免费保留，永不被裁掉。
//! 2. 从最新到最旧遍历：
//!    - 普通消息：若 `cost ≤ remaining` 则保留，否则 **break**（后续全是更旧消息，
//!      一旦放不下最新可放的，往前只会更糟；继续扫只会浪费 token）。
//!    - 工具消息组：识别一段连续的 tool-call / tool-result 消息段；**整组**要么
//!      全保留，要么全丢；整组放不下时**跳过并继续**（更早的非工具消息仍可能装得下）。
//! 3. 已选中的消息保持原顺序输出。
//!
//! 实现细节：
//! - 用 `VecDeque` + 双端操作避免 `Vec::insert(0, _)` 的 O(n²)；
//!
//! ponytail: 单次扫描 + `VecDeque::push_front` 即可达成 O(n)。

use std::collections::VecDeque;

use crate::types::{AgentModelMessage, AgentModelRole};

/// 裁剪时被丢弃的原因；测试用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationDropReason {
    /// 单条普通消息超过剩余 budget，从此处停止
    BudgetExceeded,
    /// 工具消息组放不下，整组跳过
    ToolGroupDropped,
}

/// 输入：历史消息 + token budget + 单条消息 token 计数闭包。
///
/// `count_tokens`：外部传入的 token 估算器；Rust 端不假设具体 tokenizer。
///
/// 返回：保留的消息（已按原始顺序排好）。
pub fn truncate_history<F>(
    messages: &[AgentModelMessage],
    token_budget: u32,
    count_tokens: F,
) -> Vec<AgentModelMessage>
where
    F: Fn(&AgentModelMessage) -> u32,
{
    if token_budget == 0 || messages.is_empty() {
        return Vec::new();
    }

    // 1. 识别首条 system 消息：从 budget 中免费保留（不计入 cost）。
    let first_system_index = messages.iter().position(|m| m.role.is_system());

    // 2. 构造 candidates：(index, message) 按原始顺序排列；
    //    如果有首条 system，把它从 candidates 中排除（之后单独 prepend）。
    let mut candidates: Vec<(usize, &AgentModelMessage)> = Vec::with_capacity(messages.len());
    if let Some(idx) = first_system_index {
        for (i, m) in messages.iter().enumerate() {
            if i == idx {
                continue;
            }
            candidates.push((i, m));
        }
    } else {
        candidates.extend(messages.iter().enumerate());
    }

    // 3. 反向扫描，按 group 识别工具消息。
    //    group = 一段连续的「tool-call 或 tool-result」消息段。
    let mut deque: VecDeque<AgentModelMessage> = VecDeque::with_capacity(candidates.len());
    let mut remaining = token_budget;

    let n = candidates.len();
    let mut cursor = n; // 下一个要处理的 index（从 n 开始，向 0 推进）
    while cursor > 0 {
        // 看 candidates[cursor - 1]
        cursor -= 1;
        let (_, msg) = &candidates[cursor];

        if is_tool_payload(msg) {
            // 工具 group：从 cursor 往前扩展到 [group_start, cursor+1)
            // 找 group_start：连续 tool payload 的最早位置
            let mut group_start = cursor;
            while group_start > 0 && is_tool_payload(&candidates[group_start - 1].1) {
                group_start -= 1;
            }

            // 整组 cost
            let mut group_cost: u32 = 0;
            for k in group_start..=cursor {
                group_cost = group_cost.saturating_add(count_tokens(candidates[k].1));
            }

            if group_cost <= remaining {
                // 整组保留（按原始顺序 push_front）
                for k in (group_start..=cursor).rev() {
                    deque.push_front(candidates[k].1.clone());
                }
                remaining = remaining.saturating_sub(group_cost);
            } else {
                // ponytail: 工具组放不下，整组跳过；继续看更早的非工具消息。
            }
            // 推进 cursor 到 group_start（while 顶部会再 --i，变成 group_start-1）
            cursor = group_start;
        } else {
            let cost = count_tokens(msg);
            if cost > remaining {
                // 单条超过预算 → break（再前面只会更旧更糟）
                break;
            }
            deque.push_front((*msg).clone());
            remaining = remaining.saturating_sub(cost);
        }
    }

    // 4. 把首条 system 消息放到最前
    let mut out: Vec<AgentModelMessage> = deque.into_iter().collect();
    if let Some(idx) = first_system_index {
        let sys = messages[idx].clone();
        out.insert(0, sys);
    }

    out
}

fn is_tool_payload(msg: &AgentModelMessage) -> bool {
    matches!(msg.role, AgentModelRole::Tool)
        || (matches!(msg.role, AgentModelRole::Assistant) && msg.has_tool_payload())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{AgentModelContentPart, AgentModelRole};
    use serde_json::json;

    fn user(text: &str) -> AgentModelMessage {
        AgentModelMessage::text(AgentModelRole::User, text)
    }

    fn assistant_text(text: &str) -> AgentModelMessage {
        AgentModelMessage::text(AgentModelRole::Assistant, text)
    }

    fn sys(text: &str) -> AgentModelMessage {
        AgentModelMessage::text(AgentModelRole::System, text)
    }

    fn tool_call(call_id: &str, tool_id: &str) -> AgentModelMessage {
        AgentModelMessage::assistant_tool_call(call_id, tool_id, "read_file", json!({"path": "/x"}))
    }

    fn tool_result(call_id: &str, tool_id: &str, content: &str) -> AgentModelMessage {
        AgentModelMessage::tool_result(call_id, tool_id, content)
    }

    /// 用文本字符数当 token 估算（测试用；不反映真实 tokenizer）
    fn len_count(m: &AgentModelMessage) -> u32 {
        m.parts
            .iter()
            .map(|p| match p {
                AgentModelContentPart::Text { text } => text.len() as u32,
                AgentModelContentPart::ToolCall { arguments, .. } => {
                    arguments.to_string().len() as u32
                }
                AgentModelContentPart::ToolResult { content, .. } => content.len() as u32,
            })
            .sum::<u32>()
            .max(1)
    }

    #[test]
    fn empty_input_returns_empty() {
        let out = truncate_history(&[], 100, len_count);
        assert!(out.is_empty());
    }

    #[test]
    fn zero_budget_returns_empty() {
        let msgs = vec![user("hello")];
        let out = truncate_history(&msgs, 0, len_count);
        assert!(out.is_empty());
    }

    #[test]
    fn full_history_within_budget() {
        let msgs = vec![sys("system"), user("hi"), assistant_text("hello")];
        let out = truncate_history(&msgs, 10_000, len_count);
        assert_eq!(out.len(), 3);
        assert_eq!(out[0].role, AgentModelRole::System);
    }

    #[test]
    fn first_system_message_is_always_kept_even_if_over_budget() {
        let msgs = vec![sys(&"x".repeat(200)), user("hi")];
        let out = truncate_history(&msgs, 5, len_count);
        // sys 必须保留；user "hi" 也装得下（2 chars）
        assert!(out[0].role.is_system());
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn drops_oldest_messages_when_budget_exhausted() {
        let msgs = vec![
            sys("sys"),
            user(&"a".repeat(50)),
            user(&"b".repeat(50)),
            user(&"c".repeat(50)),
            user(&"d".repeat(50)),
        ];
        // 预算 60：sys(3) + 1 条最近 user(50) = 53 ≤ 60；第 2 条 50 → 103 > 60 → break
        let out = truncate_history(&msgs, 60, len_count);
        assert_eq!(out.len(), 2);
        assert!(out[0].role.is_system());
        assert_eq!(out[1].text_payload(), "d".repeat(50));
    }

    #[test]
    fn tool_call_group_kept_atomically() {
        let msgs = vec![
            sys("sys"),
            user("ask"),
            tool_call("c1", "builtin:read"),
            tool_result("c1", "builtin:read", "file contents"),
            user("thanks"),
        ];
        let out = truncate_history(&msgs, 10_000, len_count);
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
    fn tool_group_dropped_atomically_when_doesnt_fit() {
        // sys + 极大旧 user + tool group + 新 user
        // 预算只够 sys + 新 user，tool group + 旧 user 都放不下
        let msgs = vec![
            sys("sys"),
            user(&"x".repeat(100)),
            tool_call("c1", "builtin:read"),
            tool_result("c1", "builtin:read", &"y".repeat(50)),
            user("recent"),
        ];
        // reverse 扫：recent(6) keep, group(cost ≈ 13+50=63) > remaining(44) → skip,
        //   cursor 跳到 group_start(=2)；candidates[1](x*100) cost 100 > 44 → break
        // → out = [sys, recent] = 2
        let out = truncate_history(&msgs, 50, len_count);
        assert_eq!(out.len(), 2);
        assert!(out[0].role.is_system());
        assert_eq!(out[1].text_payload(), "recent");
    }

    #[test]
    fn tool_group_kept_when_fits_and_skipped_when_group_cost_exceeds() {
        // 把 tool group 弄得很贵，让它单独超预算；同时验证 fit 时保留
        let msgs = vec![
            sys("sys"),
            user("old1"),
            tool_call("c1", "builtin:read"),
            tool_result("c1", "builtin:read", &"z".repeat(1000)),
            user("recent"),
        ];
        // recent(6) → ok，group(cost ≈ 1000+) > 4 → skip；继续：old1(4) → ok
        let out = truncate_history(&msgs, 50, len_count);
        // 期望：sys + old1 + recent；tool group 被跳过
        let tool_count = out.iter().filter(|m| is_tool_payload(m)).count();
        assert_eq!(tool_count, 0, "tool group must be dropped atomically");
        assert!(out[0].role.is_system());
        assert!(out.iter().any(|m| m.text_payload() == "recent"));
        assert!(out.iter().any(|m| m.text_payload() == "old1"));
    }

    #[test]
    fn single_message_breaking_budget_stops_iteration() {
        let msgs = vec![sys("sys"), user("small"), user(&"x".repeat(200))];
        let out = truncate_history(&msgs, 50, len_count);
        // sys 免费保留；reverse 扫到 x*200(200) > 50 → break（连 small 都不保留，
        // 因为 small 在 x*200 之前；break 在 x*200 处触发）
        assert_eq!(out.len(), 1);
        assert!(out[0].role.is_system());
    }

    #[test]
    fn orphan_tool_result_kept_when_fits() {
        let msgs = vec![
            sys("sys"),
            tool_result("orphan", "builtin:read", "result"),
            user("hi"),
        ];
        let out = truncate_history(&msgs, 10_000, len_count);
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn tool_group_kept_when_affordable_and_older_messages_follow() {
        // group 装得下 → 保留；继续看更早的非工具消息
        let msgs = vec![
            sys("sys"),
            user("older"),
            tool_call("c1", "builtin:read"),
            tool_result("c1", "builtin:read", "data"),
            user("recent"),
        ];
        // group cost ~13+4=17, recent=6, older=6, sys=3 → 总 32 ≤ 50 全保留
        let out = truncate_history(&msgs, 50, len_count);
        assert_eq!(out.len(), 5);
        // tool group 在第 2、3 位
        assert!(matches!(
            out[2].parts[0],
            AgentModelContentPart::ToolCall { .. }
        ));
        assert!(matches!(
            out[3].parts[0],
            AgentModelContentPart::ToolResult { .. }
        ));
    }
}
