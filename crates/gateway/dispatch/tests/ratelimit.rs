//! 滑动窗口限流的解析与门控行为 — 解析边界 / 窗口滑动 / 按 key 隔离 / Dispatcher 集成。

use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::Dispatch;
use dispatch::health::MemoryHealthTable;
use dispatch::ratelimit::{RateLimitSpec, SlidingWindow};
use std::collections::HashMap;
use std::sync::Arc;

// ---------- 解析边界 ----------

#[test]
fn parse_basic_units() {
    assert_eq!(
        RateLimitSpec::parse("100/s"),
        Some(RateLimitSpec {
            requests: 100,
            window_ms: 1_000
        })
    );
    assert_eq!(
        RateLimitSpec::parse("200/5m"),
        Some(RateLimitSpec {
            requests: 200,
            window_ms: 300_000
        })
    );
    assert_eq!(
        RateLimitSpec::parse("10000/d"),
        Some(RateLimitSpec {
            requests: 10_000,
            window_ms: 86_400_000
        })
    );
    assert_eq!(
        RateLimitSpec::parse("50/h"),
        Some(RateLimitSpec {
            requests: 50,
            window_ms: 3_600_000
        })
    );
}

#[test]
fn parse_invalid_returns_none() {
    // 空
    assert_eq!(RateLimitSpec::parse(""), None);
    // 缺斜杠
    assert_eq!(RateLimitSpec::parse("100s"), None);
    // 多余尾部
    assert_eq!(RateLimitSpec::parse("100/sx"), None);
    // 非法单位
    assert_eq!(RateLimitSpec::parse("100/w"), None);
    // 非法字符
    assert_eq!(RateLimitSpec::parse("abc/s"), None);
    // requests == 0
    assert_eq!(RateLimitSpec::parse("0/s"), None);
    // 倍数 == 0 (空倍数等同 1, 不允许 0)
    assert_eq!(RateLimitSpec::parse("100/0s"), None);
    // 仅单位
    assert_eq!(RateLimitSpec::parse("/s"), None);
}

// ---------- 窗口滑动 ----------

#[test]
fn admits_respects_window_slide() {
    let rl = SlidingWindow::new();
    let spec = RateLimitSpec {
        requests: 60,
        window_ms: 60_000,
    }; // 60 req / 60s

    // 同秒内连发 60 个 → 全部放行 (单秒桶不分裂)
    let t0 = 1_000_000u64;
    for i in 0..60 {
        assert!(
            rl.admits("k", Some(&spec), t0 + i),
            "第 {} 个请求应放行",
            i + 1
        );
    }
    // 第 61 个请求同秒 → 该秒桶已达 60 → 拒绝
    assert!(!rl.admits("k", Some(&spec), t0 + 60));

    // 时间推进过窗口: t = t0 + 61s → 之前的桶全部落在 [now-60s, now] 之外 → 恢复
    let t1 = t0 + 61_000;
    assert!(rl.admits("k", Some(&spec), t1));
}

#[test]
fn admits_none_spec_always_allows() {
    let rl = SlidingWindow::new();
    for _ in 0..10_000 {
        assert!(rl.admits("k", None, 0));
    }
}

// ---------- 按 key 隔离 ----------

#[test]
fn keys_are_isolated() {
    let rl = SlidingWindow::new();
    let spec = RateLimitSpec {
        requests: 5,
        window_ms: 60_000,
    };

    // k1 用满 5 个
    for _ in 0..5 {
        assert!(rl.admits("k1", Some(&spec), 1_000));
    }
    assert!(!rl.admits("k1", Some(&spec), 1_000));

    // k2 完全不受影响
    for _ in 0..5 {
        assert!(rl.admits("k2", Some(&spec), 1_000));
    }
    assert!(!rl.admits("k2", Some(&spec), 1_000));
}

// ---------- Dispatcher 集成 ----------

fn unit(key: &str, status: u8) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        group: "g".to_string(),
        public_model: "m".to_string(),
        channel_key: key.to_string(),
        key_index: 0,
        upstream_model: "m".to_string(),
        priority: 10,
        weight: 10,
        status,
    }
}

fn channel(key: &str) -> ChannelRecord {
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
        base_url: "https://upstream.example".to_string(),
        keys: vec![ChannelKey {
            index: 0,
            secret: "sk-secret".to_string(),
            rpm_limit: 0,
        }],
        max_concurrency: 10,
        status: 1,
        groups: vec!["default".to_string()],
        settings: serde_json::Value::Null,
    }
}

fn build_snapshot(units: Vec<RouteUnitRecord>, channels: Vec<&str>) -> Arc<dispatch::Snapshot> {
    let mut ch_map = HashMap::new();
    for k in channels {
        ch_map.insert(k.to_string(), channel(k));
    }
    Arc::new(dispatch::Snapshot {
        units,
        channels: ch_map,
    })
}

#[test]
fn dispatcher_without_limits_keeps_legacy_behavior() {
    // 与现有 dispatcher_select_resolves_full_candidate 一致 — 不配 limits 时行为不变
    let units = vec![unit("ch1", 1)];
    let snap = build_snapshot(units, vec!["ch1"]);
    let dispatcher = dispatch::Dispatcher::new(Some(snap), Arc::new(MemoryHealthTable::new()));
    let c = dispatcher.select("g", "m", &[]).unwrap();
    assert_eq!(c.secret, "sk-secret");
}

#[test]
fn dispatcher_rate_limited_when_all_candidates_throttled() {
    // 两个单元, 都限流 1 req / 60s; 第一个放行后第二个也被吃掉,
    // 但 rl 共用同一窗口 — 这里两次 select 都返回 RateLimited (因为
    // 每个 select 路径独立过滤, 但同 key 共享窗口)。
    let units = vec![unit("ch1", 1), unit("ch2", 1)];
    let snap = build_snapshot(units.clone(), vec!["ch1", "ch2"]);

    let rl = Arc::new(SlidingWindow::new());
    let mut limits = HashMap::new();
    // 让两个单元共用同一 key 名 → 共享同一窗口 → 第二次必拒
    limits.insert(
        "ch1".to_string(),
        RateLimitSpec {
            requests: 1,
            window_ms: 60_000,
        },
    );
    limits.insert(
        "ch2".to_string(),
        RateLimitSpec {
            requests: 1,
            window_ms: 60_000,
        },
    );

    let dispatcher = dispatch::Dispatcher::with_limits(
        Some(snap),
        Arc::new(MemoryHealthTable::new()),
        limits,
        rl,
    );

    // 第一次 — 两个候选都未超限, 应正常返回 (任一被选中)
    let _ = dispatcher.select("g", "m", &[]).unwrap();

    // 第二次 — rl 状态使两 key 都超限, 过滤后为空 → RateLimited
    let err = dispatcher.select("g", "m", &[]).unwrap_err();
    match err {
        dispatch::DispatchError::RateLimited { group, model } => {
            assert_eq!(group, "g");
            assert_eq!(model, "m");
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[test]
fn dispatcher_rate_limited_with_per_unit_limits() {
    // 单元 a 限流 1 req / 60s, 单元 b 不限流。第一次选 a (RL 允许),
    // 第二次再请求 — a 被限流但 b 仍可放行, 应正常返回 b 而不是 RateLimited。
    let units = vec![unit("a", 1), unit("b", 1)];
    let snap = build_snapshot(units.clone(), vec!["a", "b"]);

    let rl = Arc::new(SlidingWindow::new());
    let mut limits = HashMap::new();
    limits.insert(
        "a".to_string(),
        RateLimitSpec {
            requests: 1,
            window_ms: 60_000,
        },
    );

    let dispatcher = dispatch::Dispatcher::with_limits(
        Some(snap),
        Arc::new(MemoryHealthTable::new()),
        limits,
        rl,
    );

    // 第一次: 任一被选中 — a 或 b 都行
    let _ = dispatcher.select("g", "m", &[]).unwrap();
    // 第二次: a 超限但 b 仍可放行 → 应成功
    let _ = dispatcher.select("g", "m", &[]).unwrap();
}

#[test]
fn dispatcher_no_candidate_when_snapshot_empty_unchanged() {
    // 原始候选本就为空 → 维持 NoCandidate, 不被 RateLimited 取代
    let snap = build_snapshot(vec![], vec![]);
    let mut limits = HashMap::new();
    limits.insert(
        "any".to_string(),
        RateLimitSpec {
            requests: 1,
            window_ms: 60_000,
        },
    );
    let dispatcher = dispatch::Dispatcher::with_limits(
        Some(snap),
        Arc::new(MemoryHealthTable::new()),
        limits,
        Arc::new(SlidingWindow::new()),
    );
    assert!(matches!(
        dispatcher.select("g", "m", &[]),
        Err(dispatch::DispatchError::NoCandidate { .. })
    ));
}
