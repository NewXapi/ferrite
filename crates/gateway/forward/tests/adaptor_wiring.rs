//! forward ↔ protocol-bridge 接线测试：验证请求/响应经格式转换。
//!
//! 旧版本用 `(source, target)` 有向 `Codec` 的 SpyCodec 验证接线，其中还专门记了
//! 一条回归——「拿请求方向的 codec 去转响应 → 502 Unsupported」。新抽象下方向由
//! 方法名承载（`decode_request`/`encode_request` vs `decode_response`/`encode_response`），
//! 请求与响应共用同一个单格式 codec，该缺陷在结构上不可能：只要有 codec 注册，
//! 两个方向就都存在。本文件的用例因此改成验证「两跳真的发生、且产物是入站格式」。

use bytes::Bytes;
use contract::error::NormalizedError;
use contract::records::RouteUnitRecord;
use forward::ForwardTask;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use forward::pipeline::forward_once;
use gateway_pipeline::ctx::ProtocolKind;
use gateway_protocol_bridge::format_codec::FormatRegistry;
use parking_lot::Mutex;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

// ---------- mock Egress ----------

/// 假上游：返回固定 body，并记录收到的请求体。
struct MockEgress {
    captured_body: Arc<Mutex<Option<Bytes>>>,
    response: Bytes,
    content_type: &'static str,
}

impl Egress for MockEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> Pin<Box<dyn Future<Output = Result<ForwardedResponse, NormalizedError>> + Send + 'a>> {
        *self.captured_body.lock() = Some(body.clone());
        let payload = self.response.clone();
        let ct = self.content_type;
        let stream = futures_util::stream::iter(vec![Ok::<Bytes, std::io::Error>(payload)]);
        Box::pin(async move { Ok(ForwardedResponse::from_stream(200, ct, stream)) })
    }
}

// ---------- helpers ----------

fn mk_task(stream: bool, inbound: ProtocolKind, provider_type: &str) -> ForwardTask {
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
            provider_type: provider_type.into(),
            settings: serde_json::Value::Null,
        },
        path: "/v1/chat/completions".to_string(),
        headers: vec![],
        // 公开名与上游真名一致：别名改写是 no-op，本文件测的只是格式转换。
        body: Bytes::from_static(
            b"{\"model\":\"m\",\"messages\":[{\"role\":\"user\",\"content\":\"ping\"}]}",
        ),
        stream,
        provider_type: provider_type.into(),
        extra_headers: vec![],
        inbound_format: inbound,
    }
}

/// 读干一条响应流。
async fn drain(forwarded: forward::Forwarded) -> Bytes {
    use futures_util::StreamExt;
    let mut body = forwarded.body;
    let mut out = Vec::new();
    while let Some(chunk) = body.next().await {
        out.extend_from_slice(&chunk.expect("chunk"));
    }
    Bytes::from(out)
}

// ---------- 请求方向 ----------

// OpenAI 客户端 → Claude 渠道：请求体必须被转成 Claude 形状（system 顶层、必填 max_tokens）。
// 旧实现只在「入站恒 OpenAI」的假设下工作，这里显式验证入站格式确实驱动转换。
#[tokio::test]
async fn openai_client_request_is_translated_to_claude_upstream() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress {
        captured_body: captured.clone(),
        response: Bytes::from_static(br#"{"id":"msg_1","type":"message","role":"assistant","model":"m","content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}}"#),
        content_type: "application/json",
    };

    let task = mk_task(false, ProtocolKind::OpenAI, "claude");
    let _ = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("转发应成功");

    let sent = captured.lock().clone().unwrap();
    let v: serde_json::Value = serde_json::from_slice(&sent).expect("发往上游的应是 JSON");

    assert_eq!(
        v["model"], "m",
        "别名改写必须发生在转换之前（否则 model 已被搬进厂商字段）"
    );
    assert!(
        v["max_tokens"].as_u64().is_some_and(|n| n > 0),
        "Claude 必填的 max_tokens 必须被补上，实际: {v}"
    );
    assert!(
        v.get("messages").is_some(),
        "Claude 体应有 messages，实际: {v}"
    );
}

// Claude 客户端 → Claude 渠道：同格式应零转换直通（不该被解析重写）。
#[tokio::test]
async fn same_format_request_passes_through_verbatim() {
    let captured = Arc::new(Mutex::new(None));
    let egress = MockEgress {
        captured_body: captured.clone(),
        response: Bytes::from_static(b"{}"),
        content_type: "application/json",
    };

    let task = mk_task(false, ProtocolKind::Anthropic, "claude");
    let _ = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("转发应成功");

    let sent = captured.lock().clone().unwrap();
    // 上游真名改写仍会发生（model 字段），但除此之外字节应保持原样：
    // 原 body 只有 model+messages，转换若发生会产出 Claude 专属字段。
    let v: serde_json::Value = serde_json::from_slice(&sent).unwrap();
    assert!(
        v.get("max_tokens").is_none(),
        "同格式不该被注入厂商必填字段，实际: {v}"
    );
}

// ---------- 响应方向 ----------

// Claude 渠道响应 → OpenAI 客户端：必须转成 chat.completion 形状。
// 这是旧实现里那条 502 回归的正向版本——现在只要有 codec 注册就必然成立。
#[tokio::test]
async fn claude_upstream_response_is_translated_to_openai_shape() {
    let egress = MockEgress {
        captured_body: Arc::new(Mutex::new(None)),
        response: Bytes::from_static(br#"{"id":"msg_1","type":"message","role":"assistant","model":"claude-sonnet","content":[{"type":"text","text":"pong"}],"stop_reason":"end_turn","usage":{"input_tokens":7,"output_tokens":3}}"#),
        content_type: "application/json",
    };

    let task = mk_task(false, ProtocolKind::OpenAI, "claude");
    let forwarded = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("claude 响应应能转回 OpenAI 形状");

    let body = drain(forwarded).await;
    let v: serde_json::Value = serde_json::from_slice(&body).expect("产物应是 JSON");

    assert_eq!(
        v["object"], "chat.completion",
        "响应必须转成 OpenAI 形状，实际: {v}"
    );
    assert_eq!(v["choices"][0]["message"]["content"], "pong");
    assert_eq!(
        v["choices"][0]["finish_reason"], "stop",
        "Claude 的 end_turn 必须映射成 stop"
    );
}

// OpenAI 客户端 → OpenAI 渠道：同格式响应零转换直通。
#[tokio::test]
async fn same_format_response_passes_through_verbatim() {
    let payload = Bytes::from_static(
        br#"{"id":"chatcmpl-1","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":"hi"},"finish_reason":"stop"}]}"#,
    );
    let egress = MockEgress {
        captured_body: Arc::new(Mutex::new(None)),
        response: payload.clone(),
        content_type: "application/json",
    };

    let task = mk_task(false, ProtocolKind::OpenAI, "openai");
    let forwarded = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("转发应成功");

    let body = drain(forwarded).await;
    assert_eq!(
        body, payload,
        "同格式响应必须逐字节透传（不该被解析再序列化）"
    );
}

// ---------- 流式：入站格式决定客户端收到什么 ----------

// **G2 核心回归**：Claude 客户端拿到的流式响应必须是 Claude 事件形状。
// 旧实现流式路径完全不转换（`ctx.upstream` 只在非流式分支写入），客户端只会拿到
// OpenAI chunk；这里用真实注册表验证出站帧带 Claude 的 `event:` 行。
#[tokio::test]
async fn claude_client_streaming_receives_claude_event_shape() {
    // 上游是 OpenAI 渠道，回 Chat chunk。
    let upstream_sse = Bytes::from_static(
        b"data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"po\"},\"finish_reason\":null}]}\n\n\
data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ng\"},\"finish_reason\":\"stop\"}]}\n\n\
data: [DONE]\n\n",
    );
    let egress = MockEgress {
        captured_body: Arc::new(Mutex::new(None)),
        response: upstream_sse,
        content_type: "text/event-stream",
    };

    // 客户端说 Claude（路径 /v1/messages 判定为 Anthropic），渠道是 openai。
    let mut task = mk_task(true, ProtocolKind::Anthropic, "openai");
    task.path = "/v1/messages".to_string();

    let forwarded = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("流式转发应成功");

    let body = drain(forwarded).await;
    let text = String::from_utf8_lossy(&body);

    assert!(
        text.contains("event: message_start"),
        "Claude 客户端必须收到 message_start，实际: {text}"
    );
    assert!(
        text.contains("\"type\":\"content_block_delta\""),
        "Claude 客户端必须收到 content_block_delta，实际: {text}"
    );
    assert!(
        text.contains("event: message_stop"),
        "Claude 客户端必须收到 message_stop，实际: {text}"
    );
    assert!(
        !text.contains("chat.completion.chunk"),
        "不得把 OpenAI chunk 原样漏给 Claude 客户端，实际: {text}"
    );

    // 文本按 Claude 的 delta 语义分散在各帧里：把 `text_delta` 的 text 拼起来，
    // 拼出的必须是完整原文。直接 grep "pong" 会误判——两个 delta（po / ng）之间
    // 隔着帧边界，字节流里并不相邻。
    let deltas: String = text
        .lines()
        .filter_map(|l| l.strip_prefix("data: "))
        .filter_map(|d| serde_json::from_str::<serde_json::Value>(d).ok())
        .filter(|v| v["type"] == "content_block_delta")
        .filter_map(|v| v["delta"]["text"].as_str().map(str::to_string))
        .collect();
    assert_eq!(
        deltas, "pong",
        "各 delta 拼起来必须是完整文本，实际: {deltas:?}"
    );
}

// OpenAI 客户端流式打 Claude 渠道：必须收到 Chat chunk（不是 Claude 事件）。
#[tokio::test]
async fn openai_client_streaming_receives_chat_chunk_shape() {
    let upstream_sse = Bytes::from_static(
        b"event: message_start\n\
data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"m\",\"content\":[],\"usage\":{\"input_tokens\":3,\"output_tokens\":0}}}\n\n\
event: content_block_start\n\
data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
event: content_block_delta\n\
data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"pong\"}}\n\n\
event: content_block_stop\n\
data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
event: message_delta\n\
data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\n\
event: message_stop\n\
data: {\"type\":\"message_stop\"}\n\n",
    );
    let egress = MockEgress {
        captured_body: Arc::new(Mutex::new(None)),
        response: upstream_sse,
        content_type: "text/event-stream",
    };

    // 客户端说 OpenAI，渠道是 claude，路径用 Claude 风格（服务端 SSE 形状由上游决定）。
    let task = mk_task(true, ProtocolKind::OpenAI, "claude");
    let forwarded = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("流式转发应成功");

    let body = drain(forwarded).await;
    let text = String::from_utf8_lossy(&body);

    assert!(
        text.contains("chat.completion.chunk"),
        "OpenAI 客户端必须收到 chat chunk，实际: {text}"
    );
    assert!(
        text.contains("\"content\":\"pong\""),
        "文本增量必须搬过去，实际: {text}"
    );
    assert!(
        !text.contains("event: message_start"),
        "不得把 Claude 事件原样漏给 OpenAI 客户端，实际: {text}"
    );
}

// SSE 规范不要求流以空行结尾：上游最后一帧没跟空行就断开时，该帧不得被丢掉。
// 丢的往往是收尾帧，客户端会一直等或判定流异常。
#[tokio::test]
async fn unterminated_trailing_frame_is_not_dropped() {
    // 注意结尾：最后一帧的 data 行后**没有**空行分隔符。
    let upstream_sse = Bytes::from_static(
        b"data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"hi\"},\"finish_reason\":null}]}\n\n\
data: {\"id\":\"chatcmpl-1\",\"object\":\"chat.completion.chunk\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}",
    );
    let egress = MockEgress {
        captured_body: Arc::new(Mutex::new(None)),
        response: upstream_sse,
        content_type: "text/event-stream",
    };

    let mut task = mk_task(true, ProtocolKind::Anthropic, "openai");
    task.path = "/v1/messages".to_string();

    let forwarded = forward_once(
        &task,
        &egress,
        &FormatRegistry::with_defaults(),
        &Timeouts::default(),
    )
    .await
    .expect("流式转发应成功");

    let body = drain(forwarded).await;
    let text = String::from_utf8_lossy(&body);

    // 尾部那帧的 finish_reason 必须体现在出站事件里（Claude 侧是 message_delta 的 stop_reason）。
    assert!(
        text.contains("\"stop_reason\":\"end_turn\""),
        "尾部未终结帧不得被丢弃，实际: {text}"
    );
}
