//! # World Info 激活与三路注入
//!
//! 纯函数模块：按 key 扫描激活 World Info 条目，按预算控制注入量，
//! 以 Before / After / Depth 三路插入 system 消息。数据由调用方以
//! `Vec<WorldInfoEntry>` 传入，本模块不做文件 IO（世界书读取归 tavern-api）。
//!
//! 对齐 SillyTavern `world-info.js checkWorldInfo` 的核心语义：
//! - 预算 = `round(world_info_budget * maxContext / 100)`，受 budget_cap 封顶；
//! - key 大小写不敏感子串匹配，任一 key 命中即激活；
//! - 激活条目按 `(order, 原始下标)` 稳定排序，预算耗尽即停止（同 ST）。
//!
//! 边界（二期）：递归扫描（scan_state 状态机）与 timed effects
//! （sticky / cooldown / delay）不在本模块。

use crate::types::{AgentModelMessage, AgentModelRole};
use serde::{Deserialize, Serialize};

/// 消息中的世界信息插入位置。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorldInfoPosition {
    /// 在当前消息列表的最前插入系统消息。
    Before,
    /// 插入系统消息，排在开头连续 System 段之后；消息列表没有开头 System
    /// 段时该路跳过（After 相对 system 块定义）。
    After,
    /// 每条条目按自身 depth 从消息尾部往回数独立插入（对齐 ST
    /// WIDepthEntries 逐条注入）；`depth=0` / `None` / 越界均视为追加到末尾。
    Depth,
}

/// 世界信息条目；包含激活规则、内容、插入策略与概率控制。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WorldInfoEntry {
    /// 激活键列表。当前会话中任意消息的文本部分（忽略大小写）中存在 key 子串即触发激活。
    /// 多 key 任一匹配即视为激活。
    pub keys: Vec<String>,
    /// 条目的内容文本。
    pub content: String,
    /// 插入位置策略。
    pub position: WorldInfoPosition,
    /// 深度（仅 `Depth` 位置使用）：从消息尾部往回数的消息数；`None` 视为 0。
    pub depth: Option<u32>,
    /// 排序顺序，越小优先级越高；相同顺序时保持输入原始下标以确保稳定。
    pub order: i32,
    /// 激活概率（0-100），由 `probability_roll` 闭包判定。
    pub probability: u32,
}

/// 根据百分比、最大上下文长度与可选上限，计算世界信息 token 预算。
///
/// 公式对齐 ST `world-info.js:4597`：`round(percent * max_context / 100)`，
/// `budget_cap` 先封顶（budget_cap 绝对值上限语义），最后保证下限 1。
pub fn compute_world_info_budget(percent: f64, max_context: u32, budget_cap: Option<u32>) -> u32 {
    let budget = (percent * max_context as f64 / 100.0).round() as u32;
    let capped = budget_cap.map_or(budget, |cap| cap.min(budget));
    capped.max(1)
}

/// 世界信息注入核心函数。
///
/// # 参数
/// - `messages` — 当前对话历史消息切片。
/// - `entries` — 待激活的 `WorldInfoEntry` 条目切片。
/// - `budget` — token 预算，由 [`compute_world_info_budget`] 计算。
/// - `count_tokens` — 单条内容的 token 计数闭包（Rust 端不假设具体 tokenizer）。
/// - `probability_roll` — 概率判定闭包，入参为条目 `probability`（0-100），
///   返回 `false` 则跳过该条目。
///
/// # 返回
/// 新的 `Vec<AgentModelMessage>`；入参不被修改。激活条目按 `(order, 原始下标)`
/// 稳定排序，逐条累加 `count_tokens(content)`，超预算即停止（后续条目不再尝试）。
/// Before / After 各合并为一条 system 消息；Depth 每条按自身 depth 独立插入。
/// 某路无激活条目则不产生该路消息。
pub fn inject_world_info(
    messages: &[AgentModelMessage],
    entries: &[WorldInfoEntry],
    budget: u32,
    count_tokens: impl Fn(&str) -> u32,
    probability_roll: impl Fn(u32) -> bool,
) -> Vec<AgentModelMessage> {
    // 1. 扫描文本：所有消息的 text part 拼接（大小写不敏感子串匹配）。
    let mut scanned = String::new();
    for msg in messages {
        let text = msg.text_payload();
        if !text.is_empty() {
            scanned.push_str(&text);
        }
    }
    let scanned_lower = scanned.to_lowercase();

    // 2. 激活：key 任一命中 + probability 判定通过。
    let mut activated: Vec<(usize, &WorldInfoEntry)> = Vec::new();
    for (idx, entry) in entries.iter().enumerate() {
        let hit = entry
            .keys
            .iter()
            .any(|k| scanned_lower.contains(&k.to_lowercase()));
        if !hit {
            continue;
        }
        if !probability_roll(entry.probability) {
            continue;
        }
        activated.push((idx, entry));
    }

    // 3. 稳定排序：order 升序，同 order 保持原始下标。
    activated.sort_by(|a, b| a.1.order.cmp(&b.1.order).then_with(|| a.0.cmp(&b.0)));

    // 4. 预算内收集注入内容；超预算即停（同 ST：预算耗尽不继续扫）。
    let mut before_msgs = Vec::new();
    let mut after_msgs = Vec::new();
    let mut depth_msgs: Vec<(&WorldInfoEntry, AgentModelMessage)> = Vec::new();
    let mut used = 0u32;
    for (_, entry) in &activated {
        let cost = count_tokens(&entry.content);
        if used + cost > budget {
            break;
        }
        used += cost;
        match entry.position {
            WorldInfoPosition::Before => before_msgs.push(make_system_msg(&entry.content)),
            WorldInfoPosition::After => after_msgs.push(make_system_msg(&entry.content)),
            WorldInfoPosition::Depth => depth_msgs.push((entry, make_system_msg(&entry.content))),
        }
    }

    // 5. 构建结果；Before / After 合并为一条，Depth 逐条独立插入。
    let mut result = messages.to_vec();

    // Before：插到最前。
    if !before_msgs.is_empty() {
        let system_msg = merge_system_messages(before_msgs);
        result.splice(0..0, [system_msg]);
    }

    // After：插在开头连续 System 段之后；没有开头 System 段则跳过。
    if !after_msgs.is_empty() {
        let after_pos = find_first_non_system(&result);
        if after_pos > 0 {
            let system_msg = merge_system_messages(after_msgs);
            result.splice(after_pos..after_pos, [system_msg]);
        }
    }

    // Depth：每条按自身 depth 相对「原始消息数」插入；depth=0 / None / 越界
    // 追加到末尾。从最右位置往左插避免位移；同一位置内按 order 降序插入，
    // 最终左→右为 order 升序。
    if !depth_msgs.is_empty() {
        let orig_len = result.len() as u32;
        let mut placements: Vec<(usize, i32, AgentModelMessage)> = depth_msgs
            .into_iter()
            .map(|(entry, msg)| {
                let raw = entry.depth.unwrap_or(0);
                let eff = if raw >= orig_len { 0 } else { raw };
                (orig_len as usize - eff as usize, entry.order, msg)
            })
            .collect();
        placements.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
        for (idx, _, msg) in placements {
            result.splice(idx..idx, [msg]);
        }
    }

    result
}

/// 合并多条系统消息为一条，content 用 `\n` 分隔。
fn merge_system_messages(messages: Vec<AgentModelMessage>) -> AgentModelMessage {
    if messages.is_empty() {
        panic!("不应传入空消息列表");
    }
    let mut contents = Vec::new();
    for msg in messages {
        // 确保是系统消息
        debug_assert!(msg.role == AgentModelRole::System);
        let text = msg.text_payload();
        if !text.is_empty() {
            contents.push(text);
        }
    }
    AgentModelMessage::text(AgentModelRole::System, contents.join("\n"))
}

/// 找到第一个非系统消息的位置（用于 After 路插入）。
fn find_first_non_system(messages: &[AgentModelMessage]) -> usize {
    messages
        .iter()
        .position(|m| m.role != AgentModelRole::System)
        .unwrap_or(messages.len())
}

/// 创建系统消息，复用现有的 `AgentModelMessage::text` 构造函数。
fn make_system_msg(text: &str) -> AgentModelMessage {
    AgentModelMessage::text(AgentModelRole::System, text)
}
