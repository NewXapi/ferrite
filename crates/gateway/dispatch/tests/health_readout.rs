//! 健康表 / Dispatcher 只读查询口 (entries / snapshot) 行为测试 — #164。
//!
//! 钉住两点：
//! 1. `MemoryHealthTable::entries()` 在 `record` 前为空；一条 Fatal 达到
//!    cooldown_threshold 后该 unit 出现在 entries 且冷却字段生效
//!    （`cooldown_until_ms > now`，即"remaining_ms > 0"语义）；
//! 2. `Dispatcher::snapshot()` 在 `set_snapshot` 前为 `None`，装载后为 `Some`。
//!
//! 时钟全部用 `with_config_and_clock` 固定，保证 until_ms 差值确定。

use std::sync::Arc;

use dispatch::health::{ChannelOutcome, HealthSetting};
use dispatch::{FailureClass, HealthTable, MemoryHealthTable, Snapshot};

/// 构造一个最小 Snapshot（空 units/channels）用于 snapshot() 存在性测试。
fn empty_snapshot() -> Arc<Snapshot> {
    Arc::new(Snapshot::default())
}

/// entries() 未 record 前为空。
#[test]
fn entries_empty_before_any_record() {
    let table = MemoryHealthTable::new();
    assert!(table.entries().is_empty());
}

/// 连续传输层失败达 cooldown_threshold（默认 5）后，
/// 该 unit 出现在 entries 且处于冷却窗口（remaining_ms > 0）。
#[test]
fn entries_shows_cooling_unit_after_threshold_failures() {
    let now_ms = 1_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now_ms);
    let key = "ch-abc/0:gpt-4o";

    // 默认 threshold = 5；5 次 Retryable（传输层）即触发冷却。
    for _ in 0..5 {
        table.record(key, Err(FailureClass::Retryable));
    }

    let found = table.entries();
    let Some((found_key, st)) = found.iter().find(|(k, _)| k == key) else {
        panic!("cooling unit not present in entries");
    };
    assert_eq!(found_key, key);
    assert!(
        st.is_cooling(now_ms),
        "unit should be inside its cooldown window"
    );
    // 首次激活走 base 档（10s）：until - now = 10_000ms。
    assert_eq!(st.cooldown_until_ms - now_ms, 10_000);
    // 冷却触发 outcome 记为 Fatal（P1-A 分档依据）。
    assert_eq!(st.last_cooling_outcome, Some(ChannelOutcome::Fatal));
}

/// 仅 Neutral outcome（如 404）record 过的 unit 也在 entries 里，
/// 但不进入冷却 — 查询面"有记录"不等于"异常"，过滤在 handler 侧做。
#[test]
fn entries_contains_neutral_units_without_cooling() {
    let now_ms = 500u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now_ms);
    table.record("unit-neutral", Ok(404));

    let entries = table.entries();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].0, "unit-neutral");
    assert!(!entries[0].1.is_cooling(now_ms));
}

/// entries() 是锁内克隆的数据快照：读取后继续 record 不影响已读副本。
#[test]
fn entries_is_a_copy_not_a_live_view() {
    let now_ms = 2_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now_ms);
    let key = "unit-x";
    for _ in 0..5 {
        table.record(key, Err(FailureClass::Retryable));
    }
    let snapshot = table.entries();
    let reqs_before = snapshot
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, st)| st.request_count)
        .unwrap_or(0);

    table.record(key, Err(FailureClass::Retryable));
    let live = table.get(key);
    assert_eq!(
        live.request_count,
        reqs_before + 1,
        "live view should see the new record"
    );
}

/// Dispatcher::snapshot() 在 set_snapshot 前为 None。
#[test]
fn dispatcher_snapshot_none_before_set() {
    let dispatcher = dispatch::Dispatcher::new(None, Arc::new(MemoryHealthTable::new()));
    assert!(dispatcher.snapshot().is_none());
}

/// set_snapshot 后为 Some，且内容可读（空快照 units 为空但句柄有效）。
#[test]
fn dispatcher_snapshot_some_after_set() {
    let dispatcher = dispatch::Dispatcher::new(None, Arc::new(MemoryHealthTable::new()));
    dispatcher.set_snapshot(empty_snapshot());

    let snap = dispatcher
        .snapshot()
        .expect("snapshot should be readable after set_snapshot");
    assert!(snap.units.is_empty());
    assert!(snap.channels.is_empty());
}

// 429 触发的是 base 短冷却，与 5xx streak 曲线区分（P1-A）；
// 顺带钉住 entries 里能区分两种冷却来源。
#[test]
fn throttled_cooldown_is_recorded_in_entries() {
    let now_ms = 7_000_000u64;
    let cfg = HealthSetting {
        // 压低阈值便于触发。
        cooldown_threshold: 2,
        ..HealthSetting::default()
    };
    let table = MemoryHealthTable::with_config_and_clock(cfg, move || now_ms);
    let key = "unit-429";
    table.record(key, Ok(429));
    table.record(key, Ok(429));

    let st = table.get(key);
    assert!(
        st.is_cooling(now_ms),
        "429 x2 (threshold 2) should trigger base cooldown"
    );
    assert_eq!(
        st.last_cooling_outcome,
        Some(ChannelOutcome::Throttled),
        "P1-A: 429 should be tagged Throttled for base-tier cooldown"
    );
    // base 档 = cooldown_base_seconds (10s)，不随 streak 爬向 max。
    assert_eq!(st.cooldown_until_ms - now_ms, 10_000);
    assert!(table.entries().iter().any(|(k, _)| k == key));
}
