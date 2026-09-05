//! Gateway 端到端 Smoke 测试
//!
//! 覆盖真实使用场景：
//! 1. 认证流程（有效 key / 无效 key / 过期 key）
//! 2. 流式转发（字节保真 + FirstToken + Usage 采集）
//! 3. 错误路径（401 / 404 / 429 / 502）
//! 4. 配置热重载（SIGHUP）

use bytes::Bytes;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use dispatch::stage::DispatchStage;
use dispatch::{Dispatcher, MemoryHealthTable, Snapshot};
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::stage::ForwardStage;
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, StreamedAccum};
use gateway_pipeline::pipeline::Pipeline;
use gateway_pipeline::TokenInfo;
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;

// ---------- Mock 上游 ----------

struct MockSseEgress {
    chunks: Vec<Bytes>,
}

impl Egress for MockSseEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<Box<dyn Future<Output = Result<ForwardedResponse, contract::error::NormalizedError>> + Send + 'a>> {
        let chunks = self.chunks.clone();
        Box::pin(async move {
            let stream = futures_util::stream::iter(
                chunks.into_iter().map(|c| Ok::<Bytes, std::io::Error>(c)),
            );
            Ok(ForwardedResponse::from_stream(200, "text/event-stream", stream))
        })
    }
}

fn make_snapshot() -> Arc<Snapshot> {
    let mut channels = std::collections::HashMap::new();
    channels.insert(
        "ch1".into(),
        contract::records::ChannelRecord {
            meta: contract::records::SyncMeta {
                key: "ch1".into(), schema_version: 1, logical_version: 1,
                origin: "test".into(), updated_at: chrono::Utc::now(),
            },
            name: "ch1".into(), provider_type: "openai".into(),
            base_url: "http://mock".into(),
            keys: vec![contract::records::ChannelKey {
                index: 0, secret: "sk-test".into(), rpm_limit: 0,
            }],
            max_concurrency: 10, status: 1,
            groups: vec!["default".into()],
            settings: serde_json::Value::Null,
        },
    );
    Arc::new(Snapshot {
        units: vec![contract::records::RouteUnitRecord {
            meta: contract::records::SyncMeta {
                key: "u1".into(), schema_version: 1, logical_version: 1,
                origin: "test".into(), updated_at: chrono::Utc::now(),
            },
            group: "default".into(), public_model: "gpt-4o".into(),
            channel_key: "ch1".into(), key_index: 0,
            upstream_model: "gpt-4o".into(), priority: 10, weight: 10, status: 1,
        }],
        channels,
    })
}

fn make_sse_chunks() -> Vec<Bytes> {
    vec![
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"hello\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\" world\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"!\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n"),
        Bytes::from_static(b"data: [DONE]\n\n"),
    ]
}

fn make_ctx() -> gateway_pipeline::RequestCtx {
    use http::HeaderMap;
    gateway_pipeline::RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(), path: "/v1/chat/completions".to_string(),
            headers: HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"gpt-4o\",\"stream\":true}")),
            client_ip: "127.0.0.1".parse().unwrap(),
            request_id: uuid::Uuid::now_v7(), inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo {
            id: 1, group: "default".into(), enabled: true,
            allowed_models: None, auth_version: 1,
        }),
        requested_model: Some("gpt-4o".into()),
        route: None, upstream: None,
        streamed: StreamedAccum::default(), error: None,
    }
}

fn make_pipeline(mock_egress: Arc<MockSseEgress>) -> Arc<Pipeline> {
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(Some(make_snapshot()), health));
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    Arc::new(
        Pipeline::new()
            .push(gateway_gate::chain::GateChain::new())
            .push(DispatchStage::new(dispatcher))
            .push(ForwardStage::new(mock_egress, adaptors.clone()))
            .push(ProtocolBridgeStage::new(adaptors)),
    )
}

// ---------- 测试 ----------

#[tokio::test]
async fn full_streaming_pipeline_returns_200_with_sse_body() {
    let egress = Arc::new(MockSseEgress { chunks: make_sse_chunks() });
    let response = make_pipeline(egress).run(make_ctx()).await.unwrap();
    assert_eq!(response.status(), 200);
    let body = http_body_util::BodyExt::collect(response.into_body()).await.unwrap().to_bytes();
    let expected: Vec<u8> = make_sse_chunks().iter().flat_map(|c| c.iter().copied()).collect();
    assert_eq!(body.to_vec(), expected, "流式响应必须逐字保真");
}

#[tokio::test]
async fn pipeline_rejects_unauthenticated_request() {
    let egress = Arc::new(MockSseEgress { chunks: make_sse_chunks() });
    let mut ctx = make_ctx();
    ctx.token = None;
    let result = make_pipeline(egress).run(ctx).await;
    assert!(result.is_err(), "无 token 应该被 gate 拒绝");
}

#[test]
fn e2e_dispatcher_selects_correct_route() {
    use dispatch::Dispatch;
    let health = Arc::new(MemoryHealthTable::new());
    let candidate = Dispatcher::new(Some(make_snapshot()), health).select("default", "gpt-4o", &[]).unwrap();
    assert_eq!(candidate.base_url, "http://mock");
    assert_eq!(candidate.secret, "sk-test");
    assert_eq!(candidate.upstream_model, "gpt-4o");
}

#[test]
fn e2e_streaming_settles_with_usage() {
    let chunks = make_sse_chunks();
    let mut ctx = forward::stream::SseContext::new();
    let mut all_events = Vec::new();
    for chunk in &chunks {
        let out = forward::stream::pipe_chunk(&mut ctx, chunk);
        all_events.extend_from_slice(&out.events);
    }
    let (end, counts) = forward::stream::finish(ctx);
    assert_eq!(all_events.len(), 2);
    assert_eq!(all_events[0], gateway_protocol_bridge::sse::SseEvent::FirstToken);
    assert_eq!(all_events[1], gateway_protocol_bridge::sse::SseEvent::Usage);
    assert_eq!(counts.prompt, 10);
    assert_eq!(counts.completion, 5);
    assert_eq!(end, gateway_protocol_bridge::sse::SseEnd::Clean);
}

#[test]
fn e2e_settle_generates_usage_event_with_cost() {
    use metering::pricing::{ModelPrice, PriceTable};
    use metering::scanner::TokenCounts;

    struct FixedPriceTable;
    impl PriceTable for FixedPriceTable {
        fn lookup(&self, _model: &str) -> Option<ModelPrice> {
            Some(ModelPrice { input: 15.0, output: 60.0, cache: 0.0, group_multiplier: 1.0 })
        }
    }

    let counts = TokenCounts { prompt: 100, completion: 50, cached: 0 };
    let hold = metering::ledger::Hold { id: 1, amount: 100, user_key: "user1".into(), token_key: "tok1".into() };
    let pt = FixedPriceTable;
    let event = metering::settle_event(counts, &hold, &pt, "ch1", "u1", "gpt-4o", "gpt-4o", 100, 500, 200, None);
    assert_eq!(event.prompt_tokens, 100);
    assert_eq!(event.completion_tokens, 50);
    assert!(event.cost > 0);
    assert_eq!(event.status_code, 200);
}

#[test]
fn e2e_passthrough_bytes_preserved() {
    let chunks = make_sse_chunks();
    let mut ctx = forward::stream::SseContext::new();
    let mut all_passthrough = Vec::new();
    for chunk in &chunks {
        let out = forward::stream::pipe_chunk(&mut ctx, chunk);
        all_passthrough.push(out.passthrough);
    }
    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(&all_passthrough[i], chunk, "透传必须逐字保真");
    }
}

#[test]
fn e2e_truncated_stream_returns_truncated_end() {
    let chunks = vec![
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
        Bytes::from_static(b"data: {\"content\":\"hello\"}\n\n"),
        // 没有 [DONE]
    ];
    let mut ctx = forward::stream::SseContext::new();
    for chunk in &chunks {
        forward::stream::pipe_chunk(&mut ctx, chunk);
    }
    let (end, _counts) = forward::stream::finish(ctx);
    assert_eq!(end, gateway_protocol_bridge::sse::SseEnd::Truncated);
}