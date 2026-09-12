//! settle 集成测试 — 验证流式/非流式转发经扫描链后的自动结算与产物落地。
//!
//! 覆盖：
//! 1. StreamScanner 从 SSE data 帧提取 usage（结算的数据源）；
//! 2. `stream::finish` 挂了 price_table 才产出 UsageEvent，status/error
//!    由调用方注入（成功 200 / 中途断流 500 的库层证据）；
//! 3. 端到端接线：mock egress 返回带 usage 的非流式 200，注入 fake
//!    PriceTable + SettleSink 后，`ForwardStage::handle`（内部走
//!    commit_forwarded 唯一结算点）让 sink 恰好收到 1 条事件且计数、cost、
//!    group 归因全部正确。

use bytes::Bytes;
use contract::records::{RouteUnitRecord, SyncMeta, UsageEventRecord};
use forward::ForwardStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::stream::{SseContext, finish, pipe_chunk};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::{Stage, StageOutcome, TokenInfo};
use metering::SettleSink;
use metering::pricing::{ModelPrice, PriceTable};
use metering::scanner::StreamScanner;
use std::sync::{Arc, Mutex};

// ---------- 测试辅助 ----------

/// 固定价表：gpt-4o input $15/M, output $60/M；记录每次 lookup 的
/// (model, group)，验证 group 真的透传进了定价表。
#[derive(Default)]
struct RecordingPriceTable {
    looked: Mutex<Vec<(String, String)>>,
}

impl PriceTable for RecordingPriceTable {
    fn lookup(&self, model: &str, group: &str) -> Option<ModelPrice> {
        self.looked
            .lock()
            .unwrap()
            .push((model.to_string(), group.to_string()));
        Some(ModelPrice {
            input: 15.0,
            output: 60.0,
            cache: 0.0,
            group_multiplier: 1.0,
        })
    }
}

impl RecordingPriceTable {
    fn looked(&self) -> Vec<(String, String)> {
        self.looked.lock().unwrap().clone()
    }
}

/// 内存 sink mock（apps 侧实现的同型物）：收集结算事件供断言。
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

fn route(key: &str) -> SelectedRoute {
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
        base_url: "http://upstream.invalid".to_string(),
        upstream_model: "m".to_string(),
        provider_type: "openai".to_string(),
        settings: serde_json::Value::Null,
    }
}

/// 单次模式（无 with_retry）的最小 RequestCtx：route 预置、token 带 group。
fn ctx_single_shot() -> gateway_pipeline::RequestCtx {
    gateway_pipeline::RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"m\"}")),
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
        route: Some(route("c1")),
        selected_channel_key: None,
        selected_channel_name: None,
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
    }
}

/// 恒返 200 + 固定 body 的 mock egress（非流式）。
struct FixedEgress {
    body: &'static [u8],
}

impl Egress for FixedEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
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
        let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(self.body),
        )]);
        Box::pin(async move {
            Ok(ForwardedResponse::from_stream(
                200,
                "application/json",
                stream,
            ))
        })
    }
}

// ---------- scanner ----------

#[test]
fn stream_scanner_extracts_usage_for_settle() {
    let mut s = StreamScanner::new();
    s.push(&Bytes::from_static(
        b"data: {\"content\":\"hello\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
    ));
    let counts = s.finish(10);
    assert_eq!(counts.prompt, 10);
    assert_eq!(counts.completion, 5);
}

// ---------- stream::finish ----------

#[test]
fn sse_context_pipe_chunk_detects_first_token() {
    let mut ctx = SseContext::new();
    let out = pipe_chunk(
        &mut ctx,
        &Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
    );
    assert_eq!(
        out.passthrough,
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n")
    );
    assert_eq!(out.events.len(), 1);
}

fn priced_ctx() -> SseContext {
    let mut ctx = SseContext::new();
    ctx.user_key = "u1".into();
    ctx.token_key = "t1".into();
    ctx.channel_key = "ch1".into();
    ctx.public_model = "gpt-4o".into();
    ctx.upstream_model = "gpt-4o".into();
    ctx.price_table = Some(Arc::new(RecordingPriceTable::default()));
    pipe_chunk(
        &mut ctx,
        &Bytes::from_static(
            b"data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
        ),
    );
    ctx
}

/// 挂了 price_table：finish 真正结算并返回事件（不再丢弃产物）。
/// cost = (10×15 + 5×60)/1e6 × 500_000 = 225。
#[test]
fn finish_returns_settled_event_when_priced() {
    let (_end, counts, event) = finish(priced_ctx(), 200, None);
    assert_eq!((counts.prompt, counts.completion), (10, 5));
    let ev = event.expect("挂了定价表就必须结算");
    assert_eq!(ev.prompt_tokens, 10);
    assert_eq!(ev.completion_tokens, 5);
    assert_eq!(ev.cost, 225);
    assert_eq!(ev.user_key, "u1");
    assert_eq!(ev.token_key, "t1");
    assert_eq!(ev.channel_key, "ch1");
    assert_eq!(ev.status_code, 200);
    assert_eq!(ev.error, None);
}

/// 未挂 price_table：不计费，事件为 None（行为与未接计费时代一致）。
#[test]
fn finish_without_price_table_is_not_billed() {
    let ctx = SseContext::new();
    let (_end, _counts, event) = finish(ctx, 200, None);
    assert!(event.is_none(), "None 价格表 = 不计费");
}

/// 流中途出错：用已累积计数结算，status=500 + error 摘要（防泄漏账单）。
#[test]
fn finish_error_path_settles_accumulated_counts_with_500() {
    let (_end, counts, event) = finish(priced_ctx(), 500, Some("upstream reset"));
    assert_eq!(counts.completion, 5, "错误前产生的 token 必须入账");
    let ev = event.expect("错误路径同样结算");
    assert_eq!(ev.status_code, 500);
    assert_eq!(ev.error.as_deref(), Some("upstream reset"));
    assert_eq!(ev.cost, 225);
}

// ---------- ForwardStage 端到端接线（非流式） ----------

#[tokio::test]
async fn commit_forwarded_settles_non_streamed_into_sink() {
    // mock 上游: 200 + OpenAI 风格 usage（prompt 100 / completion 50）。
    let egress = Arc::new(FixedEgress {
        body: b"{\"choices\":[],\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":50}}",
    });
    let table = Arc::new(RecordingPriceTable::default());
    let sink = Arc::new(VecSink::default());
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = ForwardStage::new(egress, adaptors).with_price_table(table.clone(), sink.clone());

    let mut ctx = ctx_single_shot();
    let outcome = stage.handle(&mut ctx).await.expect("单次转发应成功");
    assert!(
        matches!(outcome, StageOutcome::Continue),
        "非流式成功应为 Continue, got {outcome:?}"
    );

    let events = sink.events();
    assert_eq!(events.len(), 1, "commit_forwarded 是唯一结算点, 恰好 1 条");
    let ev = &events[0];
    assert_eq!(ev.prompt_tokens, 100);
    assert_eq!(ev.completion_tokens, 50);
    // (100×15 + 50×60)/1e6 × 500_000 = 2_250
    assert_eq!(ev.cost, 2_250);
    assert_eq!(ev.status_code, 200);
    assert_eq!(ev.user_key, "tok-1");
    assert_eq!(ev.token_key, "tok-1");
    assert_eq!(ev.channel_key, "ch-c1");
    assert_eq!(ev.route_unit_key, "c1");
    // group 来自 ctx.token.group, 透传进 PriceTable::lookup
    assert_eq!(table.looked(), vec![("m".to_string(), "g".to_string())]);

    // 响应体照常写入 ctx.upstream（结算不干扰转发产物）
    let up = ctx.upstream.expect("ctx.upstream 应被写入");
    assert_eq!(up.status, 200);
    assert!(up.body.starts_with(b"{\"choices\""));
}

/// 未挂 pt/sink：保持不计费行为，非流式路径不产生任何事件。
#[tokio::test]
async fn unsettlement_path_without_price_table() {
    let egress = Arc::new(FixedEgress {
        body: b"{\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":50}}",
    });
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = ForwardStage::new(egress, adaptors); // 不调 with_price_table

    let mut ctx = ctx_single_shot();
    let outcome = stage.handle(&mut ctx).await.expect("单次转发应成功");
    assert!(matches!(outcome, StageOutcome::Continue));
    assert!(ctx.upstream.is_some(), "转发本身不受结算缺失影响");
}

/// 响应体无 usage：走估算兜底（prompt 由请求体估、completion 按响应字节粗估），
/// 事件仍产出——非流式路径不再静默漏账。
#[tokio::test]
async fn non_streamed_without_usage_falls_back_to_estimate() {
    let egress = Arc::new(FixedEgress {
        body: b"{\"choices\":[{\"message\":{\"content\":\"hi there\"}}]}",
    });
    let table = Arc::new(RecordingPriceTable::default());
    let sink = Arc::new(VecSink::default());
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    let stage = ForwardStage::new(egress, adaptors).with_price_table(table, sink.clone());

    let mut ctx = ctx_single_shot();
    stage.handle(&mut ctx).await.expect("单次转发应成功");

    let events = sink.events();
    assert_eq!(events.len(), 1, "缺 usage 也要估算入账");
    // prompt/completion 的具体估算值由 estimate 启发式决定，这里只要求非零
    assert!(
        events[0].completion_tokens > 0,
        "响应体长度粗估 completion 应为正"
    );
}
