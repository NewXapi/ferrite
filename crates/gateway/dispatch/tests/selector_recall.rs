//! 池枯竭紧急召回: 所有可参与层都无正权重候选 (典型: 全部冷却) 时,
//! pick 召回剩余冷却最短的渠道, 而不是直接让上层报 503 NoCandidate。

use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::STATUS_ENABLED;
use dispatch::health::{HealthSetting, HealthTable, MemoryHealthTable};
use dispatch::selector::{Selector, WeightedSelector};
use rand::SeedableRng;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const T0: u64 = 1_000_000;

fn clock_table(now: &Arc<AtomicU64>) -> MemoryHealthTable {
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    MemoryHealthTable::with_config_and_clock(HealthSetting::default(), clock)
}

fn unit(key: &str, priority: i32, weight: u32, status: u8) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        group: "default".to_string(),
        public_model: "gpt-4o".to_string(),
        channel_key: key.to_string(),
        key_index: 0,
        upstream_model: "gpt-4o".to_string(),
        priority,
        weight,
        status,
    }
}

fn pick<'a>(
    units: &[&'a RouteUnitRecord],
    health: &dyn HealthTable,
    exclude: &[String],
    now_ms: u64,
) -> Option<&'a RouteUnitRecord> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    WeightedSelector.pick(units, health, exclude, now_ms, &mut rng)
}

/// 两渠道都在冷却: a 剩余 5s, b 剩余 10s → 召回 a, 且 a 恢复可选。
#[test]
fn exhausted_pool_recalls_shortest_cooldown() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    // a: 5×429 → Throttled, base 10s 冷却, 截止 T0+10s。
    for _ in 0..5 {
        table.record("a", Ok(429));
    }
    // b: 先一轮 base 冷却 (截止 T0+10s), 冷却中段再攒 5×429 → 第二轮仍 base,
    // 截止 T0+15s。
    for _ in 0..5 {
        table.record("b", Ok(429));
    }
    now.store(T0 + 5_000, Ordering::Relaxed);
    for _ in 0..5 {
        table.record("b", Ok(429));
    }

    let now_ms = now.load(Ordering::Relaxed);
    assert_eq!(table.cooling_remaining_ms("a", now_ms), Some(5_000));
    assert_eq!(table.cooling_remaining_ms("b", now_ms), Some(10_000));

    let units = [
        unit("a", 10, 100, STATUS_ENABLED),
        unit("b", 10, 100, STATUS_ENABLED),
    ];
    let refs: Vec<&RouteUnitRecord> = units.iter().collect();
    let picked = pick(&refs, &table, &[], now_ms).expect("池枯竭应召回最短冷却渠道");
    assert_eq!(picked.meta.key, "a", "应召回剩余冷却最短的 a");

    // 召回 = 冷却此刻结束 (ramp 武装), 但连击保留。
    assert!(table.is_selectable("a", now_ms), "召回后 a 应可选");
    assert!(!table.is_selectable("b", now_ms), "b 仍在冷却, 不被召回");
    let st = table.get("a");
    assert_eq!(st.cooldown_until_ms, 0, "召回清空冷却截止时刻");
    assert_eq!(st.cooldown_streak, 1, "召回不是宽恕: 冷却连击保留");
    assert!(st.ramp_pending, "召回应武装 slow-start ramp");
}

/// 启用渠道全部在同请求的 exclude 集里 → 真无候选 → None。
#[test]
fn all_excluded_pool_returns_none() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);
    for key in ["a", "b"] {
        for _ in 0..5 {
            table.record(key, Ok(429));
        }
    }

    let units = [
        unit("a", 10, 100, STATUS_ENABLED),
        unit("b", 10, 100, STATUS_ENABLED),
    ];
    let refs: Vec<&RouteUnitRecord> = units.iter().collect();
    let exclude = ["a".to_string(), "b".to_string()];
    assert!(
        pick(&refs, &table, &exclude, T0).is_none(),
        "启用渠道全在 exclude 集 → 返回 None"
    );
}

/// 禁用渠道不参与召回: 池里只剩冷却中的禁用渠道 → None。
#[test]
fn disabled_units_are_not_recalled() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);
    for _ in 0..5 {
        table.record("a", Ok(429));
    }

    let units = [unit("a", 10, 100, 0)];
    let refs: Vec<&RouteUnitRecord> = units.iter().collect();
    assert!(pick(&refs, &table, &[], T0).is_none(), "禁用渠道不得被召回");
    assert!(
        table.cooling_remaining_ms("a", T0).is_some(),
        "召回未发生: a 仍在冷却"
    );
}
