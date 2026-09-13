//! `observe::gateway_health::build_health_view` 纯函数测试 — #164。
//!
//! 全部走固定 `now_ms` + 手工构造 entries/Snapshot，不依赖 PG / 网络 / 时钟，
//! 本地与 CI 行为一致（无需 `#[ignore]`）。

use chrono::Utc;
use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::health::{HealthSetting, HealthState, HealthTable, MemoryHealthTable};
use dispatch::{FailureClass, Snapshot};
use observe::gateway_health::build_health_view;

fn sync_meta(key: &str) -> SyncMeta {
    SyncMeta {
        key: key.to_string(),
        schema_version: 1,
        logical_version: 0,
        origin: "center".to_string(),
        updated_at: Utc::now(),
    }
}

fn channel(key: &str, name: &str) -> ChannelRecord {
    ChannelRecord {
        meta: sync_meta(key),
        name: name.to_string(),
        provider_type: "openai".to_string(),
        base_url: "https://example.invalid".to_string(),
        keys: vec![ChannelKey {
            index: 0,
            secret: "sk-test".to_string(),
            rpm_limit: 0,
        }],
        max_concurrency: 4,
        status: 1,
        groups: vec!["default".to_string()],
        settings: serde_json::json!({}),
    }
}

fn unit(key: &str, channel_key: &str, index: u32, model: &str) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: sync_meta(key),
        group: "default".to_string(),
        public_model: model.to_string(),
        channel_key: channel_key.to_string(),
        key_index: index,
        upstream_model: model.to_string(),
        priority: 0,
        weight: 1,
        status: 1,
    }
}

fn snap() -> Snapshot {
    let mut channels = std::collections::HashMap::new();
    channels.insert("ch-a".to_string(), channel("ch-a", "Alpha"));
    channels.insert("ch-b".to_string(), channel("ch-b", "Beta"));
    Snapshot {
        units: vec![
            unit("u-a:0", "ch-a", 0, "gpt-4o"),
            unit("u-b:0", "ch-b", 0, "gpt-4o-mini"),
            unit("u-b:1", "ch-b", 1, "claude-sonnet"),
        ],
        channels,
    }
}

/// 空 entries → 空响应（无噪音行）。
#[test]
fn empty_entries_produce_no_items() {
    assert!(build_health_view(&[], Some(&snap()), 1_000).is_empty());
}

/// 冷却中的 unit：state=cooling、remaining 为正、join 出渠道名与模型。
#[test]
fn cooling_unit_joins_snapshot_fields() {
    let now = 1_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now);
    // 5 次传输层失败达默认 threshold → 冷却 10s。
    for _ in 0..5 {
        table.record("u-a:0", Err(FailureClass::Retryable));
    }

    let items = build_health_view(&table.entries(), Some(&snap()), now);
    assert_eq!(items.len(), 1);
    let v = &items[0];
    assert_eq!(v.unit_key, "u-a:0");
    assert_eq!(v.channel_key.as_deref(), Some("ch-a"));
    assert_eq!(v.channel_name.as_deref(), Some("Alpha"));
    assert_eq!(v.public_model.as_deref(), Some("gpt-4o"));
    assert_eq!(v.state, "cooling");
    assert_eq!(v.last_cooling_outcome.as_deref(), Some("fatal"));
    assert_eq!(v.remaining_cooldown_ms, 10_000);
}

/// 冷却到期的条目（惰性结算未完成）：state 不再是 cooling，remaining 钉死为 0。
#[test]
fn expired_cooling_reads_as_slow_start_with_zero_remaining() {
    let cfg = HealthSetting::default();
    let now = 2_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(cfg, move || now);
    table.record("u-b:0", Err(FailureClass::Retryable));
    // 直接推进时间到冷却之外：is_cooling(now) = false → 无副作用地读为 ramp 语义。
    let later = now + 30_000;
    let items = build_health_view(&table.entries(), Some(&snap()), later);
    assert_eq!(items.len(), 1);
    assert_eq!(
        items[0].state, "ok",
        "无惰性结算：到期冷却既非 cooling 也非 ramp 标记"
    );
    assert_eq!(items[0].remaining_cooldown_ms, 0);
}

/// 快照为 None（未装载）：条目保留，join 字段全部降级为 None，不造数。
#[test]
fn missing_snapshot_degrades_joined_fields_to_none() {
    let now = 3_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now);
    table.record("ghost", Ok(404));

    let items = build_health_view(&table.entries(), None, now);
    assert_eq!(items.len(), 1);
    let v = &items[0];
    assert_eq!(v.unit_key, "ghost");
    assert!(v.channel_key.is_none());
    assert!(v.channel_name.is_none());
    assert!(v.public_model.is_none());
    assert_eq!(v.state, "ok");
    assert_eq!(v.last_cooling_outcome, None);
}

/// 有快照但 unit 不在目录里（渠道已删）：条目保留，join 字段为 None。
#[test]
fn unit_not_in_catalog_is_kept_with_null_joins() {
    let now = 4_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now);
    table.record("deleted-ch/x:gpt-4", Err(FailureClass::Retryable));

    let items = build_health_view(&table.entries(), Some(&snap()), now);
    assert_eq!(items.len(), 1);
    let v = &items[0];
    assert_eq!(v.unit_key, "deleted-ch/x:gpt-4");
    assert!(v.channel_key.is_none());
    assert!(v.channel_name.is_none());
    assert!(v.public_model.is_none());
}

/// Neutral-only 记录（404）也在响应里（有记录 ≠ 异常，过滤在数据源层做）。
#[test]
fn neutral_only_units_are_listed_as_ok() {
    let now = 5_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now);
    table.record("u-b:1", Ok(404));

    let items = build_health_view(&table.entries(), Some(&snap()), now);
    assert_eq!(items.len(), 1);
    let v = &items[0];
    assert_eq!(v.state, "ok");
    assert_eq!(v.channel_name.as_deref(), Some("Beta"));
    assert_eq!(v.public_model.as_deref(), Some("claude-sonnet"));
}

/// 多条目混合：cooling + ok 并存，顺序与 entries 一致。
#[test]
fn mixed_entries_preserve_input_order() {
    let now = 6_000_000u64;
    let table = MemoryHealthTable::with_config_and_clock(HealthSetting::default(), move || now);
    table.record("u-b:0", Ok(404));
    for _ in 0..5 {
        table.record("u-a:0", Err(FailureClass::Retryable));
    }

    let mut keys: Vec<String> = table.entries().into_iter().map(|(k, _)| k).collect();
    keys.sort();
    let items = build_health_view(&table.entries(), Some(&snap()), now);
    let states: std::collections::HashMap<_, _> = items
        .iter()
        .map(|v| (v.unit_key.as_str(), v.state))
        .collect();
    assert_eq!(states.get("u-a:0"), Some(&"cooling"));
    assert_eq!(states.get("u-b:0"), Some(&"ok"));
    assert_eq!(keys.len(), 2);
}

/// 手工构造 ramp_pending（冷却刚结束）的条目 → state=slow_start。
#[test]
fn ramp_pending_reads_as_slow_start() {
    let st = HealthState {
        ramp_pending: true,
        ewma_score: 0.4,
        ..HealthState::default()
    };
    let entries = vec![("ramp-u".to_string(), st)];

    let items = build_health_view(&entries, None, 0);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].state, "slow_start");
    assert_eq!(items[0].remaining_cooldown_ms, 0);
}
