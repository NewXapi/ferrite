//! P1-B 渠道相关 4xx 降层 — `run_retry_loop` 的循环决策契约。
//!
//! 测的行为 (全是「换不换候选 / 客户端最终看到什么」的可观察结果, 非接线):
//! 1. 渠道相关 4xx (401/403/404 形态 → `FatalButSwitchable`) 的首候选失败 →
//!    该候选进排除集、第二候选被选中并获胜; 健康回报载荷为 `Err(Fatal)`,
//!    即 health::classify 的 Neutral 语义 (不改 EWMA 不记 streak);
//! 2. 请求相关 4xx (400 形态 → `Fatal`) 单候选 → 直接透传: 一次尝试即终止,
//!    不再发起第二次 select;
//! 3. 最后一个候选 `FatalButSwitchable` → 循环以 Ok 带回该结果供上层透传,
//!    而不是伪装成 503 NoCandidate;
//! 4. 「最后一个候选才透传」的反例: switchable 之后又试了一个 Retryable 候选
//!    且无路可换 → 仍是 Err(NoCandidate), 暂存的 4xx 不得复活。
//!
//! classify_channel_scope 的纯分类断言在 failure_scope.rs;
//! egress 状态码 → channel_scoped 的映射由 stage 闭包消费, 端到端形状
//! 见 apps/gateway/tests/route_resolution.rs P0-2。

use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::health::FailureClass;
use dispatch::{AttemptOutcome, Candidate, DispatchError, RetryPolicy, run_retry_loop};
use gateway_pipeline::ctx::SelectedRoute;

fn candidate(key: &str) -> Candidate {
    SelectedRoute {
        unit: RouteUnitRecord {
            meta: SyncMeta {
                key: key.to_string(),
                schema_version: 1,
                logical_version: 1,
                origin: "test".to_string(),
                updated_at: chrono::Utc::now(),
            },
            group: "g".to_string(),
            public_model: "m".to_string(),
            channel_key: format!("ch-{key}"),
            key_index: 0,
            upstream_model: "m".to_string(),
            priority: 10,
            weight: 10,
            status: 1,
        },
        secret: "sk-test".to_string(),
        base_url: format!("http://upstream-{key}.invalid"),
        upstream_model: "m".to_string(),
        provider_type: "openai".to_string(),
        settings: serde_json::Value::Null,
    }
}

/// 单次尝试的计划结果。AttemptOutcome 非 Clone, 按 key 现造。
#[derive(Clone, Copy, PartialEq)]
enum Plan {
    /// 上游 200。
    Ok,
    /// 渠道相关 4xx (egress 401/403/404 → channel_scoped 的循环载荷)。
    Switch,
    /// 可重试失败 (5xx/传输层形态)。
    Retry,
    /// 请求相关 4xx (400 形态, 不换渠道)。
    Fatal,
}

fn outcome(plan: Plan) -> AttemptOutcome {
    match plan {
        Plan::Ok => AttemptOutcome::Done { status: 200 },
        Plan::Switch => AttemptOutcome::FatalButSwitchable(FailureClass::Fatal),
        Plan::Retry => AttemptOutcome::Retryable(FailureClass::Retryable),
        Plan::Fatal => AttemptOutcome::Fatal(FailureClass::Fatal),
    }
}

struct Harness {
    pool: Vec<Candidate>,
    plans: Vec<(&'static str, Plan)>,
    /// 每次 select 收到的排除集。
    selects: Vec<Vec<String>>,
    /// 每次 report 收到的 (unit_key, outcome)。
    reports: Vec<(String, Result<u16, FailureClass>)>,
}

impl Harness {
    async fn run(
        mut self,
    ) -> (
        Result<(dispatch::Attempt, AttemptOutcome), DispatchError>,
        Vec<Vec<String>>,
        Vec<(String, Result<u16, FailureClass>)>,
    ) {
        let result = run_retry_loop(
            "g",
            "m",
            &RetryPolicy::default(),
            |g, m, exclude| {
                self.selects.push(exclude.to_vec());
                self.pool
                    .iter()
                    .find(|c| !exclude.contains(&c.unit.meta.key))
                    .cloned()
                    .ok_or(DispatchError::NoCandidate {
                        group: g.to_string(),
                        model: m.to_string(),
                    })
            },
            |c| {
                let plan = self
                    .plans
                    .iter()
                    .find(|(k, _)| *k == c.unit.meta.key)
                    .map(|(_, p)| *p)
                    .expect("plan for candidate");
                std::future::ready(outcome(plan))
            },
            |key, outcome| self.reports.push((key.to_string(), outcome)),
        )
        .await;
        (result, self.selects, self.reports)
    }
}

#[tokio::test]
async fn channel_scoped_4xx_first_candidate_switches_to_second() {
    let (result, selects, reports) = Harness {
        pool: vec![candidate("A"), candidate("B")],
        plans: vec![("A", Plan::Switch), ("B", Plan::Ok)],
        selects: Vec::new(),
        reports: Vec::new(),
    }
    .run()
    .await;

    let (win, outcome) = result.expect("A 渠道相关 4xx 应降层到 B 并成功");
    assert_eq!(win.candidate.unit.meta.key, "B", "第二候选必须被选中");
    assert_eq!(win.attempt_no, 2);
    assert!(matches!(outcome, AttemptOutcome::Done { status: 200 }));
    // 换候选靠排除集: 第二次 select 必须排除 A。
    assert_eq!(selects, vec![Vec::<String>::new(), vec!["A".to_string()]]);
    // 健康回报: A 走 Err(Fatal) = Neutral 语义 (不污染 EWMA), B 记 200。
    assert_eq!(
        reports,
        vec![
            ("A".to_string(), Err(FailureClass::Fatal)),
            ("B".to_string(), Ok(200)),
        ]
    );
}

#[tokio::test]
async fn request_scoped_400_single_candidate_passes_through_immediately() {
    let (result, selects, reports) = Harness {
        pool: vec![candidate("A")],
        plans: vec![("A", Plan::Fatal)],
        selects: Vec::new(),
        reports: Vec::new(),
    }
    .run()
    .await;

    let (win, outcome) = result.expect("请求相关 4xx 以 Ok(Fatal) 终态带回");
    assert_eq!(win.candidate.unit.meta.key, "A");
    assert_eq!(win.attempt_no, 1);
    assert!(matches!(outcome, AttemptOutcome::Fatal(_)));
    // 关键行为: 不换渠道 = 只发起过一次 select。
    assert_eq!(selects.len(), 1, "400 形态不得触发换候选");
    assert_eq!(reports, vec![("A".to_string(), Err(FailureClass::Fatal))]);
}

#[tokio::test]
async fn last_candidate_channel_scoped_passes_through_not_no_route() {
    let (result, _selects, _reports) = Harness {
        pool: vec![candidate("A")],
        plans: vec![("A", Plan::Switch)],
        selects: Vec::new(),
        reports: Vec::new(),
    }
    .run()
    .await;

    // 唯一候选回 404: 客户端应拿到上游真实 4xx (循环带回 switchable 终态),
    // 而不是被 503 NoCandidate 掩盖。
    let (last, outcome) = result.expect("最后候选的渠道相关 4xx 必须透传");
    assert_eq!(last.candidate.unit.meta.key, "A");
    assert!(matches!(outcome, AttemptOutcome::FatalButSwitchable(_)));
}

#[tokio::test]
async fn stale_switchable_does_not_resurrect_after_retryable_last_failure() {
    let (result, _selects, _reports) = Harness {
        pool: vec![candidate("A"), candidate("B")],
        plans: vec![("A", Plan::Switch), ("B", Plan::Retry)],
        selects: Vec::new(),
        reports: Vec::new(),
    }
    .run()
    .await;

    // 「最后一个候选才透传」: 终次失败是 Retryable (非 switchable),
    // 无候选可换时仍按 NoCandidate 走, A 的暂存 4xx 不得复活。
    assert!(
        matches!(result, Err(DispatchError::NoCandidate { .. })),
        "got {result:?}"
    );
}
