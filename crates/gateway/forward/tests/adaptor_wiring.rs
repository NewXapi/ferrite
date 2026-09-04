//! forward↔protocol-bridge 接线测试：验证请求/响应经过 adaptor 转换。

use bytes::Bytes;
use contract::error::NormalizedError;
use parking_lot::Mutex;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::pipeline::{forward_once, merge_headers};
use forward::ForwardTask;
use gateway_protocol_bridge::adaptor::{AdaptorError, AdaptorRegistry, Codec, Protocol};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use contract::records::RouteUnitRecord;

// ---------- mock Egress ----------

/// 假上游：总是返回 200 + 固定 body。
struct MockEgress {
    /// 记录收到的请求体。
    captured_body: Arc<Mutex<Option<Bytes>>>,
}

impl Egress for MockEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<
        Box<dyn std::future::Future<Output = Result<ForwardedResponse, NormalizedError>> + Send + 'a>,
    > {
        *self.captured_body.lock() = Some(body.clone());
        let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(b"{\\\"gems\\\":[]}"),
        )]);
        Box::pin(async move {
            Ok(ForwardedResponse::from_stream(200, "application/json", stream))
        })
    }
}

// ---------- spy Codec ----------

/// 记录是否被调用，并对 request/response 做可观测的 transform。
struct SpyCodec {
    name: &'static str,
    source: Protocol,
    target: Protocol,
    request_called: Arc<AtomicBool>,
    response_called: Arc<AtomicBool>,
}

impl Codec for SpyCodec {
    fn source(&self) -> Protocol {
        self.source
    }
    fn target(&self) -> Protocol {
        self.target
    }
    fn adapt_request(&self, body: Bytes) -> Result<Bytes, AdaptorError> {
        self.request_called.store(true, Ordering::SeqCst);
        // 可观测 transform：注入 "_proxied":true。
        let mut v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        v["_proxied"] = serde_json::json!(true);
        Ok(serde_json::to_vec(&v).unwrap().into())
    }
    fn adapt_response(&self, chunk: Bytes) -> Result<Vec<Bytes>, AdaptorError> {
        self.response_called.store(true, Ordering::SeqCst);
        // 可观测 transform：每条 chunk 加前缀 "[adapter]"。
        let s = String::from_utf8_lossy(&chunk);
        Ok(vec![Bytes::from(format!("[adapter]{s}"))])
    }
}

// ---------- helpers ----------

fn mk_registry(codec: Arc<dyn Codec>) -> AdaptorRegistry {
    let mut reg = AdaptorRegistry::new();
    reg.register(codec);
    reg
}

fn mk_task(stream: bool) -> ForwardTask {
    ForwardTask {
        candidate: dispatch::candidate::Candidate {
            unit: RouteUnitRecord {
                meta: contract::records::SyncMeta {
                    key: "u1".into(),
                    schema_version: 1,
                    logical_version: 1,
                    origin: "test".into(),
                    updated_at: chrono::Utc::now(),
                },
                group: "default".into(),
                public_model: "m".into(),
                channel_key: "ch1".into(),
                key_index: 0,
                upstream_model: "m".into(),
                priority: 10,
                weight: 10,
                status: 1,
            },
            secret: "s".into(),
            base_url: "https://upstream.example".into(),
            upstream_model: "m".into(),
        },
        path: "/v1/chat/completions".to_string(),
        headers: vec![],
        body: Bytes::from_static(b"{\"model\":\"gpt-4o\",\"messages\":[]}"),
        stream,
        provider_type: "gemini".into(),
        extra_headers: vec![],
    }
}

// ---------- tests ----------

#[tokio::test]
async fn forward_calls_adapt_request_for_non_stream() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress { captured_body: captured.clone() };
    let request_called = Arc::new(AtomicBool::new(false));
    let response_called = Arc::new(AtomicBool::new(false));

    let codec: Arc<dyn Codec> = Arc::new(SpyCodec {
        name: "gemini_spy",
        source: Protocol::OpenAi,
        target: Protocol::Gemini,
        request_called: request_called.clone(),
        response_called: response_called.clone(),
    });
    let reg = mk_registry(codec);

    let task = mk_task(false);
    let _ = forward_once(&task, &egress, &reg, &Timeouts::default())
        .await
        .expect("forward 应成功");

    // adapt_request 被调用
    assert!(request_called.load(Ordering::SeqCst), "adapt_request 应被调用");
    // 上游收到了经过 transform 的 body（含 _proxied 字段）
    let sent_body = captured.lock().clone().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&sent_body).unwrap();
    assert!(v["_proxied"].as_bool().unwrap_or(false), "请求体应被 adapt_request 转换");
}

#[tokio::test]
async fn forward_calls_adapt_response_for_streaming() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress { captured_body: captured.clone() };
    let request_called = Arc::new(AtomicBool::new(false));
    let response_called = Arc::new(AtomicBool::new(false));

    let codec: Arc<dyn Codec> = Arc::new(SpyCodec {
        name: "gemini_spy",
        source: Protocol::OpenAi,
        target: Protocol::Gemini,
        request_called: request_called.clone(),
        response_called: response_called.clone(),
    });
    let reg = mk_registry(codec);

    let task = mk_task(true);
    let forwarded = forward_once(&task, &egress, &reg, &Timeouts::default())
        .await
        .expect("forward 应成功");

    // 消费响应流
    use futures_util::StreamExt;
    let mut body = forwarded.body;
    let first = body.next().await.unwrap().unwrap();

    // adapt_request 被调用
    assert!(request_called.load(Ordering::SeqCst));
    // adapt_response 被调用
    assert!(response_called.load(Ordering::SeqCst), "adapt_response 应被调用");
    // 响应经过 transform（前缀 [adapter]）
    let s = String::from_utf8_lossy(&first);
    assert!(s.contains("[adapter]"), "响应应被 adapt_response 转换: got {s}");
}

#[tokio::test]
async fn forward_passthrough_when_no_adaptor() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress { captured_body: captured.clone() };

    // 空 registry → 透传
    let reg = AdaptorRegistry::new();

    let task = mk_task(false);
    let _ = forward_once(&task, &egress, &reg, &Timeouts::default())
        .await
        .expect("透传应成功");

    // 上游收到原始 body（没有 _proxied 注入）
    let sent_body = captured.lock().clone().unwrap();
    assert_eq!(sent_body, task.body, "无 adaptor 时应原样转发");
}
