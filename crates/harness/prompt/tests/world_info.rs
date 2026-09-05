//! `world_info` 模块集成测试。
//!
//! 覆盖 WorldInfoEntry 激活、概率控制、预算截断、Before/After/Depth 注入位置、
//! 大小写不敏感匹配、空激活集、入参不变性等场景。
//!
//! 注入契约（与实现一致的约定）：
//! - Before / After 各合并为一条 system 消息（content 按 order 用 `\n` 连接）；
//! - Depth 每条条目按自身 depth 相对「原始消息数」独立插入，depth=0/None/越界
//!   追加到末尾；
//! - After 在消息列表没有开头 System 段时跳过；
//! - 扫描文本为空（无消息）时任何条目都不会激活。

use harness_prompt::{
    AgentModelMessage, AgentModelRole, WorldInfoEntry, WorldInfoPosition,
    compute_world_info_budget, inject_world_info,
};

/// 辅助：token 计数器（每个字符算 1 token）
fn simple_token_count(s: &str) -> u32 {
    s.len() as u32
}

/// 辅助：概率 roll 实现
fn always_true(_prob: u32) -> bool {
    true
}

fn always_false(_prob: u32) -> bool {
    false
}

/// 辅助：构造条目
#[allow(clippy::too_many_arguments)]
fn entry(
    keys: &[&str],
    content: &str,
    position: WorldInfoPosition,
    depth: Option<u32>,
    order: i32,
    probability: u32,
) -> WorldInfoEntry {
    WorldInfoEntry {
        keys: keys.iter().map(|s| s.to_string()).collect(),
        content: content.to_string(),
        position,
        depth,
        order,
        probability,
    }
}

#[test]
fn test_compute_budget_basic() {
    let max = 1000;
    // 百分比 10% -> 100 token
    assert_eq!(compute_world_info_budget(10.0, max, None), 100);
    // 百分比 0% -> 下限 1（预算至少 1，保证单条短内容仍可注入）
    assert_eq!(compute_world_info_budget(0.0, max, None), 1);
    // 百分比 200% -> 2000 token（无封顶时按公式直乘）
    assert_eq!(compute_world_info_budget(200.0, max, None), 2000);
    // cap 封顶：min(30, 500) = 30
    assert_eq!(compute_world_info_budget(50.0, max, Some(30)), 30);
    // cap 低于下限：min(0, 100) = 0，下限抬回 1
    assert_eq!(compute_world_info_budget(10.0, max, Some(0)), 1);
    // 浮点 round：33.333% × 1000 = 333.33 → 333
    assert_eq!(compute_world_info_budget(33.333, max, None), 333);
    // 33.5% × 1000 = 335 → 335
    assert_eq!(compute_world_info_budget(33.5, max, None), 335);
}

#[test]
fn test_activation_case_insensitive() {
    let entries = vec![entry(
        &["foo"],
        "world info content",
        WorldInfoPosition::Before,
        None,
        0,
        100,
    )];
    let messages = vec![AgentModelMessage::text(
        AgentModelRole::User,
        "Hello foo bar",
    )];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // 消息内含 "foo"（大小写不敏感），激活并插入 Before 位置
    assert_eq!(result.len(), 2);
    assert!(result[0].role == AgentModelRole::System);
    assert!(result[0].text_payload().contains("world info"));
}

#[test]
fn test_multiple_keys_any_match() {
    let entries = vec![
        entry(
            &["foo", "bar"],
            "match",
            WorldInfoPosition::Before,
            None,
            0,
            100,
        ),
        entry(
            &["baz"],
            "no match",
            WorldInfoPosition::Before,
            None,
            1,
            100,
        ),
    ];
    let messages = vec![
        AgentModelMessage::text(AgentModelRole::User, "foo"),
        AgentModelMessage::text(AgentModelRole::Assistant, "baz"),
    ];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // 两个条目都激活（任一匹配），同路合并为一条 system 消息插在最前
    assert_eq!(result.len(), 3);
    assert!(result[0].role == AgentModelRole::System);
    let head = result[0].text_payload();
    assert!(head.contains("match"), "order 0 条目应在合并消息内: {head}");
    assert!(
        head.contains("no match"),
        "order 1 条目应在合并消息内: {head}"
    );
}

#[test]
fn test_probability_roll_skips() {
    let entries = vec![entry(
        &["test"],
        "prob skip",
        WorldInfoPosition::Before,
        None,
        0,
        0,
    )];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "test")];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(
        &messages,
        &entries,
        budget,
        simple_token_count,
        always_false,
    );
    // roll 返回 false 应跳过，不产生注入
    assert_eq!(result.len(), 1);
    assert!(result[0].role == AgentModelRole::User);
}

#[test]
fn test_budget_truncation() {
    let entries = vec![
        entry(&["a"], "short", WorldInfoPosition::Before, None, 0, 100),
        entry(
            &["b"],
            "this is a long content that exceeds budget",
            WorldInfoPosition::Before,
            None,
            1,
            100,
        ),
        entry(&["c"], "third", WorldInfoPosition::Before, None, 2, 100),
    ];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "a b c")];
    // 预算 5：第一条 5 token 恰好用满，第二条超预算即停（同 ST，后续不再尝试）
    let result = inject_world_info(&messages, &entries, 5, simple_token_count, always_true);
    assert_eq!(result.len(), 2);
    assert!(result[0].role == AgentModelRole::System);
    assert!(result[0].text_payload().contains("short"));
    assert!(result[1].role == AgentModelRole::User);
}

#[test]
fn test_before_injection() {
    let entries = vec![entry(
        &["trigger"],
        "before info",
        WorldInfoPosition::Before,
        None,
        0,
        100,
    )];
    let messages = vec![
        AgentModelMessage::text(AgentModelRole::User, "trigger"),
        AgentModelMessage::text(AgentModelRole::Assistant, "reply"),
    ];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    assert_eq!(result.len(), 3);
    // 系统消息在最前，原始消息顺序保持
    assert!(result[0].role == AgentModelRole::System);
    assert!(result[0].text_payload().contains("before info"));
    assert!(result[1].role == AgentModelRole::User);
    assert!(result[2].role == AgentModelRole::Assistant);
}

#[test]
fn test_after_injection() {
    let entries = vec![entry(
        &["trigger"],
        "after info",
        WorldInfoPosition::After,
        None,
        0,
        100,
    )];
    let messages = vec![
        AgentModelMessage::text(AgentModelRole::System, "existing system"),
        AgentModelMessage::text(AgentModelRole::User, "trigger"),
        AgentModelMessage::text(AgentModelRole::Assistant, "reply"),
        AgentModelMessage::text(AgentModelRole::System, "another system"),
    ];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // After 路插在开头连续 System 段之后：[System, WI, User, Assistant, System]
    assert_eq!(result.len(), 5);
    assert!(result[0].role == AgentModelRole::System);
    assert!(result[1].role == AgentModelRole::System);
    assert!(result[1].text_payload().contains("after info"));
    assert!(result[2].role == AgentModelRole::User);
    assert_eq!(result[2].text_payload(), "trigger");
    assert!(result[3].role == AgentModelRole::Assistant);
    assert!(result[4].role == AgentModelRole::System);
}

#[test]
fn test_depth_injection() {
    let entries = vec![
        entry(
            &["trigger"],
            "depth 0",
            WorldInfoPosition::Depth,
            Some(0),
            0,
            100,
        ),
        entry(
            &["trigger"],
            "depth 1",
            WorldInfoPosition::Depth,
            Some(1),
            1,
            100,
        ),
        entry(
            &["trigger"],
            "depth 2",
            WorldInfoPosition::Depth,
            Some(2),
            2,
            100,
        ),
    ];
    let messages = vec![
        AgentModelMessage::text(AgentModelRole::User, "trigger"),
        AgentModelMessage::text(AgentModelRole::Assistant, "reply"),
        AgentModelMessage::text(AgentModelRole::System, "system1"),
        AgentModelMessage::text(AgentModelRole::System, "system2"),
        AgentModelMessage::text(AgentModelRole::User, "user2"),
    ];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // 3 条独立插入，均相对原始 5 条消息的位置：
    //   depth2 → 原始 idx 3（system2 前）, depth1 → 原始 idx 4（user2 前）, depth0 → 追加末尾
    // 最终布局：[U, A, S1, d2, S2, d1, U2, d0]
    assert_eq!(result.len(), 8);
    assert!(result[3].role == AgentModelRole::System);
    assert!(result[3].text_payload().contains("depth 2"));
    assert!(result[5].role == AgentModelRole::System);
    assert!(result[5].text_payload().contains("depth 1"));
    assert!(result[7].role == AgentModelRole::System);
    assert!(result[7].text_payload().contains("depth 0"));
    // 原消息相对顺序保持：system2 在 d2 之后、user2 在 d1 之后
    assert_eq!(result[4].text_payload(), "system2");
    assert_eq!(result[6].text_payload(), "user2");
}

#[test]
fn test_depth_clamp() {
    let entries = vec![entry(
        &["trigger"],
        "depth clamp",
        WorldInfoPosition::Depth,
        Some(100),
        0,
        100,
    )];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "trigger")];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // depth 100 越界，视为 0：追加到末尾
    assert_eq!(result.len(), 2);
    assert!(result[1].role == AgentModelRole::System);
    assert!(result[1].text_payload().contains("depth clamp"));
}

#[test]
fn test_empty_activation_returns_original() {
    let entries: Vec<WorldInfoEntry> = vec![];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "hello")];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // 空激活集应返回等价克隆
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].text_payload(), "hello");
    // 确保是深拷贝（新分配的消息），不是同一引用
    assert!(!std::ptr::eq(&result[0], &messages[0]));
}

#[test]
fn test_input_unchanged() {
    let original_messages = vec![AgentModelMessage::text(AgentModelRole::User, "test")];
    let entries = vec![entry(
        &["test"],
        "info",
        WorldInfoPosition::Before,
        None,
        0,
        100,
    )];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(
        &original_messages,
        &entries,
        budget,
        simple_token_count,
        always_true,
    );
    // 验证入参未被修改（切片不持有所有权）
    assert_eq!(original_messages.len(), 1);
    assert_eq!(original_messages[0].text_payload(), "test");
    // 结果包含注入内容
    assert_eq!(result.len(), 2);
}

#[test]
fn test_no_system_messages_after_injection() {
    let entries = vec![entry(
        &["trigger"],
        "after only",
        WorldInfoPosition::After,
        None,
        0,
        100,
    )];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "trigger")];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // After 相对 system 块定义：没有开头 System 段时该路跳过
    assert_eq!(result.len(), 1);
    assert!(result[0].role == AgentModelRole::User);
}

#[test]
fn test_before_injection_no_existing_messages() {
    let entries = vec![entry(
        &["trigger"],
        "before empty",
        WorldInfoPosition::Before,
        None,
        0,
        100,
    )];
    let messages: Vec<AgentModelMessage> = vec![];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // 空消息列表 → 扫描文本为空，key 无法命中，条目不激活 → 无注入
    assert_eq!(result.len(), 0);
}

#[test]
fn test_depth_none_interprets_as_zero() {
    let entries = vec![entry(
        &["trigger"],
        "depth none",
        WorldInfoPosition::Depth,
        None,
        0,
        100,
    )];
    let messages = vec![AgentModelMessage::text(AgentModelRole::User, "trigger")];
    let budget = compute_world_info_budget(50.0, 1000, None);
    let result = inject_world_info(&messages, &entries, budget, simple_token_count, always_true);
    // None 视为 0，追加到末尾
    assert_eq!(result.len(), 2);
    assert!(result[1].role == AgentModelRole::System);
    assert!(result[1].text_payload().contains("depth none"));
}
