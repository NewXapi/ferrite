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
//! mock 风格与 retry_wiring.rs 一致：手写 Dispatch + 脚本化 Egress，不发真实网络。

use bytes::Bytes;
use contract::error::NormalizedError;
use contract::records::{RouteUnitRecord, SyncMeta, UsageEventRecord};
use dispatch::health::FailureClass;
use dispatch::{Candidate, Dispatch, DispatchError, RetryPolicy};
use forward::ForwardStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::{Stage, StageError, TokenInfo, UpstreamError};
use metering::SettleSink;
use metering::pricing::{ModelPrice, PriceTable};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

// ---------- 测试辅助 ----------

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
        // base_url 含 key 标记, ScriptedEgress 据此区分候选。
        base_url: format!("http://upstream-{key}.invalid"),
        upstream_model: "m".to_string(),
        provider_type: "openai".to_string(),
        settings: serde_json::Value::Null,
    }
}

/// 手写 mock Dispatch：按 exclude 顺序吐候选，与 retry_wiring 同构。
struct MockDispatch {
    candidates: Vec<Candidate>,
}

impl MockDispatch {
    fn new(candidates: Vec<Candidate>) -> Self {
        Self { candidates }
    }
}

impl Dispatch for MockDispatch {
    fn select(
        &self,
        group: &str,
        model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError> {
        self.candidates
            .iter()
            .find(|c| !exclude.contains(&c.unit.meta.key))
            .cloned()
            .ok_or_else(|| DispatchError::NoCandidate {
                group: group.to_string(),
                model: model.to_string(),
            })
    }

    fn report(&self, _unit_key: &str, _outcome: Result<u16, FailureClass>) {}
}

/// 脚本化 egress mock：按 url 里的候选标记返回分类错误（本文件只测失败路径，
/// 不返回成功体）。错误与真实 egress::classify_status 输出同构
/// （502 → retryable，400 → 非 retryable）。
struct ScriptedEgress {
    plans: Vec<(String, u16, bool)>,
    calls: AtomicUsize,
}

impl ScriptedEgress {
    fn new(plans: &[(&str, u16, bool)]) -> Self {
        Self {
            plans: plans
                .iter()
                .map(|(k, s, r)| (k.to_string(), *s, *r))
                .collect(),
            calls: AtomicUsize::new(0),
        }
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Egress for ScriptedEgress {
    fn execute<'a>(
        &'a self,
        url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<ForwardedResponse, contract::error::NormalizedError>,
                > + Send
                + 'a,
        >,
    > {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let (_, status, retryable) = self
            .plans
            .iter()
            .find(|(marker, ..)| url.contains(marker))
            .unwrap_or_else(|| panic!("ScriptedEgress: unexpected url {url}"));
        let (status, retryable) = (*status, *retryable);
        let err = NormalizedError {
            code: contract::error::code::UPSTREAM_ERROR,
            status,
            retryable,
            // 4xx 观测用例保持"不可切换渠道"语义（retryable 用例此字段无意义）
            channel_scoped: false,
            message: format!("upstream {status}"),
        };
        Box::pin(async move { Err(err) })
    }
}

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

/// 内存 sink mock：收集失败观测事件供断言。
#[derive(Default)]
struct VecSink(Mutex<Vec<UsageEventRecord>>);

impl SettleSink for VecSink {
    fn submit(&self, event: UsageEventRecord) {
        self.0.lock().unwrap().push(event);
    }
}

impl VecSink {
    fn events(&self) -> Vec<UsageEventRecord> {
        self.0.lock().unwrap().clone()
    }
}

/// 构造 ctx：body 决定流式意图（含 `"stream":true` 即流式），route 预置给
/// 单次模式用（with_retry 后 handle 忽略它）。
fn ctx_with_body(body: &'static [u8], route: Candidate) -> gateway_pipeline::RequestCtx {
    gateway_pipeline::RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(body)),
            client_ip: "127.0.0.1".parse().unwrap(),
            request_id: uuid::Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo {
            id: "tok-1".into(),
            group: "g".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("m".to_string()),
        route: Some(route),
        selected_channel_key: None,
        selected_channel_name: None,
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
    }
}

const NON_STREAM_BODY: &[u8] = b"{\"model\":\"m\"}";
const STREAM_BODY: &[u8] = b"{\"model\":\"m\",\"stream\":true}";

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
    let egress = Arc::new(ScriptedEgress::new(&[("upstream-c1", 502, true)]));
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
    assert_eq!(egress.calls(), 1);
}

// ---------- 用例 2: 预算耗尽 → 每次失败尝试各 1 条 ----------

#[tokio::test]
async fn retry_exhaustion_records_each_failed_attempt() {
    let dispatch = Arc::new(MockDispatch::new(vec![candidate("c1"), candidate("c2")]));
    let egress = Arc::new(ScriptedEgress::new(&[
        ("upstream-c1", 502, true),
        ("upstream-c2", 502, true),
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
    assert_eq!(egress.calls(), 2);
}

// ---------- 用例 3: Fatal 4xx 同样留痕, 且不触碰第二候选 ----------

#[tokio::test]
async fn fatal_4xx_records_one_event_and_short_circuits() {
    let dispatch = Arc::new(MockDispatch::new(vec![candidate("c1"), candidate("c2")]));
    let egress = Arc::new(ScriptedEgress::new(&[
        ("upstream-c1", 400, false),
        ("upstream-c2", 502, true),
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
    assert_eq!(egress.calls(), 1, "Fatal 不得触碰 c2");
}

// ---------- 用例 4: 流式请求失败 → is_stream=true ----------

#[tokio::test]
async fn streaming_request_failure_marks_is_stream_true() {
    let egress = Arc::new(ScriptedEgress::new(&[("upstream-c1", 502, true)]));
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
    let egress = Arc::new(ScriptedEgress::new(&[("upstream-c1", 502, true)]));
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
    assert_eq!(egress.calls(), 1);
}
