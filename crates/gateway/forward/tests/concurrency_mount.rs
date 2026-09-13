//! ForwardStage 并发闸挂载（v2）集成测试 — 全局 Semaphore 在 forward 内部路径生效。
//!
//! 验证契约（并发闸挂载 v2）：
//! 1. `with_concurrency(2)` + 3 个并发请求（单次模式）：mock egress 延迟占住
//!    槽位制造竞争窗口，第 3 个请求在 `forward_task` 之前的 try_acquire 处被拒
//!    → 客户端侧得 `rate_limited`（429 语义），且**从未触碰 egress**；
//! 2. `with_concurrency(2)` + 重试模式（生产装配形状）：被拒请求经重试循环
//!    仍全被拒 → 预算耗尽时透传最后错误，终态仍携带 429 语义；
//! 3. `with_concurrency(4)` + 3 个并发：槽位充足 → 全部成功；
//! 4. 未调用 `with_concurrency`（`concurrency == None`）：行为与挂载前完全一致，
//!    3 个并发全部成功——既有路径零破坏。
//!
//! mock 风格同 `retry_wiring.rs`：egress 用 trait 注入的脚本化 mock（这里加
//! `tokio::time::sleep` 模拟上游延迟以占住槽位），不发真实网络。并发闸是
//! **全局**信号量（整体上限，不分渠道），换候选重试也撞同一把锁，故
//! Dispatch mock 恒返回同一候选（忽略 exclude）——与真实多候选场景下
//! "重试全被拒"的结局一致。

use bytes::Bytes;
use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::{Candidate, Dispatch, DispatchError, RetryPolicy};
use forward::ForwardStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::{Stage, StageError, StageOutcome, TokenInfo, UpstreamError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
        base_url: format!("http://upstream-{key}.invalid"),
        upstream_model: "m".to_string(),
        provider_type: "openai".to_string(),
        settings: serde_json::Value::Null,
    }
}

/// 恒返回同一候选的 Dispatch mock：并发闸是全局信号量，"换候选重试"依然
/// 撞同一把锁，exclude 语义与本测试无关；恒返回保证重试预算被真实耗尽
/// （若尊重 exclude，单候选在首次失败后就会 NoCandidate → 404，掩盖 429 语义）。
struct SingleDispatch {
    candidate: Candidate,
}

impl Dispatch for SingleDispatch {
    fn select(
        &self,
        _group: &str,
        _model: &str,
        _exclude: &[String],
    ) -> Result<Candidate, DispatchError> {
        Ok(self.candidate.clone())
    }

    fn report(&self, _unit_key: &str, _outcome: Result<u16, dispatch::health::FailureClass>) {}
}

/// 延迟 egress mock：sleep `delay` 模拟上游耗时（占住并发槽位制造竞争窗口），
/// 然后返回固定 200 成功体。记录每次调用，供断言"被拒请求从未触碰 egress"。
struct DelayedEgress {
    delay: Duration,
    calls: Mutex<Vec<String>>,
}

impl DelayedEgress {
    fn new(delay: Duration) -> Self {
        Self {
            delay,
            calls: Mutex::new(Vec::new()),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

impl Egress for DelayedEgress {
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
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            let body = Bytes::from_static(b"{\"ok\":true}");
            let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(body)]);
            Ok(ForwardedResponse::from_stream(
                200,
                "application/json",
                stream,
            ))
        })
    }
}

/// 非流式请求 ctx（形状同 retry_wiring.rs：route 预置候选，token 归因齐全）。
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
        selected_channel_key: None,
        selected_channel_name: None,
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
        drop_guards: Vec::new(),
    }
}

/// 并发发 3 个请求（tokio::spawn + 共享 Arc<ForwardStage>），收齐
/// `(stage 结果, 写入 ctx 的上游状态码)`。
async fn run_three(
    stage: Arc<ForwardStage>,
    candidate: Candidate,
) -> Vec<(Result<StageOutcome, StageError>, Option<u16>)> {
    let mut handles = Vec::new();
    for _ in 0..3 {
        let stage = Arc::clone(&stage);
        let candidate = candidate.clone();
        handles.push(tokio::spawn(async move {
            let mut ctx = ctx_with_route(candidate);
            let outcome = stage.handle(&mut ctx).await;
            let upstream_status = ctx.upstream.map(|u| u.status);
            (outcome, upstream_status)
        }));
    }
    futures_util::future::join_all(handles)
        .await
        .into_iter()
        .map(|r| r.expect("spawned test task panicked"))
        .collect()
}

/// 断言"恰好 n_ok 个成功、其余全部是 429 语义拒绝"。
fn assert_split(
    results: &[(Result<StageOutcome, StageError>, Option<u16>)],
    n_ok: usize,
    why: &str,
) {
    let ok = results
        .iter()
        .filter(|(r, _)| matches!(r, Ok(StageOutcome::Continue)))
        .count();
    assert_eq!(ok, n_ok, "{why}: 成功数应为 {n_ok}, results = {results:?}");
    assert_eq!(
        results.len() - ok,
        3 - n_ok,
        "{why}: 其余请求都应被并发闸拒绝"
    );
}

// ---------- 用例 1: 单次模式, 限 2 槽 + 3 并发 → 第 3 个 429 且未触碰 egress ----------

#[tokio::test]
async fn single_mode_concurrency_limit_rejects_third_request() {
    // 行为断言：两个请求各持一个槽位（egress sleep 200ms 占位），第三个请求
    // 在 forward_task 之前的 try_acquire 被拒 → StageError 携带 rate_limited
    // 429 语义；被拒请求不发上游（egress 调用数 == 2）。
    let egress = Arc::new(DelayedEgress::new(Duration::from_millis(200)));
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = Arc::new(ForwardStage::new(egress.clone(), adaptors).with_concurrency(2));
    let results = run_three(stage, candidate("c1")).await;

    assert_split(&results, 2, "限 2 槽时第 3 个请求必须被拒");
    for (outcome, _) in &results {
        if let Err(StageError::Upstream(UpstreamError::Status { code, body_preview })) = outcome {
            assert_eq!(*code, 429, "并发拒绝应携带 429 语义");
            let msg = String::from_utf8_lossy(body_preview);
            assert_eq!(
                msg, "concurrency limit reached",
                "单次路径直接透传并发闸错误消息"
            );
        }
    }
    assert_eq!(
        egress.call_count(),
        2,
        "被拒请求不得触碰 egress（acquire 在 forward_task 之前）"
    );
}

// ---------- 用例 2: 重试模式（生产装配形状）, 限 2 槽 + 3 并发 → 预算耗尽仍透传 429 ----------

#[tokio::test]
async fn retry_mode_concurrency_limit_exhausts_budget_with_429() {
    // 生产装配 = with_retry + with_concurrency。被拒请求在重试循环内每次
    // attempt 的 acquire 都失败（全局信号量换候选也满）→ RetriesExhausted →
    // 终态 Upstream 错误透传最后暂存错误的 429 状态码与并发闸消息。
    let egress = Arc::new(DelayedEgress::new(Duration::from_millis(200)));
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let dispatch = Arc::new(SingleDispatch {
        candidate: candidate("c1"),
    });
    let stage = Arc::new(
        ForwardStage::new(egress.clone(), adaptors)
            .with_retry(dispatch, RetryPolicy::default())
            .with_concurrency(2),
    );
    let results = run_three(stage, candidate("c1")).await;

    assert_split(&results, 2, "重试模式下全局闸同样只放行 2 个");
    for (outcome, _) in &results {
        if let Err(StageError::Upstream(UpstreamError::Status { code, body_preview })) = outcome {
            assert_eq!(*code, 429, "预算耗尽应透传最后错误（并发闸 429）的状态码");
            let msg = String::from_utf8_lossy(body_preview);
            assert!(
                msg.starts_with("retry budget exhausted")
                    && msg.contains("concurrency limit reached"),
                "终态消息应含耗尽说明与并发闸诊断, got {msg}"
            );
        }
    }
    assert_eq!(egress.call_count(), 2, "被拒请求的重试全程都不触碰 egress");
}

// ---------- 用例 3: 限 4 槽 + 3 并发 → 全部成功 ----------

#[tokio::test]
async fn concurrency_headroom_allows_all_requests() {
    // 槽位(4)多于并发数(3)：acquire 全部成功，无拒绝路径介入。
    let egress = Arc::new(DelayedEgress::new(Duration::from_millis(100)));
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let dispatch = Arc::new(SingleDispatch {
        candidate: candidate("c1"),
    });
    let stage = Arc::new(
        ForwardStage::new(egress.clone(), adaptors)
            .with_retry(dispatch, RetryPolicy::default())
            .with_concurrency(4),
    );
    let results = run_three(stage, candidate("c1")).await;

    assert_split(&results, 3, "槽位充足时 3 个并发都应成功");
    for upstream_status in results.iter().map(|(_, s)| s) {
        assert_eq!(*upstream_status, Some(200), "成功请求应写回上游 200");
    }
    assert_eq!(egress.call_count(), 3, "每个请求各发一次上游");
}

// ---------- 用例 4: 未挂载（concurrency == None）→ 行为与挂载前一致 ----------

#[tokio::test]
async fn without_concurrency_mount_behaviour_unchanged() {
    // with_concurrency 未调用：acquire 直接跳过，3 个并发全部成功——
    // 既有测试/生产路径（builder 未调时）零破坏的直接证据。
    let egress = Arc::new(DelayedEgress::new(Duration::from_millis(100)));
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let dispatch = Arc::new(SingleDispatch {
        candidate: candidate("c1"),
    });
    let stage = Arc::new(
        ForwardStage::new(egress.clone(), adaptors).with_retry(dispatch, RetryPolicy::default()),
    );
    let results = run_three(stage, candidate("c1")).await;

    assert_split(&results, 3, "未挂载并发闸时行为必须与挂载前完全一致");
    assert_eq!(egress.call_count(), 3);
}
