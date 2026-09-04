//! 并发行为 — Mutex 保护的共享状态在多线程下不 panic、不越限。
//!
//! 覆盖: SlidingWindow 并发记账 (放行数必须 ≤ 限额 — Mutex 原子性),
//! MemoryHealthTable 并发 record/get (无 panic、状态可读、最终一致)。

use dispatch::health::{FailureClass, HealthTable, MemoryHealthTable};
use dispatch::ratelimit::{RateLimitSpec, SlidingWindow};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn sliding_window_concurrent_admits_never_exceeds_limit() {
    // 8 线程 × 200 请求并发打同一 key (限额 100/1s, 固定 now → 同一秒桶):
    // Mutex 串行化后放行数必须恰好 ≤ 100 — 多线程下记账泄漏/双放行会立刻暴露。
    let rl = Arc::new(SlidingWindow::new());
    let spec = RateLimitSpec {
        requests: 100,
        window_ms: 1_000,
    };
    let now = 1_000_000u64; // 固定时钟, 全部落入同一秒桶

    let mut handles = Vec::new();
    for _ in 0..8 {
        let rl = Arc::clone(&rl);
        handles.push(std::thread::spawn(move || {
            let mut allowed = 0;
            for _ in 0..200 {
                if rl.admits("shared", Some(&spec), now) {
                    allowed += 1;
                }
            }
            allowed
        }));
    }

    let total: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    assert!(total <= 100, "并发记账越过限额: {total} 放行 (限额 100)");
    // 固定单桶下理论上恰好 100; 留 ≤ 100 的保守断言防实现细节变化
    assert_eq!(total, 100, "同一秒桶、限额内, 并发后应恰好放行 100 次");
}

#[test]
fn health_table_concurrent_record_get_no_panic() {
    // 8 线程各 1000 次混合 record (成功/失败交错, 共享 16 个 key):
    // 验证 Mutex 保护下无 panic (毒化会把 unwrap_or_else 兜住 → 状态仍一致),
    // 且失败连击统计跨线程正确累计。
    let table = Arc::new(MemoryHealthTable::new());

    let mut handles = Vec::new();
    for _ in 0..8 {
        let table = Arc::clone(&table);
        handles.push(std::thread::spawn(move || {
            for i in 0..1000 {
                let key = format!("k{}", i % 16);
                let outcome = if i % 3 == 0 {
                    Err(FailureClass::Retryable)
                } else {
                    Ok(200)
                };
                table.record(&key, outcome);
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    // 无 panic 即过主断言; 附加: 状态可读且已有观测
    let st = table.get("k0");
    assert!(st.request_count > 0, "并发 record 后应有观测计数");
    assert!(
        st.ewma_score > 0.0 && st.ewma_score <= 1.0,
        "健康分应在 (0,1]"
    );
    // 并发交错下同一 key 的失败可能连续到达 (跨线程), 触发冷却与否是调度
    // 相关的 — 不在这里断言; 并发安全的核心断言是: 无 panic + 状态可读 +
    // 健康分有界。熔断行为由单线程测试 (fatal_streak_trips...) 覆盖。
}

#[test]
fn health_table_concurrent_streak_trips_breaker() {
    // 多线程对同一 key 连续打失败: 熔断阈值 5, 30 个失败跨线程抵达后
    // 必须进入冷却 — 验证 streak 在锁保护下跨线程正确累计。
    let table = Arc::new(MemoryHealthTable::new());

    let mut handles = Vec::new();
    for _ in 0..3 {
        let table = Arc::clone(&table);
        handles.push(std::thread::spawn(move || {
            for _ in 0..10 {
                table.record("hot", Err(FailureClass::Retryable));
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }

    let st = table.get("hot");
    assert!(
        st.cooldown_until_ms > 0,
        "30 个并发失败应触发熔断 (streak 跨线程累计)"
    );
    // 30 并发失败 ≥5 触发冷却; 冷却会重置 request_count=0, 断言冷却/降权结果
    assert!(
        st.cooldown_streak > 0,
        "30 并发失败应触发冷却 (streak 累计)"
    );
}

// ---------- 无锁路径并发读: Dispatcher select 与 health 读 ----------

#[test]
fn dispatcher_concurrent_select_returns_valid_candidate() {
    use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
    use dispatch::Dispatch;

    fn unit(key: &str) -> RouteUnitRecord {
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
            status: 1,
        }
    }

    let units: Vec<RouteUnitRecord> = (0..8).map(|i| unit(&format!("ch{i}"))).collect();
    let channels: HashMap<String, ChannelRecord> = units
        .iter()
        .map(|u| {
            (
                u.meta.key.clone(),
                ChannelRecord {
                    meta: u.meta.clone(),
                    name: u.meta.key.clone(),
                    provider_type: "openai".to_string(),
                    base_url: "https://u".to_string(),
                    keys: vec![ChannelKey {
                        index: 0,
                        secret: "s".to_string(),
                        rpm_limit: 0,
                    }],
                    max_concurrency: 10,
                    status: 1,
                    groups: vec!["g".to_string()],
                    settings: serde_json::Value::Null,
                },
            )
        })
        .collect();

    let dispatcher = Arc::new(dispatch::Dispatcher::new(
        Some(Arc::new(dispatch::Snapshot { units, channels })),
        Arc::new(MemoryHealthTable::new()),
    ));

    // 8 线程并发 select: 每次都必须成功返回一个可转发候选 (快照只读 + 健康
    // 表锁保护下无 panic/无空手而归)
    let mut handles = Vec::new();
    for _ in 0..8 {
        let d = Arc::clone(&dispatcher);
        handles.push(std::thread::spawn(move || {
            for _ in 0..200 {
                let c = d.select("g", "m", &[]).unwrap();
                assert!(!c.secret.is_empty());
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
}
