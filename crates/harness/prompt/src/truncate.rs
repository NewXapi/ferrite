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

    // 3. 先做「按 call/result 配对的分组」，再反向按 group 装入 budget。
    //    一组 = [assistant(tool_calls), tool(result)*] —— 每个 tool call 回合
    //    单独成组；孤立 tool_result 单成一组；普通消息单独成组。这样相邻的
    //    `callA/resultA/callB/resultB` 不会被旧实现粘成一个超组。
    let groups = partition_into_groups(&candidates);
    let mut deque: VecDeque<AgentModelMessage> = VecDeque::with_capacity(candidates.len());
    let mut remaining = token_budget;

    for group in groups.iter().rev() {
        let mut group_cost: u32 = 0;
        for &idx in group {
            group_cost = group_cost.saturating_add(count_tokens(candidates[idx].1));
        }
        if group_cost <= remaining {
            for &idx in group.iter().rev() {
                deque.push_front(candidates[idx].1.clone());
            }
            remaining = remaining.saturating_sub(group_cost);
        } else if group.iter().all(|&idx| is_tool_payload(candidates[idx].1)) {
            // ponytail: tool group 装不下，整组跳过；继续看更早的普通消息组。
        } else {
            // ponytail: 非工具单条装不下 → 停止（再前面只会更旧更糟）。
            break;
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

/// 把 `candidates`（已剔除首条 system 的消息列表）按 call/result 配对分组。
/// 返回每个组包含的 `candidates` 下标列表（已按原始顺序）。
fn partition_into_groups(candidates: &[(usize, &AgentModelMessage)]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let n = candidates.len();
    let mut i = 0;
    while i < n {
        let (_, msg) = &candidates[i];
        if is_assistant_tool_call(msg) {
            // 起点：assistant(tool_calls)；group = 这一条 + 紧跟的所有 tool(result)
            let mut group = vec![i];
            let mut j = i + 1;
            while j < n && matches!(candidates[j].1.role, AgentModelRole::Tool) {
                group.push(j);
                j += 1;
            }
            groups.push(group);
            i = j;
        } else if matches!(msg.role, AgentModelRole::Tool) {
            // 孤立 tool_result（无前置 call）→ 单成一组
            groups.push(vec![i]);
            i += 1;
        } else {
            // 普通消息 → 单成一组
            groups.push(vec![i]);
            i += 1;
        }
    }
    groups
}

fn is_tool_payload(msg: &AgentModelMessage) -> bool {
    matches!(msg.role, AgentModelRole::Tool)
        || (matches!(msg.role, AgentModelRole::Assistant) && msg.has_tool_payload())
}

fn is_assistant_tool_call(msg: &AgentModelMessage) -> bool {
    matches!(msg.role, AgentModelRole::Assistant) && msg.has_tool_payload()
}
