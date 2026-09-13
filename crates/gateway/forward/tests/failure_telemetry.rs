//! 失败遥测集成测试 — 上游失败尝试落**零成本观测事件**。
//!
//! 覆盖（对应 forward::stage::submit_failed 契约）：
//! 1. 单次模式：forward_task 返回 `Err(NormalizedError)` → sink 恰好收到 1 条
//!    status=502、cost=0、counts 全 0、error 非空的事件，且 StageError 照旧短路；
//! 2. 重试模式：预算耗尽后**每次失败尝试各 1 条**（重试也留观测痕迹）；
//! 3. Fatal 4xx：单次尝试即终止，同样产出 1 条观测事件且不触碰第二候选；
//! 4. `is_stream` 记录请求意图：非流式请求失败 false、流式请求失败 true；
//! 5. 未挂 pt/sink 时失败路径行为不变（照常报错，不产出事件、不 panic）。
//!
//! mock 风格与 retry_wiring.rs 一致：手写 Dispatch + 脚本化 Egress，不发真实网络
//! （mock 与请求构造器提取在 `tests/common/mod.rs` 共享；本文件只构造失败计划）。

mod common;

use contract::records::UsageEventRecord;
use dispatch::RetryPolicy;
use forward::ForwardStage;
use gateway_pipeline::{Stage, StageError, UpstreamError};
use metering::pricing::{ModelPrice, PriceTable};
use std::sync::Arc;

use common::*;

// ---------- 测试辅助 ----------

/// 固定价表：证明零成本来自 counts 全 0，而非"没挂到价"。
struct FixedPriceTable;

impl PriceTable for FixedPriceTable {
    fn lookup(&self, _model: &str, _group: &str) -> Option<ModelPrice> {
        Some(ModelPrice {
            input: 15.0,
            output: 60.0,
            cache: 0.0,
            group_multiplier: 1.0,
        })
    }
}

/// 挂 fake pt/sink 的 stage 构造器，返回 stage 与 sink 句柄。
fn priced_stage(egress: Arc<ScriptedEgress>) -> (ForwardStage, Arc<VecSink>) {
    let sink = Arc::new(VecSink::default());
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = ForwardStage::new(egress, adaptors)
        .with_price_table(Arc::new(FixedPriceTable), sink.clone());
    (stage, sink)
}

/// 失败观测事件的基础断言：counts 全 0、cost=0、归因键与成功路径同源。
fn assert_zero_cost_observation(ev: &UsageEventRecord, unit_key: &str) {
    assert_eq!(
        (ev.prompt_tokens, ev.completion_tokens, ev.cached_tokens),
        (0, 0, 0),
        "观测事件 counts 必须全 0"
    );
    assert_eq!(ev.cost, 0, "观测事件不得产生账单");
    assert_eq!(ev.user_key, "tok-1");
    assert_eq!(ev.token_key, "tok-1");
    assert_eq!(ev.channel_key, format!("ch-{unit_key}"));
    assert_eq!(ev.route_unit_key, unit_key);
    assert_eq!(ev.public_model, "m");
    assert_eq!(ev.upstream_model, "m");
}

// ---------- 用例 1: 单次模式失败 → 1 条观测事件 + 照常短路 ----------

#[tokio::test]
async fn single_shot_failure_emits_one_zero_cost_event() {
    let c1 = candidate("c1");
    let egress = Arc::new(ScriptedEgress::new(vec![(
        "upstream-c1",
        Plan::Fail {
            status: 502,
            retryable: true,
        },
    )]));
    let (stage, sink) = priced_stage(egress.clone());

    let mut ctx = ctx_with_body(NON_STREAM_BODY, c1);
    let err = stage
        .handle(&mut ctx)
        .await
        .expect_err("上游 502 应短路为 StageError");
    match err {
        StageError::Upstream(UpstreamError::Status { code, .. }) => assert_eq!(code, 502),
        other => panic!("期望 Upstream 502, got {other:?}"),
    }

    let events = sink.events();
    assert_eq!(events.len(), 1, "失败请求必须留下 1 条观测痕迹");
    let ev = &events[0];
    assert_zero_cost_observation(ev, "c1");
    assert_eq!(ev.status_code, 502);
    assert_eq!(ev.error.as_deref(), Some("upstream 502"), "error 非空");
    assert!(!ev.is_stream, "非流式请求失败, is_stream=false");
    assert_eq!(egress.calls().len(), 1);
}

// ---------- 用例 2: 预算耗尽 → 每次失败尝试各 1 条 ----------

#[tokio::test]
async fn retry_exhaustion_records_each_failed_attempt() {
    let dispatch = Arc::new(MockDispatch::new(vec![candidate("c1"), candidate("c2")]));
    let egress = Arc::new(ScriptedEgress::new(vec![
        (
            "upstream-c1",
            Plan::Fail {
                status: 502,
                retryable: true,
            },
        ),
        (
            "upstream-c2",
            Plan::Fail {
                status: 502,
                retryable: true,
            },
        ),
    ]));
    let (stage, sink) = priced_stage(egress.clone());
    let stage = stage.with_retry(
        dispatch,
        RetryPolicy {
            max_attempts: 2,
            ..RetryPolicy::default()
        },
    );

    let mut ctx = ctx_with_body(NON_STREAM_BODY, candidate("c1"));
    let err = stage.handle(&mut ctx).await.expect_err("预算耗尽应报错");
    assert!(
        matches!(
            err,
            StageError::Upstream(UpstreamError::Status { code: 502, .. })
        ),
        "期望 Upstream 502, got {err:?}"
    );

    let events = sink.events();
    assert_eq!(events.len(), 2, "重试预算内的每次失败尝试都要留痕");
    assert_zero_cost_observation(&events[0], "c1");
    assert_zero_cost_observation(&events[1], "c2");
    assert!(
        events
            .iter()
            .all(|e| e.status_code == 502 && e.error.is_some()),
        "两条都应是 502 且带错误摘要: {events:?}"
    );
    assert!(
        events.iter().all(|e| !e.is_stream),
        "非流式请求的两次失败都应为 is_stream=false"
    );
    assert_eq!(egress.calls().len(), 2);
}

// ---------- 用例 3: Fatal 4xx 同样留痕, 且不触碰第二候选 ----------

#[tokio::test]
async fn fatal_4xx_records_one_event_and_short_circuits() {
    let dispatch = Arc::new(MockDispatch::new(vec![candidate("c1"), candidate("c2")]));
    let egress = Arc::new(ScriptedEgress::new(vec![
        (
            "upstream-c1",
            Plan::Fail {
                status: 400,
                retryable: false,
            },
        ),
        (
            "upstream-c2",
            Plan::Fail {
                status: 502,
                retryable: true,
            },
        ),
    ]));
    let (stage, sink) = priced_stage(egress.clone());
    let stage = stage.with_retry(dispatch, RetryPolicy::default());

    let mut ctx = ctx_with_body(NON_STREAM_BODY, candidate("c1"));
    let err = stage
        .handle(&mut ctx)
        .await
        .expect_err("Fatal 4xx 应透传报错");
    match err {
        StageError::Upstream(UpstreamError::Status { code, .. }) => assert_eq!(code, 400),
        other => panic!("期望 Upstream 400, got {other:?}"),
    }

    let events = sink.events();
    assert_eq!(events.len(), 1, "Fatal 终止也要留下观测痕迹");
    assert_zero_cost_observation(&events[0], "c1");
    assert_eq!(events[0].status_code, 400);
    assert_eq!(events[0].error.as_deref(), Some("upstream 400"));
    assert_eq!(egress.calls().len(), 1, "Fatal 不得触碰 c2");
}

// ---------- 用例 4: 流式请求失败 → is_stream=true ----------

#[tokio::test]
async fn streaming_request_failure_marks_is_stream_true() {
    let egress = Arc::new(ScriptedEgress::new(vec![(
        "upstream-c1",
        Plan::Fail {
            status: 502,
            retryable: true,
        },
    )]));
    let (stage, sink) = priced_stage(egress.clone());

    let mut ctx = ctx_with_body(STREAM_BODY, candidate("c1"));
    let err = stage.handle(&mut ctx).await.expect_err("应短路报错");
    assert!(
        matches!(
            err,
            StageError::Upstream(UpstreamError::Status { code: 502, .. })
        ),
        "got {err:?}"
    );

    let events = sink.events();
    assert_eq!(events.len(), 1);
    assert!(
        events[0].is_stream,
        "is_stream 记录请求意图: 流式请求失败仍应 true"
    );
    assert_eq!(events[0].cost, 0);
    assert_eq!(events[0].status_code, 502);
}

// ---------- 用例 5: 未挂 pt/sink → 行为与接计费前一致 ----------

#[tokio::test]
async fn failure_without_sink_still_short_circuits() {
    let egress = Arc::new(ScriptedEgress::new(vec![(
        "upstream-c1",
        Plan::Fail {
            status: 502,
            retryable: true,
        },
    )]));
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = ForwardStage::new(egress.clone(), adaptors); // 不 with_price_table

    let mut ctx = ctx_with_body(NON_STREAM_BODY, candidate("c1"));
    let err = stage
        .handle(&mut ctx)
        .await
        .expect_err("未挂计费通道不影响报错语义");
    assert!(
        matches!(
            err,
            StageError::Upstream(UpstreamError::Status { code: 502, .. })
        ),
        "got {err:?}"
    );
    assert_eq!(egress.calls().len(), 1);
}
