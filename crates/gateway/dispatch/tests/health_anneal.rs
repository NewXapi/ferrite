//! 冷却到期后的空闲退火: cooldown_streak 按 base 窗口 (10s) 线性衰减,
//! 活跃成功上报重置退火起点。全部经 MemoryHealthTable 公开 API + 注入时钟。

use dispatch::health::{FailureClass, HealthSetting, HealthTable, MemoryHealthTable};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// 默认配置: cooldown_base_seconds = 10 → 退火窗口 10s。
const T0: u64 = 1_000_000;
const WINDOW_MS: u64 = 10_000;

fn clock_table(now: &Arc<AtomicU64>) -> MemoryHealthTable {
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    MemoryHealthTable::with_config_and_clock(HealthSetting::default(), clock)
}

/// 攒两轮冷却 (cooldown_streak 1 → 2), 返回第二轮冷却的到期时刻 + 1。
/// 调用方推进时钟到该时刻并触发惰性结算后, 退火锚点即被设好。
fn wind_to_second_cooldown(now: &Arc<AtomicU64>, table: &MemoryHealthTable, key: &str) -> u64 {
    // 第一轮: 5 次传输层失败 → streak=1, base 10s 冷却。
    for _ in 0..5 {
        table.record(key, Err(FailureClass::Retryable));
    }
    let expiry1 = table.get(key).cooldown_until_ms;
    now.store(expiry1 + 1, Ordering::Relaxed);
    // 第二轮: 过期后重新攒 5 次 → streak=2 (曲线第二档)。
    for _ in 0..5 {
        table.record(key, Err(FailureClass::Retryable));
    }
    table.get(key).cooldown_until_ms + 1
}

#[test]
fn idle_after_cooldown_anneals_streak_to_zero() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);
    let key = "ch-a/0:gpt-4o";

    let anchor = wind_to_second_cooldown(&now, &table, key);

    // 冷却到期: is_selectable 惰性结算 → finish_cooldown 设退火锚点, 连击保留。
    now.store(anchor, Ordering::Relaxed);
    assert!(table.is_selectable(key, anchor));
    assert_eq!(table.get(key).cooldown_streak, 2, "退火前连击保留");
    assert_eq!(
        table.get(key).anneal_since_ms,
        anchor,
        "锚点 = 冷却结束时刻"
    );

    // 半个窗口: 不衰减 (只按完整窗口计)。
    let half = anchor + WINDOW_MS / 2;
    now.store(half, Ordering::Relaxed);
    assert!(table.is_selectable(key, half));
    assert_eq!(table.get(key).cooldown_streak, 2, "未满一个完整窗口不衰减");

    // 再推进到 2 个窗口: 2 → 0。
    let full = anchor + 2 * WINDOW_MS;
    now.store(full, Ordering::Relaxed);
    assert!(table.is_selectable(key, full));
    assert_eq!(
        table.get(key).cooldown_streak,
        0,
        "推进 2 个 base 窗口后连击应衰减到 0"
    );
}

#[test]
fn success_resets_anneal_anchor() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);
    let key = "ch-a/0:gpt-4o";

    let anchor = wind_to_second_cooldown(&now, &table, key);
    now.store(anchor, Ordering::Relaxed);
    assert!(table.is_selectable(key, anchor));

    // 推进 1 个窗口后一次成功上报: Success 分支连击 2 → 1, 锚点清零。
    now.store(anchor + WINDOW_MS, Ordering::Relaxed);
    table.record(key, Ok(200));
    let st = table.get(key);
    assert_eq!(st.cooldown_streak, 1, "成功递减冷却连击");
    assert_eq!(st.anneal_since_ms, 0, "活跃事件重置退火起点");

    // 锚点为 0 后退火休眠: 再推进 2 个窗口连击仍停在 1
    // (活跃流量本身会继续递减; 退火要等下一次冷却结束重设锚点)。
    let later = anchor + 3 * WINDOW_MS;
    now.store(later, Ordering::Relaxed);
    assert!(table.is_selectable(key, later));
    assert_eq!(table.get(key).cooldown_streak, 1);
}
