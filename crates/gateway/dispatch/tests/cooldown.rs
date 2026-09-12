//! P1-A outcome 驱动的差异化冷却行为测试 (规格: todo/gateway-resilience.md P1-A)。
//!
//! 测的行为——冷却时长按触发 outcome 分档, 全部经 MemoryHealthTable 公开 API +
//! 注入时钟断言 `cooldown_until_ms - now` (外部可观测时长, 不测私有函数):
//! - 401-run 升级的 Fatal → max 档, 显著长于 5xx streak 的曲线首档;
//! - 429 → base 短冷却, 第二档激活仍钉在 base (5xx 曲线此时已爬向 max);
//! - Neutral (400 / 孤立 401) 永不触发冷却、不记 streak;
//! - unauthorized_run 计数扩展到 403 (混合凭据 run 同样升级)。

use dispatch::health::{
    ChannelOutcome, FailureClass, HealthSetting, HealthState, HealthTable, MemoryHealthTable,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// 默认配置: base 10s / max 60s / threshold 5 / 曲线 α 0.3。
const T0: u64 = 1_000_000;

fn clock_table(now: &Arc<AtomicU64>) -> MemoryHealthTable {
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    MemoryHealthTable::with_config_and_clock(HealthSetting::default(), clock)
}

fn cooldown_len_ms(st: &HealthState, now: u64) -> u64 {
    st.cooldown_until_ms.saturating_sub(now)
}

#[test]
fn unauthorized_run_cooldown_is_max_tier_and_beats_5xx_curve() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    // 5xx streak: 首次激活走 phase0 曲线 → base 10s。
    for _ in 0..5 {
        table.record("five_xx", Ok(500));
    }
    let st5 = table.get("five_xx");
    assert_eq!(cooldown_len_ms(&st5, T0), 10_000, "5xx 首档应为曲线 base");
    assert_eq!(st5.last_cooling_outcome, Some(ChannelOutcome::Fatal));

    // 401 run: 前 2 次 Neutral, 第 3 次起升级 Fatal 计 streak, 第 7 次触顶 → max 档。
    for _ in 0..7 {
        table.record("unauthorized", Ok(401));
    }
    let st4 = table.get("unauthorized");
    assert_eq!(
        cooldown_len_ms(&st4, T0),
        60_000,
        "401-run 升级的 Fatal 应直接 max 档 (key 失效是持续态)"
    );
    assert!(
        cooldown_len_ms(&st4, T0) > cooldown_len_ms(&st5, T0),
        "401-run 冷却必须长于 5xx 同档冷却"
    );
}

#[test]
fn throttled_cooldown_stays_at_base_across_activations() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    for _ in 0..5 {
        table.record("throttled", Ok(429));
    }
    let st = table.get("throttled");
    assert_eq!(cooldown_len_ms(&st, T0), 10_000, "429 应钉在 base 短冷却");
    assert_eq!(st.last_cooling_outcome, Some(ChannelOutcome::Throttled));

    // 冷却过期后再次触顶: 5xx 曲线第二档已爬到 45s, 429 仍是 base (暂态不爬升)。
    let t1 = st.cooldown_until_ms + 1;
    now.store(t1, Ordering::Relaxed);
    for _ in 0..5 {
        table.record("throttled", Ok(429));
    }
    let st = table.get("throttled");
    assert_eq!(st.cooldown_streak, 2);
    assert_eq!(cooldown_len_ms(&st, t1), 10_000, "第二轮 429 仍应是 base 短冷却");
}

#[test]
fn neutral_never_enters_cooldown() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    // 请求坏 (400) 与孤立 401 (后跟成功) 都是渠道无责。
    for _ in 0..20 {
        table.record("bad_request", Ok(400));
        table.record("orphan_401", Ok(401));
        table.record("orphan_401", Ok(200));
    }
    for key in ["bad_request", "orphan_401"] {
        let st = table.get(key);
        assert_eq!(st.cooldown_until_ms, 0, "{key} 不应进冷却");
        assert_eq!(st.failure_streak, 0, "{key} Neutral 不记 streak");
        assert_eq!(st.last_cooling_outcome, None);
        assert!(table.is_selectable(key, T0));
    }
    // Neutral 不改分: 全 400 的渠道 EWMA 保持满格。
    assert_eq!(table.get("bad_request").ewma_score, 1.0);
}

#[test]
fn unauthorized_run_counts_403_alongside_401() {
    let now = Arc::new(AtomicU64::new(T0));
    let table = clock_table(&now);

    // 混合 run: 401, 403 孤立时仍 Neutral (不改分), 第 3 次达阈值升级 Fatal。
    table.record("mixed", Ok(401));
    table.record("mixed", Ok(403));
    let st = table.get("mixed");
    assert_eq!(st.unauthorized_run, 2);
    assert_eq!(st.failure_streak, 0, "孤立 401/403 应保持 Neutral 语义");
    table.record("mixed", Ok(403));
    let st = table.get("mixed");
    assert_eq!(st.unauthorized_run, 3, "403 应与 401 同样计数");
    assert_eq!(st.failure_streak, 1, "run 达阈值应升级 Fatal 并记 streak");

    // 成功清零 run; 403 不再走旧 `_` 分支的清零语义。
    table.record("mixed", Ok(200));
    assert_eq!(table.get("mixed").unauthorized_run, 0);

    // 传输层失败 (非凭据) 走曲线档而不是 max 档: 先攒 run 再断连 → 清零。
    table.record("transport", Ok(401));
    table.record("transport", Ok(401));
    table.record("transport", Err(FailureClass::Retryable));
    assert_eq!(table.get("transport").unauthorized_run, 0);
}
