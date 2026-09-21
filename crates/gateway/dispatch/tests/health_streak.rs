//! 回归: 交替的 Fatal / Neutral 上报必须让 failure_streak 累积到冷却阈值。
//!
//! bug: apply_outcome 的 Neutral 分支清零 failure_streak, 于是 500 (Fatal) 与
//! 404 (Neutral) 交替上报时连击在 1↔0 之间跳, cooldown_threshold=5 永远达不到,
//! 渠道永不冷却。修复后 Neutral 只保留"不改分、不计请求"的语义, 连击保留。

use dispatch::health::{HealthSetting, HealthTable, MemoryHealthTable};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// 默认配置: cooldown_threshold = 5, base 冷却 10s。
const T0: u64 = 1_000_000;

fn clock_table(now: &Arc<AtomicU64>) -> MemoryHealthTable {
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    MemoryHealthTable::with_config_and_clock(HealthSetting::default(), clock)
}

#[test]
fn alternating_fatal_neutral_trips_cooldown() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);
    let key = "ch-a/0:gpt-4o";

    // 4 轮 500/404 交替: Fatal +1, Neutral 不清零 → 连击应线性爬到 4。
    for round in 1..=4u32 {
        table.record(key, Ok(500));
        assert_eq!(
            table.get(key).failure_streak,
            round,
            "第 {round} 轮: Fatal 必须累加连击"
        );
        table.record(key, Ok(404));
        assert_eq!(
            table.get(key).failure_streak,
            round,
            "第 {round} 轮: Neutral (渠道无责) 不得清零失败连击"
        );
    }

    // 第 5 次 Fatal 达阈值 → 冷却激活 (start_cooldown 自身会清零 failure_streak,
    // 外部可观测的终态是 is_selectable == false)。
    table.record(key, Ok(500));
    assert!(
        !table.is_selectable(key, T0),
        "交替失败累计达阈值 5 → 渠道必须进入冷却"
    );
    let st = table.get(key);
    assert!(st.cooldown_until_ms > T0, "冷却截止时刻必须在将来");
    assert_eq!(st.cooldown_streak, 1, "首次冷却激活");
}

/// Neutral 的其它语义不变: 不计请求、不改 EWMA 分数。
#[test]
fn neutral_still_skips_score_and_request_count() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    // 先攒出可信 EWMA (6 次成功, 超过 min_requests=5)。
    for _ in 0..6 {
        table.record("ch", Ok(200));
    }
    let score_before = table.get("ch").ewma_score;
    let count_before = table.get("ch").request_count;

    table.record("ch", Ok(422));
    let st = table.get("ch");
    assert_eq!(st.request_count, count_before, "Neutral 不计请求");
    assert_eq!(st.ewma_score, score_before, "Neutral 不改 EWMA 分数");
}
