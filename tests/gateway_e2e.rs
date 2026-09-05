//! Gateway 端到端集成测试
//!
//! 完整链路: HTTP Request → Pipeline → GateChain → DispatchStage → ForwardStage → SseScanner → StreamScanner → Response

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
            let stream = futures_util::stream::iter(chunks.into_iter().map(|c| Ok::<Bytes, std::io::Error>(c)));
            Ok(ForwardedResponse::from_stream(200, "text/event-stream", stream))
        })
    }
}

fn make_snapshot() -> Arc<Snapshot> {
    let mut channels = std::collections::HashMap::new();
    channels.insert("ch1".into(), contract::records::ChannelRecord {
        meta: contract::records::SyncMeta { key: "ch1".into(), schema_version: 1, logical_version: 1, origin: "test".into(), updated_at: chrono::Utc::now() },
        name: "ch1".into(), provider_type: "openai".into(), base_url: "http://mock".into(),
        keys: vec![contract::records::ChannelKey { index: 0, secret: "sk-test".into(), rpm_limit: 0 }],
        max_concurrency: 10, status: 1, groups: vec!["default".into()], settings: serde_json::Value::Null,
    });
    Arc::new(Snapshot {
        units: vec![contract::records::RouteUnitRecord {
            meta: contract::records::SyncMeta { key: "u1".into(), schema_version: 1, logical_version: 1, origin: "test".into(), updated_at: chrono::Utc::now() },
            group: "default".into(), public_model: "gpt-4o".into(), channel_key: "ch1".into(),
            key_index: 0, upstream_model: "gpt-4o".into(), priority: 10, weight: 10, status: 1,
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
            client_ip: "127.0.0.1".parse().unwrap(), request_id: uuid::Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo { id: 1, group: "default".into(), enabled: true, allowed_models: None, auth_version: 1 }),
        requested_model: Some("gpt-4o".into()),
        route: None, upstream: None, streamed: StreamedAccum::default(), error: None,
    }
}

fn make_pipeline(mock_egress: Arc<MockSseEgress>) -> Arc<Pipeline> {
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(Some(make_snapshot()), health));
    let adaptors = Arc::new(AdaptorRegistry::with_defaults());
    Arc::new(Pipeline::new()
        .push(gateway_gate::chain::GateChain::new())
        .push(DispatchStage::new(dispatcher))
        .push(ForwardStage::new(mock_egress, adaptors.clone()))
        .push(ProtocolBridgeStage::new(adaptors)))
}

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
    assert!(make_pipeline(egress).run(make_ctx()).await.is_ok());
    assert!(make_pipeline(Arc::new(MockSseEgress { chunks: make_sse_chunks() })).run(ctx).await.is_err(), "无 token 应该被 gate 拒绝");
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
