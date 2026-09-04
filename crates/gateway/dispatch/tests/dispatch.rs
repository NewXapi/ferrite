//! dispatch 健康表 / 分层选择 / failover 预算的行为测试。

use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::Dispatch;
use dispatch::candidate::resolve_candidate;
use dispatch::health::{FailureClass, HealthTable, MemoryHealthTable};
use dispatch::retry::{AttemptOutcome, Failover, RetryPolicy, run_retry_loop};
use dispatch::selector::{Selector, WeightedSelector};
use rand::SeedableRng;
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

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

fn assert_pick(
    units: &[RouteUnitRecord],
    health: &dyn HealthTable,
    exclude: &[String],
) -> Option<RouteUnitRecord> {
    let refs: Vec<&RouteUnitRecord> = units.iter().collect();
    let mut rng = rand::rngs::StdRng::seed_from_u64(42);
    WeightedSelector
        .pick(&refs, health, exclude, 0, &mut rng)
        .cloned()
}

// ---------- health (phase0 track_health.go 模型) ----------

/// 固定时钟表 + 测试配置 (冷却 10s→60s, 阈值 5)。
fn clock_table(now: &Arc<AtomicU64>) -> MemoryHealthTable {
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    MemoryHealthTable::with_config_and_clock(dispatch::health::HealthSetting::default(), clock)
}

#[test]
fn fatal_streak_trips_cooldown_and_recovers_with_ramp() {
    let now = Arc::new(AtomicU64::new(1_000_000));
    let table = clock_table(&now);

    // 4 次 5xx 未达阈值 → 仍可选
    for _ in 0..4 {
        table.record("u1", Err(FailureClass::Retryable));
    }
    assert!(table.is_selectable("u1", now.load(Ordering::Relaxed)));

    // 第 5 次 → 冷却, base 10s (cooldown_streak=0 → 第一次激活恰好 base)
    table.record("u1", Err(FailureClass::Retryable));
    let st = table.get("u1");
    assert_eq!(st.cooldown_streak, 1);
    assert_eq!(
        st.cooldown_until_ms - 1_000_000,
        10_000,
        "首次冷却应为基础 10s"
    );
    assert!(!table.is_selectable("u1", now.load(Ordering::Relaxed)));

    // 冷却中失败不延长 (phase0 语义: 冷却时长由 cooldown_streak 决定, 无顺延)
    let first_expiry = table.get("u1").cooldown_until_ms;
    now.store(first_expiry - 5000, Ordering::Relaxed);
    table.record("u1", Err(FailureClass::Retryable));
    assert_eq!(
        table.get("u1").cooldown_until_ms,
        first_expiry,
        "phase0: 冷却时长只由 streak 决定"
    );

    // 冷却结束 → 恢复可选, 进入 slow-start ramp (RampPending → 权重 = 1/min_requests)
    let expiry = table.get("u1").cooldown_until_ms;
    now.store(expiry + 1, Ordering::Relaxed);
    assert!(table.is_selectable("u1", expiry + 1));
    let st = table.get("u1");
    assert!(st.ramp_pending, "冷却结束后应武装 ramp");
    // RoutingWeight = base × ewma_score × slow_start_factor; ramp_pending → 1/min_requests
    let w = table.routing_weight("u1", 100, expiry + 1);
    assert!(
        (w - 100.0 * (1.0 / 5.0)).abs() < 1e-6,
        "ramp 地板权重: got {w}"
    );

    // 一次成功 → ramp 前进: request_count=1 → factor = 1/5
    table.record("u1", Ok(200));
    let st = table.get("u1");
    assert!(!st.ramp_pending, "成功应退出 ramp_pending");
    assert_eq!(st.request_count, 1);
    let w = table.routing_weight("u1", 100, expiry + 1);
    assert!(
        (w - 100.0 * (1.0 / 5.0)).abs() < 1e-6,
        "ramp 1/5 权重: got {w}"
    );
}

#[test]
fn outcome_classification_matches_phase0() {
    let table = MemoryHealthTable::new();

    // 2xx → Success: ewma 上升
    for _ in 0..6 {
        table.record("u1", Ok(200));
    }
    assert_eq!(table.get("u1").ewma_score, 1.0, "全成功 → 满健康分");

    // 429 → Throttled: obs=0.7, 轻度降权非致命
    let table = MemoryHealthTable::new();
    for _ in 0..6 {
        table.record("u2", Ok(200));
    }
    table.record("u2", Ok(429)); // 注意: 429 是 Ok(status), 按状态码分类
    let st = table.get("u2");
    assert!(
        (st.ewma_score - (0.3 * 0.7 + 0.7 * 1.0)).abs() < 1e-9,
        "429 → obs 0.7 混合: got {}",
        st.ewma_score
    );

    // 400 → Neutral: 不改分不计请求
    let table = MemoryHealthTable::new();
    table.record("u3", Ok(400));
    let st = table.get("u3");
    assert_eq!(st.request_count, 0, "Neutral 不计请求");
    assert_eq!(st.ewma_score, 1.0, "Neutral 不改分");
    assert_eq!(st.failure_streak, 0);
}

#[test]
fn unauthorized_escalates_after_three_401() {
    let table = MemoryHealthTable::new();
    // 孤立 401 → Neutral (不惩罚渠道)
    table.record("u1", Ok(401));
    assert_eq!(table.get("u1").request_count, 0);
    // 连续 3 次 401 → Fatal
    for _ in 0..2 {
        table.record("u1", Ok(401));
    }
    let st = table.get("u1");
    assert_eq!(st.unauthorized_run, 3);
    assert!(st.ramp_exited, "升级 Fatal 应退出 ramp");
}

#[test]
fn cooldown_duration_escalates_with_streak() {
    let now = Arc::new(AtomicU64::new(1_000_000));
    let table = clock_table(&now);

    // 第一次冷却: base 10s
    for _ in 0..5 {
        table.record("u1", Err(FailureClass::Retryable));
    }
    let first = table.get("u1").cooldown_until_ms;
    assert_eq!(first - 1_000_000, 10_000);

    // 冷却结束 → 再来 5 次 → 第二次冷却: base + (max-base)×(1-α^1) = 10+50×0.7 = 45s
    now.store(first + 1, Ordering::Relaxed);
    for _ in 0..5 {
        table.record("u1", Err(FailureClass::Retryable));
    }
    let second = table.get("u1").cooldown_until_ms;
    assert_eq!(
        second - (first + 1),
        45_000,
        "第二次冷却应 45s (streak=1, α=0.3)"
    );
}

#[test]
fn ewma_score_floor_and_ramp_exit() {
    // 不触发冷却的配置 (阈值 999): 全 5xx → score 跌向 MinScore (0.05) 下限,
    // 且 ramp_exited 保持 true (真实失败退出 ramp)。
    let cfg = dispatch::health::HealthSetting {
        min_requests: 0,
        cooldown_threshold: 999,
        ..Default::default()
    };
    let now = Arc::new(AtomicU64::new(0));
    let clock = {
        let now = now.clone();
        move || now.load(Ordering::Relaxed)
    };
    let table = MemoryHealthTable::with_config_and_clock(cfg, clock);
    for _ in 0..30 {
        table.record("u1", Err(FailureClass::Retryable));
    }
    let st = table.get("u1");
    assert!(
        st.ewma_score >= 0.05 - 1e-9,
        "score 不应低于 MinScore: {}",
        st.ewma_score
    );
    assert!(st.ramp_exited, "fatal 应立即退出 ramp");
}

#[test]
fn routing_weight_zero_while_cooling() {
    let now = Arc::new(AtomicU64::new(1_000_000));
    let table = clock_table(&now);
    for _ in 0..5 {
        table.record("u1", Err(FailureClass::Retryable));
    }
    let w = table.routing_weight("u1", 100, now.load(Ordering::Relaxed));
    assert_eq!(w, 0.0, "冷却中权重为 0");
}

#[test]
fn retry_loop_succeeds_after_one_failover() {
    // 第一次尝试 Retryable → mark_tried → 第二次命中另一候选 → Done。
    let mut attempts = 0u32;
    let result = rt().block_on(run_retry_loop(
        "g",
        "m",
        &RetryPolicy { max_attempts: 3 },
        |_: &str, _: &str, exclude: &[String]| {
            let key = if exclude.is_empty() { "ch1" } else { "ch2" };
            let mut u = unit("g", "m", key, 10, 10, 1);
            u.channel_key = key.to_string();
            Ok(dispatch::candidate::Candidate {
                unit: u,
                secret: "s".into(),
                base_url: "https://u".into(),
                upstream_model: "m".into(),
            })
        },
        |_: &dispatch::candidate::Candidate| {
            attempts += 1;
            let n = attempts;
            async move {
                if n == 1 {
                    AttemptOutcome::Retryable(FailureClass::Retryable)
                } else {
                    AttemptOutcome::Done { status: 200 }
                }
            }
        },
        |_k, _o| {},
    ));
    // 第二次尝试命中另一候选 → Done, run_retry_loop 返回 Ok。
    assert!(
        matches!(result, Ok(AttemptOutcome::Done { status: 200 })),
        "可重试失败后换候选应最终成功, got: {result:?}"
    );
    assert_eq!(attempts, 2, "第一次可重试失败后应换候选重试");
}

#[test]
fn retry_loop_exhausts_budget_with_retryable() {
    // 全部可重试失败 → 预算耗尽 → RetriesExhausted。
    let err = rt().block_on(run_retry_loop(
        "g",
        "m",
        &RetryPolicy { max_attempts: 2 },
        |_: &str, _: &str, exclude: &[String]| {
            let key = if exclude.is_empty() { "ch1" } else { "ch2" };
            let mut u = unit("g", "m", key, 10, 10, 1);
            u.channel_key = key.to_string();
            Ok(dispatch::candidate::Candidate {
                unit: u,
                secret: "s".into(),
                base_url: "https://u".into(),
                upstream_model: "m".into(),
            })
        },
        |_: &dispatch::candidate::Candidate| async {
            AttemptOutcome::Retryable(FailureClass::Retryable)
        },
        |_k, _o| {},
    ));
    assert!(matches!(
        err,
        Err(dispatch::DispatchError::RetriesExhausted { .. })
    ));
}

#[test]
fn retry_loop_fatal_stops_immediately() {
    // Fatal → 不重试, 单次尝试即返回。
    let mut attempts = 0u32;
    let outcome = rt().block_on(run_retry_loop(
        "g",
        "m",
        &RetryPolicy { max_attempts: 3 },
        |_: &str, _: &str, _: &[String]| {
            let u = unit("g", "m", "ch1", 10, 10, 1);
            Ok(dispatch::candidate::Candidate {
                unit: u,
                secret: "s".into(),
                base_url: "https://u".into(),
                upstream_model: "m".into(),
            })
        },
        |_: &dispatch::candidate::Candidate| {
            attempts += 1;
            async move { AttemptOutcome::Fatal(FailureClass::Fatal) }
        },
        |_k, _o| {},
    ));
    assert_eq!(attempts, 1, "Fatal 不应重试");
    assert!(matches!(outcome, Ok(AttemptOutcome::Fatal(_))));
}

#[test]
fn retry_loop_select_error_propagates() {
    // select 报 NoCandidate → 循环立即向上传播, 不再尝试。
    let err = rt().block_on(run_retry_loop(
        "g",
        "m",
        &RetryPolicy { max_attempts: 3 },
        |group: &str, model: &str, _: &[String]| {
            Err(dispatch::DispatchError::NoCandidate {
                group: group.into(),
                model: model.into(),
            })
        },
        |_: &dispatch::candidate::Candidate| async { AttemptOutcome::Done { status: 200 } },
        |_k, _o| {},
    ));
    assert!(matches!(
        err,
        Err(dispatch::DispatchError::NoCandidate { .. })
    ));
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
    // weight=0 单元仍可被选中 (routingBaseWeight +1 语义, base 1:11 ≈ 8.3%),
    // 但份额必须显著低于重权重单元。固定 seed → 确定性, 跨 12 seed 累计
    // 验证 (单 seed 300 抽 8.3% 可能落空, 累计后 P(一次都不中)≈0)。
    let units = [
        unit("g", "m", "zero", 10, 0, 1),
        unit("g", "m", "heavy", 10, 10, 1),
    ];
    let health = MemoryHealthTable::new();
    let mut zero_picks = 0usize;
    let mut total = 0usize;
    for seed in 1u64..=12 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        for _ in 0..300 {
            let refs: Vec<&RouteUnitRecord> = units.iter().collect();
            let picked = WeightedSelector
                .pick(&refs, &health, &[], 0, &mut rng)
                .unwrap();
            if picked.meta.key == "zero" {
                zero_picks += 1;
            }
            total += 1;
        }
    }
    assert!(
        zero_picks > 0,
        "weight=0 候选必须仍可被选中 (routingBaseWeight +1 语义)"
    );
    let ratio = zero_picks as f64 / total as f64;
    assert!(
        ratio < 0.5,
        "weight=0 份额应显著低于 weight=10: {ratio:.3} ({zero_picks}/{total})"
    );
}

#[test]
fn weighted_distribution_matches_configured_ratio() {
    // pick_weighted 核心数学: 同 priority 层, weight 10:30 → 期望 25%/75%。
    // 用固定 seed 的 StdRng 注入 → 结果完全确定性, 永无 flake;
    // 跨 8 个 seed 验证比例方向与幅度 (75% ± 5%)。
    let units = [
        unit("g", "m", "light", 10, 10, 1),
        unit("g", "m", "heavy", 10, 30, 1),
    ];
    let health = MemoryHealthTable::new();
    let n = 2000;
    for seed in 1u64..=8 {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
        let mut heavy = 0;
        for _ in 0..n {
            let refs: Vec<&RouteUnitRecord> = units.iter().collect();
            let picked = WeightedSelector
                .pick(&refs, &health, &[], 0, &mut rng)
                .unwrap();
            if picked.meta.key == "heavy" {
                heavy += 1;
            }
        }
        let ratio = heavy as f64 / n as f64;
        assert!(
            (0.70..=0.80).contains(&ratio),
            "seed {seed}: weight 10:30 期望 ~75%, 实测 {ratio:.3} ({heavy}/{n})"
        );
    }
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
    let health2 = MemoryHealthTable::with_config_and_clock(
        dispatch::health::HealthSetting::default(),
        clock2,
    );
    for _ in 0..5 {
        health2.record("a", Err(FailureClass::Retryable));
        health2.record("b", Err(FailureClass::Retryable));
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
    let health =
        MemoryHealthTable::with_config_and_clock(dispatch::health::HealthSetting::default(), clock);
    let units = [
        unit("g", "m", "top", 20, 100, 1),
        unit("g", "m", "low", 10, 100, 1),
    ];
    for _ in 0..5 {
        health.record("top", Err(FailureClass::Retryable));
    }
    // top 冷却窗口 [0, 10000), now=5000 正处于冷却中
    now.store(5_000, Ordering::Relaxed);
    assert!(!health.is_selectable("top", 5_000), "top 应在冷却中");
    assert!(health.is_selectable("low", 5_000), "low 应可用");
    // top 冷却中 → 落到 low 层 (wildtoken selectWeightedByPriority fallthrough)
    assert_eq!(assert_pick(&units, &health, &[]).unwrap().meta.key, "low");
}

// ---------- retry ----------

#[test]
fn failover_budget_and_tried_set() {
    let mut f = Failover::new(RetryPolicy { max_attempts: 3 });
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
fn failover_end_to_end_switches_candidate() {
    // 端到端: select 得 A → 失败 mark_tried(A) → 下次 select 排除 A 换 B。
    let units = vec![
        unit("g", "m", "ch1", 10, 10, 1),
        unit("g", "m", "ch2", 10, 10, 1),
    ];
    let mut channels: HashMap<String, ChannelRecord> = HashMap::new();
    channels.insert("ch1".to_string(), channel("ch1", "s1", "https://u1"));
    channels.insert("ch2".to_string(), channel("ch2", "s2", "https://u2"));
    let snap = Arc::new(dispatch::Snapshot { units, channels });
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));

    let mut failover = dispatch::retry::Failover::new(dispatch::retry::RetryPolicy::default());
    assert_eq!(failover.next_attempt(), Some(1));
    let first = dispatcher.select("g", "m", failover.exclude()).unwrap();
    failover.mark_tried(first.unit.meta.key.clone());

    assert_eq!(failover.next_attempt(), Some(2));
    let second = dispatcher.select("g", "m", failover.exclude()).unwrap();
    assert_ne!(
        second.unit.meta.key, first.unit.meta.key,
        "排除已试候选后必须换渠道"
    );
    assert!(failover.exclude().contains(&first.unit.meta.key));
}

#[test]
fn single_candidate_is_selected() {
    // 单候选: 直接选中, 不经过分层随机 (new-api selectByWeight 单候选直返语义)
    let units = vec![unit("g", "m", "only", 10, 0, 1)];
    let mut channels: HashMap<String, ChannelRecord> = HashMap::new();
    channels.insert("only".to_string(), channel("only", "s1", "https://u1"));
    let snap = Arc::new(dispatch::Snapshot { units, channels });
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));
    let c = dispatcher.select("g", "m", &[]).unwrap();
    assert_eq!(c.unit.meta.key, "only");
}

#[test]
fn dispatcher_missing_key_index_returns_no_candidate() {
    let mut u = unit("g", "m", "ch1", 10, 10, 1);
    u.key_index = 5; // channel 只有 index=0 的 key
    let mut channels: HashMap<String, ChannelRecord> = HashMap::new();
    channels.insert(
        "ch1".to_string(),
        channel("ch1", "sk-secret", "https://upstream.example"),
    );
    let snap = Arc::new(dispatch::Snapshot {
        units: vec![u],
        channels,
    });
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));
    assert!(matches!(
        dispatcher.select("g", "m", &[]),
        Err(dispatch::DispatchError::NoCandidate { .. })
    ));
}

#[test]
fn dispatcher_skips_disabled_channel() {
    // channel.status = 2 (手动禁用) → 单元不可调度 (new-api 渠道状态门控)
    let mut ch = channel("ch1", "sk-secret", "https://upstream.example");
    ch.status = 2;
    let mut channels: HashMap<String, ChannelRecord> = HashMap::new();
    channels.insert("ch1".to_string(), ch);
    let snap = Arc::new(dispatch::Snapshot {
        units: vec![unit("g", "m", "ch1", 10, 10, 1)],
        channels,
    });
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));
    assert!(matches!(
        dispatcher.select("g", "m", &[]),
        Err(dispatch::DispatchError::NoCandidate { .. })
    ));
}

#[test]
fn dispatcher_fails_closed_before_snapshot() {
    let dispatcher = dispatch::Dispatcher::new(None, Arc::new(MemoryHealthTable::new()));
    assert!(matches!(
        dispatcher.select("g", "m", &[]),
        Err(dispatch::DispatchError::SnapshotNotReady)
    ));
}
