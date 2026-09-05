//! 门闸 + 调度 + 转发 + 协议出口的组装集成测试。
//!
//! 验证 pipeline 最短链路: GateChain(空) → DispatchStage → ForwardStage
//! → ProtocolBridgeStage 串成 Pipeline 后, 一个请求能走完并产出响应。
//!
//! 用 mock egress 返回假上游响应, 不发起真实网络 (Egress trait 注入)。

use bytes::Bytes;
use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::stage::DispatchStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::stage::ForwardStage;
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, StreamedAccum};
use gateway_pipeline::pipeline::Pipeline;
use gateway_protocol_bridge::stage::ProtocolBridgeStage;
use std::sync::Arc;

// ---------- mock egress ----------

/// 假上游: 总是返回 200 + 固定 JSON body。
struct MockEgress;

impl Egress for MockEgress {
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
        Box::pin(async move {
            let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
                Bytes::from_static(b"{\"choices\":[]}"),
            )]);
            Ok(ForwardedResponse::from_stream(
                200,
                "application/json",
                stream,
            ))
        })
    }
}

// ---------- assembly ----------

fn mk_ctx() -> gateway_pipeline::RequestCtx {
    let meta = RequestMeta {
        method: "POST".to_string(),
        path: "/v1/chat/completions".to_string(),
        headers: http::HeaderMap::new(),
        body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"gpt-4o\"}")),
        client_ip: "127.0.0.1".parse().unwrap(),
        request_id: uuid::Uuid::now_v7(),
        inbound_protocol: ProtocolKind::OpenAI,
    };
    gateway_pipeline::RequestCtx {
        request: meta,
        token: Some(gateway_pipeline::TokenInfo {
            id: 1,
            group: "default".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("gpt-4o".to_string()),
        route: None,
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
    }
}

fn mk_snapshot() -> Arc<dispatch::Snapshot> {
    let unit = RouteUnitRecord {
        meta: SyncMeta {
            key: "u1".into(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".into(),
            updated_at: chrono::Utc::now(),
        },
        group: "default".into(),
        public_model: "gpt-4o".into(),
        channel_key: "ch1".into(),
        key_index: 0,
        upstream_model: "gpt-4o".into(),
        priority: 10,
        weight: 10,
        status: 1,
    };
    let channel = ChannelRecord {
        meta: SyncMeta {
            key: "ch1".into(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".into(),
            updated_at: chrono::Utc::now(),
        },
        name: "ch1".into(),
        provider_type: "openai".into(),
        base_url: "https://upstream.example".into(),
        keys: vec![ChannelKey {
            index: 0,
            secret: "sk-test".into(),
            rpm_limit: 0,
        }],
        max_concurrency: 10,
        status: 1,
        groups: vec!["default".into()],
        settings: serde_json::Value::Null,
    };
    let mut channels = std::collections::HashMap::new();
    channels.insert("ch1".into(), channel);
    Arc::new(dispatch::Snapshot {
        units: vec![unit],
        channels,
    })
}

// ---------- tests ----------

#[tokio::test]
async fn full_chain_runs_to_protocol_bridge() {
    // 组装四段链路
    let gate = gateway_gate::chain::GateChain::new(); // 空 gate → 直通
    let dispatcher = Arc::new(dispatch::Dispatcher::new(
        Some(mk_snapshot()),
        Arc::new(dispatch::health::MemoryHealthTable::new()),
    ));
    let dispatch_stage = DispatchStage::new(dispatcher);
    let forward_stage = ForwardStage::new(Arc::new(MockEgress), Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::with_defaults()));
    let bridge = ProtocolBridgeStage::new(Arc::new(
        gateway_protocol_bridge::adaptor::AdaptorRegistry::with_defaults(),
    ));

    let pipe = Pipeline::new()
        .push(gate)
        .push(dispatch_stage)
        .push(forward_stage)
        .push(bridge);

    let ctx = mk_ctx();
    let result = pipe.run(ctx).await;
    let resp = result.expect("full chain should produce a response");
    assert_eq!(resp.status(), 200, "mock upstream 200 应透传");
}

#[tokio::test]
async fn chain_stops_at_gate_on_rejection() {
    // 带拒绝的 gate (AuthGate 无 token 快照 → InvalidApiKey 短路到 401)
    let tokens = Arc::new(arc_swap::ArcSwap::from_pointee(
        gateway_gate::snapshot::TokenSnapshot::default(),
    ));
    let auth = gateway_gate::auth::AuthGate::new(tokens);
    let gate = gateway_gate::chain::GateChain::new().push(auth);

    let pipe = Pipeline::new().push(gate);
    let ctx = mk_ctx();
    let resp = pipe.run(ctx).await.expect("gate short-circuit is Ok");
    assert_eq!(resp.status(), 401, "无 key → AuthGate 拒绝");
}
