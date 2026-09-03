//! dispatch 健康表 / 分层选择 / failover 预算的行为测试。

use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::Dispatch;
use dispatch::candidate::resolve_candidate;
use dispatch::health::{defaults, FailureClass, HealthTable, MemoryHealthTable};
use dispatch::retry::{Failover, RetryPolicy};
use dispatch::selector::{Selector, WeightedSelector};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

fn unit(
    group: &str,
    model: &str,
    key: &str,
    priority: i32,
    weight: u32,
    status: u8,
) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        group: group.to_string(),
        public_model: model.to_string(),
        channel_key: key.to_string(),
        key_index: 0,
        upstream_model: model.to_string(),
        priority,
        weight,
        status,
    }
}

fn channel(key: &str, secret: &str, base_url: &str) -> ChannelRecord {
    ChannelRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        name: key.to_string(),
        provider_type: "openai".to_string(),
        base_url: base_url.to_string(),
        keys: vec![ChannelKey {
            index: 0,
            secret: secret.to_string(),
            rpm_limit: 0,
        }],
        max_concurrency: 10,
        status: 1,
        groups: vec!["default".to_string()],
        settings: serde_json::Value::Null,
    }
}

fn assert_pick(
    units: &[RouteUnitRecord],
    health: &dyn HealthTable,
    exclude: &[String],
) -> Option<RouteUnitRecord> {
    WeightedSelector.pick(units, health, exclude, 0)
}

// ---------- health ----------

#[test]
fn failure_streak_trips_cooldown_and_recovers() {
    let now = Arc::new(AtomicU64::new(1_000_000));
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    let table = MemoryHealthTable::with_clock(clock);

    // 4 次失败未达阈值 → 仍可选
    for _ in 0..4 {
        table.record("u1", Err(FailureClass::Retryable), 1000);
    }
    assert!(table.is_selectable("u1", now.load(Ordering::Relaxed)));

    // 第 5 次 → 熔断, 冷却 30s 内不可选
    table.record("u1", Err(FailureClass::Retryable), 1000);
    assert!(!table.is_selectable("u1", now.load(Ordering::Relaxed)));

    // 冷却中再失败 → 窗口顺延 (时间停在窗口内)
    let first_expiry = table.get("u1").cooldown_until_ms;
    now.store(first_expiry - 5000, Ordering::Relaxed);
    table.record("u1", Err(FailureClass::Retryable), 1000);
    assert!(table.get("u1").cooldown_until_ms > first_expiry);
    assert!(!table.is_selectable("u1", first_expiry + 5000));

    // 冷却结束 → 恢复可选, 但慢启动折扣尚未回满
    let expiry = table.get("u1").cooldown_until_ms;
    now.store(expiry + 1, Ordering::Relaxed);
    assert!(table.is_selectable("u1", expiry + 1));
    assert!(table.get("u1").slow_start < 1.0);

    // 一次成功 → slow_start 回升一步 (0.5 + 0.25 = 0.75)
    table.record("u1", Ok(200), 800);
    assert_eq!(table.get("u1").slow_start, 0.75);
    // snap: SLOW_START_INITIAL + 1 * STEP
    let _ = defaults::SLOW_START_STEP;
}

#[test]
fn success_updates_ewma_and_clears_streak() {
    let table = MemoryHealthTable::new();
    table.record("u1", Ok(200), 1000);
    table.record("u1", Err(FailureClass::Retryable), 1000);
    table.record("u1", Ok(200), 3000);
    let st = table.get("u1");
    // ewma: 首样本 1000, 二次 alpha 混合 3000
    let expected = defaults::EWMA_ALPHA * 3000.0 + (1.0 - defaults::EWMA_ALPHA) * 1000.0;
    assert!((st.ewma_latency_ms - expected).abs() < 0.001);
    assert_eq!(st.failure_streak, 0);
    assert_eq!(st.samples, 2);
}

#[test]
fn latency_quality_neutral_below_min_samples() {
    let table = MemoryHealthTable::new();
    table.record("u1", Ok(200), 2500); // 1 sample < MIN_SAMPLES
    let st = table.get("u1");
    assert_eq!(dispatch::health::latency_quality(&st), 1.0);
    for _ in 0..4 {
        table.record("u1", Ok(200), 2500);
    }
    let st = table.get("u1");
    let q = dispatch::health::latency_quality(&st);
    assert!(q > 1.0 && q <= defaults::LATENCY_CEIL); // 2500ms < 30s target → 加分
}

// ---------- selector ----------

#[test]
fn higher_priority_tier_wins_even_at_lower_weight() {
    let units = vec![
        unit("g", "m", "low", 10, 100, 1),
        unit("g", "m", "high", 20, 1, 1),
    ];
    let health = MemoryHealthTable::new();
    for _ in 0..20 {
        let picked = assert_pick(&units, &health, &[]).unwrap();
        assert_eq!(
            picked.meta.key, "high",
            "高优先层必须优先于低优先层的任意权重"
        );
    }
}

#[test]
fn weight_zero_stays_selectable_at_lowest_share() {
    let units = vec![
        unit("g", "m", "zero", 10, 0, 1),
        unit("g", "m", "heavy", 10, 10, 1),
    ];
    let health = MemoryHealthTable::new();
    // weight=0 单元仍可能被选中 (routingBaseWeight +1 语义), 只是份额极小
    let mut zero_picks = 0;
    for _ in 0..300 {
        let picked = assert_pick(&units, &health, &[]).unwrap();
        if picked.meta.key == "zero" {
            zero_picks += 1;
        }
    }
    // base 1:11 → P(zero)≈8.3%, 300 抽一个不中都不到 1e-12
    assert!(
        zero_picks > 0,
        "weight=0 候选必须仍可被选中 (routingBaseWeight +1 语义)"
    );
    assert!(
        zero_picks < 150,
        "weight=0 份额应显著低于 weight=10: got {zero_picks}/300"
    );
}

#[test]
fn exclude_and_cooldown_remove_candidates() {
    let units = vec![
        unit("g", "m", "a", 10, 10, 1),
        unit("g", "m", "b", 10, 10, 1),
    ];
    let health = MemoryHealthTable::new();
    // exclude a → 永远选 b
    for _ in 0..5 {
        assert_eq!(
            assert_pick(&units, &health, &["a".to_string()])
                .unwrap()
                .meta
                .key,
            "b"
        );
    }
    // b 冷却 → 无候选
    let now = Arc::new(AtomicU64::new(0));
    let clock2 = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    let health2 = MemoryHealthTable::with_clock(clock2);
    for _ in 0..5 {
        health2.record("a", Err(FailureClass::Retryable), 1000);
        health2.record("b", Err(FailureClass::Retryable), 1000);
    }
    assert!(
        assert_pick(&units, &health2, &[]).is_none(),
        "全冷却 → 整层落空返回 None"
    );
}

#[test]
fn tier_fallthrough_when_top_tier_all_cooled() {
    let now = Arc::new(AtomicU64::new(0));
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    let health = MemoryHealthTable::with_clock(clock);
    let units = vec![
        unit("g", "m", "top", 20, 100, 1),
        unit("g", "m", "low", 10, 100, 1),
    ];
    for _ in 0..5 {
        health.record("top", Err(FailureClass::Retryable), 1000);
    }
    // top 冷却窗口 [0, 30000), now=10000 正处于冷却中
    now.store(10_000, Ordering::Relaxed);
    // top 冷却中 → 落到 low 层 (wildtoken selectWeightedByPriority fallthrough)
    assert_eq!(assert_pick(&units, &health, &[]).unwrap().meta.key, "low");
}

// ---------- retry ----------

#[test]
fn failover_budget_and_tried_set() {
    let mut f = Failover::new(RetryPolicy {
        max_attempts: 3,
        ignore_first_priority_on_retry: false,
    });
    assert_eq!(f.next_attempt(), Some(1));
    assert_eq!(f.next_attempt(), Some(2));
    assert_eq!(f.next_attempt(), Some(3));
    assert_eq!(f.next_attempt(), None, "预算耗尽");
    assert!(f.is_exhausted());

    let mut f = Failover::new(RetryPolicy::default());
    assert_eq!(f.next_attempt(), Some(1));
    f.mark_tried("k1");
    f.mark_tried("k1"); // 去重
    assert_eq!(f.exclude(), &["k1".to_string()]);
}

// ---------- candidate ----------

#[test]
fn resolve_candidate_picks_key_by_index() {
    let u = unit("g", "m", "ch1", 10, 10, 1);
    let ch = channel("ch1", "sk-secret", "https://upstream.example");
    let c = resolve_candidate(&u, &ch).unwrap();
    assert_eq!(c.secret, "sk-secret");
    assert_eq!(c.base_url, "https://upstream.example");
    assert_eq!(c.upstream_model, "m");

    // key_index 越界 → None
    let mut u2 = u.clone();
    u2.key_index = 7;
    assert!(resolve_candidate(&u2, &ch).is_none());
}

// ---------- dispatcher ----------

#[test]
fn dispatcher_select_resolves_full_candidate() {
    let units = vec![unit("g", "m", "ch1", 10, 10, 1)];
    let mut channels: HashMap<String, ChannelRecord> = HashMap::new();
    channels.insert(
        "ch1".to_string(),
        channel("ch1", "sk-secret", "https://upstream.example"),
    );
    let snap = Arc::new(dispatch::Snapshot { units, channels });
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));
    let c = dispatcher.select("g", "m", &[]).unwrap();
    assert_eq!(c.secret, "sk-secret");
    assert_eq!(c.base_url, "https://upstream.example");
}

#[test]
fn dispatcher_fails_closed_before_snapshot() {
    let dispatcher = dispatch::Dispatcher::new(None, Arc::new(MemoryHealthTable::new()));
    assert!(matches!(
        dispatcher.select("g", "m", &[]),
        Err(dispatch::DispatchError::SnapshotNotReady)
    ));
}
