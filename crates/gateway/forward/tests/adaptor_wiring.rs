//! forward↔protocol-bridge 接线测试：验证请求/响应经过 adaptor 转换。

use bytes::Bytes;
use contract::error::NormalizedError;
use contract::records::RouteUnitRecord;
use forward::ForwardTask;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::pipeline::forward_once;
use gateway_protocol_bridge::adaptor::{AdaptorError, AdaptorRegistry, Codec, Protocol};
use parking_lot::Mutex;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

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
        Box<
            dyn std::future::Future<Output = Result<ForwardedResponse, NormalizedError>>
                + Send
                + 'a,
        >,
    > {
        *self.captured_body.lock() = Some(body.clone());
        let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
            Bytes::from_static(b"{\\\"gems\\\":[]}"),
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

// ---------- spy Codec ----------

/// 记录是否被调用，并对 request/response 做可观测的 transform。
struct SpyCodec {
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

/// 注册一个 codec。
///
/// 只覆盖 `(source, target)` 一个方向 —— 与内建注册表一致：`Codec` 有向，
/// 请求与响应各查自己方向。
fn mk_registry(codec: Arc<dyn Codec>) -> AdaptorRegistry {
    let mut reg = AdaptorRegistry::new();
    reg.register(codec);
    reg
}

/// 注册请求与响应两个方向的 spy codec，模拟真实注册表的成对登记。
fn mk_bidi_registry(
    request_called: Arc<AtomicBool>,
    response_called: Arc<AtomicBool>,
) -> AdaptorRegistry {
    let mut reg = AdaptorRegistry::new();
    reg.register(Arc::new(SpyCodec {
        source: Protocol::OpenAi,
        target: Protocol::Gemini,
        request_called: request_called.clone(),
        response_called: response_called.clone(),
    }));
    reg.register(Arc::new(SpyCodec {
        source: Protocol::Gemini,
        target: Protocol::OpenAi,
        request_called,
        response_called,
    }));
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
            provider_type: "gemini".into(),
            settings: serde_json::Value::Null,
        },
        path: "/v1/chat/completions".to_string(),
        headers: vec![],
        // 公开名与上游真名一致（常见情形）：别名改写是 no-op，
        // 这样本文件测的就只是 adaptor 转换，不掺入 model 改写。
        body: Bytes::from_static(b"{\"model\":\"m\",\"messages\":[]}"),
        stream,
        provider_type: "gemini".into(),
        extra_headers: vec![],
    }
}

// ---------- tests ----------

#[tokio::test]
async fn forward_calls_adapt_request_for_non_stream() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress {
        captured_body: captured.clone(),
    };
    let request_called = Arc::new(AtomicBool::new(false));
    let response_called = Arc::new(AtomicBool::new(false));

    let codec: Arc<dyn Codec> = Arc::new(SpyCodec {
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
    assert!(
        request_called.load(Ordering::SeqCst),
        "adapt_request 应被调用"
    );
    // 上游收到了经过 transform 的 body（含 _proxied 字段）
    let sent_body = captured.lock().clone().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&sent_body).unwrap();
    assert!(
        v["_proxied"].as_bool().unwrap_or(false),
        "请求体应被 adapt_request 转换"
    );
}

#[tokio::test]
async fn forward_calls_adapt_response_for_streaming() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress {
        captured_body: captured.clone(),
    };
    let request_called = Arc::new(AtomicBool::new(false));
    let response_called = Arc::new(AtomicBool::new(false));

    // 响应方向要查 (Gemini → OpenAi)，所以两个方向都得登记。
    let reg = mk_bidi_registry(request_called.clone(), response_called.clone());

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
    assert!(
        response_called.load(Ordering::SeqCst),
        "adapt_response 应被调用"
    );
    // 响应经过 transform（前缀 [adapter]）
    let s = String::from_utf8_lossy(&first);
    assert!(
        s.contains("[adapter]"),
        "响应应被 adapt_response 转换: got {s}"
    );
}

#[tokio::test]
async fn forward_passthrough_when_no_adaptor() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress {
        captured_body: captured.clone(),
    };

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

/// 请求与响应必须各用自己方向的 codec。
///
/// `Codec` 是有向的：内建 `ClaudeCodec` 用 `to_claude` 区分方向，请求方向的
/// 那个对 `adapt_response` 直接返回 `Unsupported`。用同一个 codec 两头转，
/// 每个 claude / gemini 渠道的响应都会变成 502。
///
/// 回归：smoke 里 claude 渠道请求发出去正常、响应 502 "unsupported conversion:
/// OpenAi -> Claude"。此处用真实内建注册表（而非双向 SpyCodec）复现。
#[tokio::test]
async fn forward_uses_reverse_codec_for_response() {
    /// 假上游：回 Anthropic Messages 形状的响应。
    struct ClaudeShapedEgress;
    impl Egress for ClaudeShapedEgress {
        fn execute<'a>(
            &'a self,
            _url: &'a str,
            _headers: &'a [(String, String)],
            _body: Bytes,
            _timeouts: &'a Timeouts,
        ) -> Pin<
            Box<
                dyn std::future::Future<Output = Result<ForwardedResponse, NormalizedError>>
                    + Send
                    + 'a,
            >,
        > {
            let payload = br#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet","content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn","usage":{"input_tokens":7,"output_tokens":3}}"#;
            let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(
                Bytes::from_static(payload),
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

    let mut task = mk_task(false);
    task.provider_type = "claude".to_string();

    let forwarded = forward_once(
        &task,
        &ClaudeShapedEgress,
        &AdaptorRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("claude 响应应能转回 OpenAI 形状, 而不是 Unsupported");

    use futures_util::StreamExt;
    let mut body = forwarded.body;
    let chunk = body
        .next()
        .await
        .expect("应有响应体")
        .expect("响应体应可读");
    let v: serde_json::Value = serde_json::from_slice(&chunk).expect("应是合法 JSON");

    // 客户端拿到的必须是 OpenAI chat.completion，而不是原始 Anthropic 形状。
    assert_eq!(v["object"], "chat.completion");
    assert_eq!(v["choices"][0]["message"]["content"], "pong");
}
