//! ForwardStage::with_retry 接线集成测试 — 重试循环 + 健康回报进生产路径。
//!
//! 验证三件事（对应 retry 编排契约）：
//! 1. 可重试失败后换候选：第二次 select 的 exclude 必须含已试候选 key，
//!    响应来自获胜候选，且每次尝试都向 Dispatch::report 回报健康；
//! 2. 预算耗尽：全部候选可重试失败 → 502 (Upstream "retry budget exhausted")；
//! 3. Fatal (retryable=false) 不换渠道：单次尝试即终止，上游状态码透传。
//!
//! Dispatch 用手写 mock（记录 select 的 exclude 与 report 调用序列）；
//! egress 用脚本化 mock（按候选 base_url 返回固定成功体 / 分类错误），
//! 与 forward/tests 现有 mock 风格一致（Egress trait 注入, 不发真实网络）。

use bytes::Bytes;
use contract::error::NormalizedError;
use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::health::FailureClass;
use dispatch::{Candidate, Dispatch, DispatchError, RetryPolicy};
use forward::ForwardStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::{Stage, StageError, StageOutcome, TokenInfo, UpstreamError};
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

/// 手写 mock Dispatch：按 exclude 顺序吐 c1 → c2，并把每次 select 收到的
/// exclude 集与每次 report 原样记录，供断言接线行为。
struct MockDispatch {
    candidates: Vec<Candidate>,
    selects: Mutex<Vec<Vec<String>>>,
    reports: Mutex<Vec<(String, Result<u16, FailureClass>)>>,
}

impl MockDispatch {
    fn new(candidates: Vec<Candidate>) -> Self {
        Self {
            candidates,
            selects: Mutex::new(Vec::new()),
            reports: Mutex::new(Vec::new()),
        }
    }
    fn selects(&self) -> Vec<Vec<String>> {
        self.selects.lock().unwrap().clone()
    }
    fn reports(&self) -> Vec<(String, Result<u16, FailureClass>)> {
        self.reports.lock().unwrap().clone()
    }
}

impl Dispatch for MockDispatch {
    fn select(
        &self,
        group: &str,
        model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError> {
        self.selects
            .lock()
            .unwrap()
            .push(exclude.iter().map(|s| s.to_string()).collect());
        self.candidates
            .iter()
            .find(|c| !exclude.contains(&c.unit.meta.key))
            .cloned()
            .ok_or_else(|| DispatchError::NoCandidate {
                group: group.to_string(),
                model: model.to_string(),
            })
    }

    fn report(&self, unit_key: &str, outcome: Result<u16, FailureClass>) {
        self.reports
            .lock()
            .unwrap()
            .push((unit_key.to_string(), outcome));
    }
}

/// 脚本化 egress mock：按 url 里的候选标记返回固定结果。
/// `Fail` 构造的 NormalizedError 与真实 egress::classify_status 的输出同构
/// （502 → retryable，400 → 非 retryable），forward 只见 trait 返回值。
enum Plan {
    Ok { body: &'static [u8] },
    Fail { status: u16, retryable: bool },
}

struct ScriptedEgress {
    plans: Vec<(String, Plan)>,
    calls: Mutex<Vec<String>>,
}

impl ScriptedEgress {
    fn new(plans: Vec<(&str, Plan)>) -> Self {
        Self {
            plans: plans.into_iter().map(|(k, p)| (k.to_string(), p)).collect(),
            calls: Mutex::new(Vec::new()),
        }
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
        self.calls.lock().unwrap().push(url.to_string());
        let (_, plan) = self
            .plans
            .iter()
            .find(|(marker, _)| url.contains(marker))
            .unwrap_or_else(|| panic!("ScriptedEgress: unexpected url {url}"));
        match plan {
            Plan::Ok { body } => {
                let body = Bytes::from_static(body);
                let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(body)]);
                Box::pin(async move {
                    Ok(ForwardedResponse::from_stream(
                        200,
                        "application/json",
                        stream,
                    ))
                })
            }
            Plan::Fail { status, retryable } => {
                let (status, retryable) = (*status, *retryable);
                let err = NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status,
                    retryable,
                    message: format!("upstream {status}"),
                };
                Box::pin(async move { Err(err) })
            }
        }
    }
}

/// 非流式请求 ctx。route 预置为获胜候选 c2 以验证 with_retry 后
/// handle **忽略** ctx.route（若读 route 会直接打 c2，不会先打 c1）。
fn ctx_with_route(route: Candidate) -> gateway_pipeline::RequestCtx {
    let meta = RequestMeta {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: http::HeaderMap::new(),
        body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"m\"}")),
        client_ip: "127.0.0.1".parse().unwrap(),
        request_id: uuid::Uuid::now_v7(),
        inbound_protocol: ProtocolKind::OpenAI,
    };
    gateway_pipeline::RequestCtx {
        request: meta,
        token: Some(TokenInfo {
            id: "tok-1".into(),
            group: "g".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("m".to_string()),
        route: Some(route),
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
    }
}

fn stage_with_retry(
    egress: Arc<ScriptedEgress>,
    dispatch: Arc<MockDispatch>,
    max_attempts: u32,
) -> ForwardStage {
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    ForwardStage::new(egress, adaptors).with_retry(dispatch, RetryPolicy { max_attempts })
}

// ---------- 用例 1: c1 可重试失败 → 换 c2 成功 ----------

#[tokio::test]
async fn retryable_failure_switches_candidate_and_reports_health() {
    // 行为断言：c1 返 502（retryable）→ 第二次 select 必须把 c1 key 放进
    // exclude；最终响应来自 c2；report 序列 = [c1: Err(Retryable), c2: Ok(200)]。
    let c1 = candidate("c1");
    let c2 = candidate("c2");
    let dispatch = Arc::new(MockDispatch::new(vec![c1.clone(), c2.clone()]));
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
            Plan::Ok {
                body: b"{\"from\":\"c2\"}",
            },
        ),
    ]));
    let stage = stage_with_retry(egress.clone(), dispatch.clone(), 3);

    // route 指向 c2: 若 handle 仍读 ctx.route 就会跳过 c1。
    let mut ctx = ctx_with_route(c2.clone());
    let outcome = stage.handle(&mut ctx).await.expect("c2 应成功");
    assert!(
        matches!(outcome, StageOutcome::Continue),
        "非流式成功应为 Continue, got {outcome:?}"
    );

    let up = ctx.upstream.expect("ctx.upstream 应被写入");
    assert_eq!(up.status, 200, "获胜尝试的状态码应落 ctx");
    assert_eq!(
        up.body,
        Bytes::from_static(b"{\"from\":\"c2\"}"),
        "响应体必须来自 c2 而不是 route 预置目标直接命中"
    );

    let selects = dispatch.selects();
    assert_eq!(selects.len(), 2, "c1 失败后应再选一次");
    assert!(selects[0].is_empty(), "首选无排除集");
    assert_eq!(
        selects[1],
        vec!["c1".to_string()],
        "第二次 select 的 exclude 必须含已试 c1 的 unit key"
    );

    assert_eq!(
        dispatch.reports(),
        vec![
            ("c1".to_string(), Err(FailureClass::Retryable)),
            ("c2".to_string(), Ok(200)),
        ],
        "每次尝试都要健康回报"
    );
}

// ---------- 用例 2: 全可重试失败 → 预算耗尽 502 ----------

#[tokio::test]
async fn all_retryable_failures_exhaust_budget_to_502() {
    // 两候选都 502；max_attempts=2 用尽后映射 RetriesExhausted → 502
    // Upstream("retry budget exhausted")，report 记录两条 Err(Retryable)。
    let c1 = candidate("c1");
    let c2 = candidate("c2");
    let dispatch = Arc::new(MockDispatch::new(vec![c1.clone(), c2.clone()]));
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
    let stage = stage_with_retry(egress.clone(), dispatch.clone(), 2);

    let mut ctx = ctx_with_route(c1.clone());
    let err = stage
        .handle(&mut ctx)
        .await
        .expect_err("预算耗尽必须报错而非静默成功");
    match err {
        StageError::Upstream(UpstreamError::Status { code, body_preview }) => {
            assert_eq!(code, 502, "RetriesExhausted 映射 502");
            // 预算耗尽时带上最后一个上游错误诊断 (ocr 采纳)，不再是纯静态串。
            let msg = String::from_utf8_lossy(&body_preview);
            assert!(
                msg.starts_with("retry budget exhausted") && msg.contains("upstream 502"),
                "body 应含耗尽说明与最后上游诊断, got {msg}"
            );
        }
        other => panic!("期望 Upstream 502, got {other:?}"),
    }
    assert!(ctx.upstream.is_none(), "失败路径不得写 ctx.upstream");

    assert_eq!(
        dispatch.reports(),
        vec![
            ("c1".to_string(), Err(FailureClass::Retryable)),
            ("c2".to_string(), Err(FailureClass::Retryable)),
        ],
        "每次失败尝试都要回报 Retryable"
    );
    assert_eq!(
        egress.calls.lock().unwrap().len(),
        2,
        "两次尝试各发一次上游"
    );
}

// ---------- 用例 3: c1 返 4xx (Fatal) → 不换渠道, 状态透传 ----------

#[tokio::test]
async fn fatal_4xx_does_not_switch_candidate() {
    // 4xx 非 retryable → 客户端/协议问题换渠道救不了：单次尝试即终止，
    // 上游 400 透传成 Upstream(400)，c2 完全不被触碰，report 只有一条 Fatal。
    let c1 = candidate("c1");
    let c2 = candidate("c2");
    let dispatch = Arc::new(MockDispatch::new(vec![c1.clone(), c2.clone()]));
    let egress = Arc::new(ScriptedEgress::new(vec![
        (
            "upstream-c1",
            Plan::Fail {
                status: 400,
                retryable: false,
            },
        ),
        ("upstream-c2", Plan::Ok { body: b"{}" }),
    ]));
    let stage = stage_with_retry(egress.clone(), dispatch.clone(), 3);

    let mut ctx = ctx_with_route(c1.clone());
    let err = stage
        .handle(&mut ctx)
        .await
        .expect_err("4xx 应映射为 Upstream 错误");
    match err {
        StageError::Upstream(UpstreamError::Status { code, body_preview }) => {
            assert_eq!(code, 400, "上游 4xx 状态码应透传, 不得吞成 502");
            assert_eq!(body_preview, b"upstream 400".to_vec());
        }
        other => panic!("期望 Upstream 400, got {other:?}"),
    }

    assert_eq!(dispatch.selects().len(), 1, "Fatal 不得触发第二次选路");
    assert_eq!(
        dispatch.reports(),
        vec![("c1".to_string(), Err(FailureClass::Fatal))],
        "Fatal 回报必须记录, 否则健康表永远空"
    );
    let calls = egress.calls.lock().unwrap();
    assert_eq!(calls.len(), 1, "只允许触碰 c1 一次");
    assert!(calls[0].contains("upstream-c1"), "c2 不得被请求");
}
