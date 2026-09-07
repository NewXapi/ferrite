//! ForwardStage::with_proxies 接线测试 — 验证租约 Client 替换注入 egress。
//!
//! 契约：
//! - proxies = None → 使用注入的 mock egress
//! - proxies = Some(manager with node) → 使用租约 Client（mock 不被调用）
//! - proxies = Some(manager no node) → 使用直连 Client（mock 不被调用）

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use bytes::Bytes;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::stage::ForwardStage;
use gateway_pipeline::TokenInfo;
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, SelectedRoute, StreamedAccum};
use gateway_pipeline::pipeline::Pipeline;
use gateway_proxy::manager::ProxyManager;
use gateway_proxy::node::ProxyScheme;
use gateway_proxy::pool::{ProxyNode, ProxySnapshot};

const STREAM: StreamedAccum = StreamedAccum {
    prompt_tokens: 0,
    completion_tokens: 0,
    first_token_at: None,
};

// ---------- spy egress：记录是否被调用 ----------

struct SpyEgress {
    call_count: Arc<AtomicUsize>,
}

impl Egress for SpyEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<ForwardedResponse, contract::error::NormalizedError>>
                + Send
                + 'a,
        >,
    > {
        self.call_count.fetch_add(1, Ordering::SeqCst);
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

fn make_ctx(route: Option<SelectedRoute>) -> gateway_pipeline::RequestCtx {
    gateway_pipeline::RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"gpt-4o\"}")),
            client_ip: "127.0.0.1".parse().unwrap(),
            request_id: uuid::Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo {
            id: 1,
            group: "default".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("gpt-4o".to_string()),
        route,
        upstream: None,
        streamed: STREAM,
        error: None,
    }
}

fn route_for(key: &str, base: &str) -> SelectedRoute {
    let unit = contract::records::RouteUnitRecord {
        meta: contract::records::SyncMeta {
            key: format!("unit-{key}"),
            schema_version: contract::SCHEMA_VERSION,
            logical_version: 1,
            origin: "test".into(),
            updated_at: chrono::Utc::now(),
        },
        group: "default".into(),
        public_model: "gpt-4o".into(),
        channel_key: key.into(),
        key_index: 0,
        upstream_model: "gpt-4o".into(),
        priority: 10,
        weight: 10,
        status: 1,
    };
    SelectedRoute {
        unit,
        secret: "sk-test".into(),
        base_url: base.into(),
        upstream_model: "gpt-4o".into(),
        provider_type: "openai".into(),
        settings: serde_json::Value::Null,
    }
}

fn make_pipeline_with(egress: Arc<SpyEgress>, proxies: Option<Arc<ProxyManager>>) -> Arc<Pipeline> {
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::with_defaults());
    let mut stage = ForwardStage::new(egress, adaptors.clone());
    if let Some(proxies) = proxies {
        stage = stage.with_proxies(proxies);
    }
    Arc::new(Pipeline::new().push(stage).push(
        gateway_protocol_bridge::stage::ProtocolBridgeStage::new(adaptors),
    ))
}

// ---------- tests ----------

/// proxies = None → 注入的 mock egress 被调用。
#[tokio::test]
async fn proxies_none_uses_injected_egress() {
    let spy = Arc::new(SpyEgress {
        call_count: Arc::new(AtomicUsize::new(0)),
    });
    let pipe = make_pipeline_with(spy.clone(), None);
    let ctx = make_ctx(Some(route_for("ch", "https://upstream.example")));
    let resp = pipe.run(ctx).await.expect("pipeline should succeed");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        spy.call_count.load(Ordering::SeqCst),
        1,
        "mock egress should be called"
    );
}

/// proxies = Some(manager with node) → 租约 Client 替换 mock；mock 不被调用。
/// 节点指向不可达端口，请求失败，但失败来自租约 Client 而非 mock。
#[tokio::test]
async fn proxies_with_node_uses_leased_client() {
    let spy = Arc::new(SpyEgress {
        call_count: Arc::new(AtomicUsize::new(0)),
    });
    let manager = Arc::new(ProxyManager::new());
    manager.install(ProxySnapshot {
        nodes: vec![ProxyNode {
            id: 1,
            scheme: ProxyScheme::Http,
            host: "127.0.0.1".into(),
            port: 1,
            auth: None,
            channel_keys: vec!["ch".into()],
            priority: 10,
        }],
    });
    let pipe = make_pipeline_with(spy.clone(), Some(manager));
    let ctx = make_ctx(Some(route_for("ch", "https://upstream.example")));
    let _ = pipe.run(ctx).await;
    assert_eq!(
        spy.call_count.load(Ordering::SeqCst),
        0,
        "leased client should replace mock egress"
    );
}

/// proxies = Some(manager no node) → 直连 Client 替换 mock；mock 不被调用。
#[tokio::test]
async fn proxies_without_node_uses_direct_client() {
    let spy = Arc::new(SpyEgress {
        call_count: Arc::new(AtomicUsize::new(0)),
    });
    let manager = Arc::new(ProxyManager::new());
    let pipe = make_pipeline_with(spy.clone(), Some(manager));
    let ctx = make_ctx(Some(route_for("missing", "https://upstream.example")));
    let _ = pipe.run(ctx).await;
    assert_eq!(
        spy.call_count.load(Ordering::SeqCst),
        0,
        "direct client should replace mock egress"
    );
}
