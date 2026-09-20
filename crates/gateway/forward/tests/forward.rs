//! forward 转发管道的行为测试 — adapter URL/头 / sanitize / egress / stream。

use bytes::Bytes;
use forward::ForwardTask;
use forward::adapter::{prepare, sanitize_client_headers};
use forward::egress::{Egress, ReqwestEgress, Timeouts};
use forward::stream::{AbortGuard, SseContext, finish, pipe_chunk};
use gateway_protocol_bridge::FormatRegistry;
use std::sync::Arc;
use std::time::Duration;

use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::candidate::Candidate;
use gateway_pipeline::ctx::ProtocolKind;

// ---------- 测试辅助 ----------

fn candidate(provider: &str) -> Candidate {
    Candidate {
        unit: RouteUnitRecord {
            meta: SyncMeta {
                key: "u1".to_string(),
                schema_version: 1,
                logical_version: 1,
                origin: "test".to_string(),
                updated_at: chrono::Utc::now(),
            },
            group: "g".to_string(),
            public_model: "m".to_string(),
            channel_key: "ch1".to_string(),
            key_index: 0,
            upstream_model: "m".to_string(),
            priority: 10,
            weight: 10,
            status: 1,
        },
        secret: format!("sk-{provider}-secret"),
        base_url: "https://upstream.example".to_string(),
        upstream_model: "m".to_string(),
        provider_type: provider.to_string(),
        settings: serde_json::Value::Null,
    }
}

// ---------- adapter ----------

/// 客户端 path 不得决定上游端点 —— 转换后的体是什么格式, URL 就必须是什么端点。
/// 这是跨格式转换能真正打通的另一半 (体在 protocol-bridge 转, 路径在 adapter 转)。
#[test]
fn adapter_url_follows_upstream_protocol_not_client_path() {
    // Claude 客户端 (打 /v1/messages) 命中 OpenAI 渠道 → 必须 /v1/chat/completions,
    // 因为体已被编成 Chat 格式; 用客户端的 /v1/messages 会 404。
    let c = candidate("openai");
    let p = prepare(
        &c,
        "/v1/messages",
        "openai",
        ProtocolKind::Anthropic,
        false,
        "m",
        vec![],
    );
    assert_eq!(p.url, "https://upstream.example/v1/chat/completions");

    // Gemini 客户端 命中 Claude 渠道 → /v1/messages, 客户端的 :generateContent 作废。
    let c = candidate("claude");
    let p = prepare(
        &c,
        "/v1beta/models/gemini-pro:generateContent",
        "claude",
        ProtocolKind::Gemini,
        false,
        "m",
        vec![],
    );
    assert_eq!(p.url, "https://upstream.example/v1/messages");

    // OpenAI 渠道 + Responses 客户端 → Responses 端点 (体是 Responses 格式)。
    let c = candidate("openai");
    let p = prepare(
        &c,
        "/v1/chat/completions",
        "openai",
        ProtocolKind::OpenAIResp,
        false,
        "m",
        vec![],
    );
    assert_eq!(p.url, "https://upstream.example/v1/responses");

    // Gemini 渠道: model 入路径, verb 随 stream; model 取上游真名而非客户端别名。
    let c = candidate("gemini");
    let p = prepare(
        &c,
        "/v1/messages",
        "gemini",
        ProtocolKind::OpenAI,
        true,
        "gpt-4-0613",
        vec![],
    );
    assert_eq!(
        p.url,
        "https://upstream.example/v1beta/models/gpt-4-0613:streamGenerateContent"
    );
    let p = prepare(
        &c,
        "/v1/messages",
        "gemini",
        ProtocolKind::OpenAI,
        false,
        "gpt-4-0613",
        vec![],
    );
    assert_eq!(
        p.url,
        "https://upstream.example/v1beta/models/gpt-4-0613:generateContent"
    );

    // passthrough: 客户端 path 原样, 不被改写。
    let c = candidate("passthrough");
    let p = prepare(
        &c,
        "/custom/endpoint",
        "passthrough",
        ProtocolKind::OpenAI,
        false,
        "m",
        vec![],
    );
    assert_eq!(p.url, "https://upstream.example/custom/endpoint");
}

#[test]
fn adapter_builds_url_per_provider() {
    // openai → base + /v1 + path
    let c = candidate("openai");
    let p = prepare(
        &c,
        "/chat/completions",
        "openai",
        ProtocolKind::OpenAI,
        false,
        "m",
        vec![],
    );
    assert_eq!(p.url, "https://upstream.example/v1/chat/completions");
    assert_eq!(p.auth_header.0, "Authorization");
    assert_eq!(p.auth_header.1, "Bearer sk-openai-secret");

    // claude → base + /v1/messages
    let c = candidate("claude");
    let p = prepare(
        &c,
        "/messages",
        "claude",
        ProtocolKind::Anthropic,
        false,
        "m",
        vec![],
    );
    assert!(p.url.ends_with("/v1/messages"));
    assert_eq!(p.auth_header.0, "x-api-key");
    assert_eq!(p.auth_header.1, "sk-claude-secret");

    // gemini → base + /v1beta/...
    let c = candidate("gemini");
    let p = prepare(
        &c,
        "/v1/messages",
        "gemini",
        ProtocolKind::Anthropic,
        false,
        "gemini-pro",
        vec![],
    );
    assert_eq!(
        p.url,
        "https://upstream.example/v1beta/models/gemini-pro:generateContent"
    );
    assert_eq!(p.auth_header.0, "x-goog-api-key");
}

#[test]
fn adapter_extra_headers_carried_into_merge() {
    // prepare 只把 extra_headers 存进产物; 覆盖发生在 pipeline::merge_headers
    // (后置头覆盖前置 — 渠道 settings 头盖过鉴权头)。
    let c = candidate("openai");
    let p = prepare(
        &c,
        "/chat/completions",
        "openai",
        ProtocolKind::OpenAI,
        false,
        "m",
        vec![("authorization".to_string(), "Bearer override".to_string())],
    );
    assert_eq!(p.auth_header.0, "Authorization");
    assert_eq!(p.auth_header.1, "Bearer sk-openai-secret");
    assert_eq!(p.extra_headers.len(), 1);
    // merge_headers: 顺序 = auth_header → extra_headers → client_headers, 重复键后者胜
    let merged = forward::pipeline::merge_headers(&p, &[]);
    assert_eq!(merged.len(), 2);
    assert_eq!(
        merged.iter().find(|(k, _)| k == "authorization").unwrap().1,
        "Bearer override",
        "extra_headers 应在 merge 时覆盖 auth_header"
    );
}

#[test]
fn sanitize_strips_hop_by_hop_and_credentials() {
    let headers = vec![
        ("authorization".to_string(), "Bearer sk-xxx".to_string()),
        ("cookie".to_string(), "session=1".to_string()),
        ("host".to_string(), "client.example".to_string()),
        ("content-length".to_string(), "100".to_string()),
        ("connection".to_string(), "keep-alive".to_string()),
        ("transfer-encoding".to_string(), "chunked".to_string()),
        ("proxy-authorization".to_string(), "Basic abc".to_string()),
        ("x-api-key".to_string(), "sk-xxx".to_string()),
        ("accept".to_string(), "application/json".to_string()),
        ("user-agent".to_string(), "curl/8".to_string()),
        ("x-custom".to_string(), "keep-me".to_string()),
    ];
    let kept = sanitize_client_headers(&headers);
    let names: Vec<&str> = kept.iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["accept", "user-agent", "x-custom"]);
}

// ---------- egress ----------

/// 起一个极简 TCP HTTP 服务，返回 `status` 与 `body`。
async fn serve_once(
    status_line: &'static str,
    body: &'static str,
) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        let (mut sock, _) = listener.accept().await.unwrap();
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let mut buf = [0u8; 4096];
        let _ = sock.read(&mut buf).await; // 读请求头
        let resp = format!(
            "HTTP/1.1 {status_line}\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = sock.write_all(resp.as_bytes()).await;
        let _ = sock.flush().await;
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn egress_returns_body_for_2xx() {
    let (url, server) = serve_once("200 OK", "hello world").await;
    let egress = ReqwestEgress::new();
    let resp = egress
        .execute(&url, &[], Bytes::new(), &Timeouts::default())
        .await
        .expect("2xx 应成功");
    assert_eq!(resp.status(), 200);

    let mut stream = resp.into_body_stream();
    use futures_util::StreamExt;
    let mut all = Vec::new();
    while let Some(chunk) = stream.next().await {
        all.extend_from_slice(&chunk.unwrap());
    }
    assert_eq!(String::from_utf8_lossy(&all), "hello world");
    server.await.unwrap();
}

#[tokio::test]
async fn egress_classifies_429_as_retryable() {
    let (url, server) = serve_once("429 Too Many Requests", "rate limited").await;
    let egress = ReqwestEgress::new();
    let err = egress
        .execute(&url, &[], Bytes::new(), &Timeouts::default())
        .await
        .expect_err("429 应为错误");
    assert!(err.retryable, "429 应标记可重试");
    assert_eq!(err.status, 429);
    server.await.unwrap();
}

// ---------- stream ----------

#[test]
fn pipe_chunk_passthrough_preserves_bytes() {
    let mut ctx = SseContext::new();
    let chunk = Bytes::from_static(b"data: {\"hello\":1}\n\n");
    let out = pipe_chunk(&mut ctx, &chunk);
    assert_eq!(out.passthrough, chunk, "透传必须逐字保真");
}

#[test]
fn pipe_chunk_passthrough_and_scanner_contract() {
    // protocol::SseScanner 当前是桩 (TODO#502 行状态机未实现): push 恒返回
    // 空事件、finish 恒 Truncated。这里验证 forward 的调用契约正确:
    // 1) 透传逐字保真 (forward 的硬保证); 2) events 是 Vec 且可消费;
    // 3) finish 返回 SseEnd (扫描器完成后由调用方消费, 不可丢弃)。
    let mut ctx = SseContext::new();
    let frame = Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n");
    let out = pipe_chunk(&mut ctx, &frame);
    assert_eq!(out.passthrough, frame, "透传必须逐字保真");
    assert_eq!(out.events.len(), 1, "首个 data 行应触发 FirstToken");
    assert_eq!(
        out.events[0],
        gateway_protocol_bridge::sse::SseEvent::FirstToken
    );
    let (_end, _counts, _event) = finish(ctx, 200, None); // 扫描器完成信号必须被消费
}

#[test]
fn abort_guard_cancel_signals_watcher() {
    let (guard, _stop_rx) = AbortGuard::new();
    guard.cancel();
    // 取消信号走 CancellationToken (cancelled() future), oneshot 是 stop 通道
    let res = tokio::runtime::Runtime::new().unwrap().block_on(async {
        tokio::time::timeout(Duration::from_millis(200), guard.cancelled()).await
    });
    assert!(res.is_ok(), "cancel() 后 cancelled() 应被唤醒");
}

#[test]
fn forward_task_is_cloneable() {
    let task = ForwardTask {
        candidate: candidate("openai"),
        path: "/v1/chat/completions".to_string(),
        headers: vec![],
        body: Bytes::from_static(b"{}"),
        stream: false,
        provider_type: "openai".to_string(),
        extra_headers: vec![],
        inbound_format: gateway_pipeline::ctx::ProtocolKind::OpenAI,
    };
    let clone = task.clone();
    assert_eq!(clone.path, task.path);
    assert_eq!(clone.body, task.body);
    assert_eq!(clone.candidate.secret, task.candidate.secret);
}

// ---------- 公开别名 → 上游真名改写 ----------

/// 上游只认真名：路由单元把 `gpt-4` 映射到 `gpt-4-0613` 时，发出去的体里
/// `model` 必须是真名，否则上游报 model not found。
/// 回归：smoke 发现别名映射未生效，上游收到的仍是公开别名。
/// resolve_upstream_model 决定 Gemini 路径里的 model: 渠道映射优先, 缺省回落客户端。
/// 真实 HTTP 回环: Claude 客户端 (打 /v1/messages) 命中 OpenAI 渠道时,
/// 上游必须收到 /v1/chat/completions —— 这是本 PR 修的 bug 的可观测复现。
///
/// forward_once 全链路 (prepare 真实拼 URL → reqwest 真发 → 响应转回入站格式),
/// mock 上游只回最简 Chat 完成体; 转换失败会直接让测试红。
#[tokio::test]
async fn forward_once_routes_claude_client_to_openai_endpoint() {
    use std::io::Read;
    use std::net::TcpListener;

    // 单次 mock 上游: 抓请求行, 回最简非流式 Chat 响应。
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let (req_path_tx, req_path_rx) = std::sync::mpsc::channel::<String>();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let n = stream.read(&mut buf).expect("read");
        let req = String::from_utf8_lossy(&buf[..n]);
        // 请求行第一段: "POST /v1/chat/completions HTTP/1.1"
        let path = req
            .lines()
            .next()
            .and_then(|l| l.split_whitespace().nth(1))
            .unwrap_or_default()
            .to_string();
        let _ = req_path_tx.send(path);
        let body = serde_json::json!({
            "id":"chatcmpl-1","object":"chat.completion","created":0,
            "model":"m","choices":[{"index":0,"message":{
                "role":"assistant","content":"hi"},"finish_reason":"stop"}],
            "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
        });
        let bytes = body.to_string();
        let resp = format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{}",
            bytes.len(),
            bytes
        );
        use std::io::Write;
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    });

    // Claude 客户端形状的请求体 + 客户端路径 /v1/messages。
    let client_body = Bytes::from(
        serde_json::json!({
            "model":"m","max_tokens":16,
            "messages":[{"role":"user","content":"hi"}]
        })
        .to_string(),
    );
    let task = ForwardTask {
        candidate: candidate("openai"),
        path: "/v1/messages".to_string(),
        headers: vec![("accept".to_string(), "application/json".to_string())],
        body: client_body,
        stream: false,
        provider_type: "openai".to_string(),
        extra_headers: vec![],
        inbound_format: ProtocolKind::Anthropic,
    };
    // candidate() 的 base_url 指向 example; 换成本地 mock 上游。
    let mut task = task;
    task.candidate.base_url = format!("http://127.0.0.1:{port}");

    let egress = ReqwestEgress::new();
    let formats = Arc::new(FormatRegistry::with_defaults());
    let timeouts = Timeouts::default();

    let forwarded = forward::pipeline::forward_once(&task, &egress, &formats, &timeouts)
        .await
        .expect("forward must succeed");

    assert_eq!(forwarded.status, 200);
    // 核心断言: 上游收到的路径是 OpenAI 端点, 不是客户端的 /v1/messages。
    assert_eq!(
        req_path_rx.recv().expect("upstream saw a request"),
        "/v1/chat/completions",
        "Claude 客户端打 OpenAI 渠道必须落到 chat/completions 端点"
    );
}

/// 跨格式 + 上游 4xx 的真实契约: forward_once 必须把上游的 400 原样传播为
/// Err(NormalizedError{status:400, retryable:false}), 而不是转成可重试 502。
///
/// 这条路径上有一个容易踩错的点: 两个生产 Egress (ReqwestEgress / AdapterEgress)
/// 都在 execute 内把非 2xx 归类成 Err 后由 `?` 传播, translate_response 因此
/// 永远只见到 2xx。本测试锁住这个契约 —— 若哪天有 Egress 改成透传非 2xx 的
/// Ok(ForwardedResponse), 错误体就会落到 translate_response 被成功形状 decoder
/// 解析失败, 上游的 400 就会退化成 ferrite 的可重试 502 (诱导重试已被拒绝的请求)。
#[tokio::test]
async fn cross_format_upstream_400_propagates_as_400_not_502() {
    use std::io::Read;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).expect("read");
        let err = r#"{"error":{"message":"invalid_api_key","type":"invalid_request_error","code":"invalid_api_key"}}"#;
        let resp = format!(
            "HTTP/1.1 400 Bad Request\r\ncontent-type: application/json\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{}",
            err.len(),
            err
        );
        use std::io::Write;
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    });

    let mut task = ForwardTask {
        candidate: candidate("openai"),
        path: "/v1/messages".to_string(),
        headers: vec![("accept".to_string(), "application/json".to_string())],
        body: Bytes::from(
            serde_json::json!({"model":"m","max_tokens":16,"messages":[{"role":"user","content":"hi"}]})
                .to_string(),
        ),
        stream: false,
        provider_type: "openai".to_string(),
        extra_headers: vec![],
        inbound_format: ProtocolKind::Anthropic,
    };
    task.candidate.base_url = format!("http://127.0.0.1:{port}");

    // 客户端说 Anthropic、渠道是 openai: 跨格式。真实 reqwest 链路。
    let result = forward::pipeline::forward_once(
        &task,
        &ReqwestEgress::new(),
        &Arc::new(FormatRegistry::with_defaults()),
        &Timeouts::default(),
    )
    .await;

    let err = match result {
        Ok(forwarded) => panic!(
            "上游 4xx 必须传播为 Err, 不该是 Ok(Forwarded): status={}",
            forwarded.status
        ),
        Err(e) => e,
    };

    assert_eq!(err.status, 400, "上游 400 必须保持 400, 不该变成 502");
    assert!(!err.retryable, "4xx 是请求本身坏, 不该可重试");
    assert!(
        err.message.contains("invalid_api_key"),
        "上游错误体应进 message 供 error_mapping 转成客户端格式, 实际: {}",
        err.message
    );
}

/// 流式上游非 2xx 的真实回环契约：上游以 SSE content-type 回 413 时，客户端
/// 必须拿到 413（不是可重试的 502），错误体经 error_mapping 转成入站格式形状。
///
/// 注意：这条回环**不验证管道守卫本身**——ReqwestEgress 已在 execute 内把非 2xx
/// 归类为 Err（见上一条非流式测试的说明），请求根本走不到守卫；守卫的故障注入
/// 覆盖在 adaptor_wiring.rs。本测试锁的是「413 不退化成 502、错误形状按入站
/// 格式回出」这条面向客户端的契约。
#[tokio::test]
async fn stream_upstream_413_propagates_as_413_not_sse() {
    use std::io::Read;
    use std::net::TcpListener;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("addr").port();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = [0u8; 4096];
        let _ = stream.read(&mut buf).expect("read");
        // 关键: content-type 是 text/event-stream, 状态码却是 413 ——
        // 这正是会骗过流式分支、让错误字节进 SSE scanner 的场景。
        let err = r#"{"error":{"message":"context_length_exceeded","type":"invalid_request_error","code":"context_length_exceeded"}}"#;
        let resp = format!(
            "HTTP/1.1 413 Payload Too Large\r\ncontent-type: text/event-stream\r\n\
             content-length: {}\r\nconnection: close\r\n\r\n{}",
            err.len(),
            err
        );
        use std::io::Write;
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    });

    // 客户端说 Anthropic (SSE)、渠道是 openai: 跨格式流式, 必须过
    // inbound != upstream_format 的 SSE 转换分支, 不能走同格式透传捷径。
    let mut task = ForwardTask {
        candidate: candidate("openai"),
        path: "/v1/messages".to_string(),
        headers: vec![("accept".to_string(), "text/event-stream".to_string())],
        body: Bytes::from(
            serde_json::json!({"model":"m","max_tokens":16,"stream":true,
                "messages":[{"role":"user","content":"hi"}]})
            .to_string(),
        ),
        stream: true,
        provider_type: "openai".to_string(),
        extra_headers: vec![],
        inbound_format: ProtocolKind::Anthropic,
    };
    task.candidate.base_url = format!("http://127.0.0.1:{port}");

    let result = forward::pipeline::forward_once(
        &task,
        &ReqwestEgress::new(),
        &Arc::new(FormatRegistry::with_defaults()),
        &Timeouts::default(),
    )
    .await;

    let err = match result {
        Ok(forwarded) => panic!(
            "流式上游 4xx 必须传播为 Err, 不该把错误体当 SSE 透传: status={} content_type={}",
            forwarded.status, forwarded.content_type
        ),
        Err(e) => e,
    };

    assert_eq!(err.status, 413, "上游 413 必须保持 413, 不该变成 502");
    assert!(
        !err.retryable,
        "4xx 是请求本身坏, 不该可重试 (会诱导重试已拒绝的请求)"
    );
    assert!(
        err.message.contains("context_length_exceeded"),
        "上游错误体应进 message 供 error_mapping 转成客户端格式, 实际: {}",
        err.message
    );

    // 客户端侧: 入站格式是 Anthropic, 错误体必须是 Anthropic 形状
    // (不是上游的 OpenAI 形状) —— 这条链路由 error_mapping::map_error 收口。
    let shape = gateway_protocol_bridge::error_mapping::to_anthropic_shape(&err);
    assert_eq!(
        shape["type"], "error",
        "Anthropic 错误体顶层 type 必须是 error"
    );
    // 不 pin 整条 message：classify_status 把预览截断成最多 200 字符，客户端
    // 侧的精确错误文本由上游决定，只锁可观测的形状。
}

/// model 直接进 URL 路径段, 客户端可发任意串 — 非法值必须被拒, 不能改写上游路径。
#[test]
fn resolve_upstream_model_rejects_unsafe_for_url() {
    use forward::pipeline::resolve_upstream_model;
    // 渠道映射的非法值同样拒 (坏配置不该产出畸形 URL)。
    assert_eq!(
        resolve_upstream_model(&Bytes::from(b"{}".as_ref()), "a/b"),
        ""
    );
    assert_eq!(
        resolve_upstream_model(&Bytes::from(b"{}".as_ref()), "../x"),
        ""
    );
    // 单独的 ".." 不是穿越: URL 里它是 "..:verb" 一个段, 放行。
    assert_eq!(
        resolve_upstream_model(&Bytes::from(b"{}".as_ref()), ".."),
        ".."
    );
    // 客户端非法 model → 不寻址。
    let bad = Bytes::from(r#"{"model":"x/../../etc"}"#);
    assert_eq!(resolve_upstream_model(&bad, ""), "");
    // 合法值照常: 含 . - _ 的模型名是常态。
    assert_eq!(
        resolve_upstream_model(&Bytes::from(r#"{"model":"gemini-1.5.pro"}"#), ""),
        "gemini-1.5.pro"
    );
}

#[test]
fn resolve_upstream_model_prefers_channel_mapping() {
    use forward::pipeline::resolve_upstream_model;
    let body = Bytes::from(r#"{"model":"gpt-4","messages":[]}"#);
    // 渠道有真名映射 → 用真名, 客户端别名作废。
    assert_eq!(resolve_upstream_model(&body, "gpt-4-0613"), "gpt-4-0613");
    // 无映射 (空串) → 回落客户端发的 model。
    assert_eq!(resolve_upstream_model(&body, ""), "gpt-4");
    // 无映射且体非 JSON → 空, 由 build_url 退回客户端 path。
    assert_eq!(
        resolve_upstream_model(&Bytes::from(b"not json".as_ref()), ""),
        ""
    );
    // 无 model 字段 → 空。
    assert_eq!(resolve_upstream_model(&Bytes::from(r#"{"foo":1}"#), ""), "");
}

#[test]
fn rewrite_upstream_model_replaces_alias() {
    let body =
        Bytes::from_static(br#"{"model":"gpt-4","messages":[{"role":"user","content":"x"}]}"#);
    let out = forward::pipeline::rewrite_upstream_model(&body, "gpt-4-0613");
    let v: serde_json::Value = serde_json::from_slice(&out).expect("仍是合法 JSON");
    assert_eq!(v["model"], "gpt-4-0613");
    // 其余字段不能被改写动作破坏。
    assert_eq!(v["messages"][0]["content"], "x");
}

/// 没有可靠改写位置时透传，不猜测也不报错：空真名、非 JSON、无 model 字段。
#[test]
fn rewrite_upstream_model_passes_through_when_not_applicable() {
    let json = Bytes::from_static(br#"{"model":"gpt-4"}"#);
    assert_eq!(
        forward::pipeline::rewrite_upstream_model(&json, ""),
        json,
        "上游真名为空时原样透传"
    );

    let not_json = Bytes::from_static(b"not json at all");
    assert_eq!(
        forward::pipeline::rewrite_upstream_model(&not_json, "m"),
        not_json,
        "非 JSON 体原样透传"
    );

    // 无 model 字段 = 非聊天类请求（如 /v1/models），不应凭空插入 model。
    let no_model = Bytes::from_static(br#"{"input":"hi"}"#);
    assert_eq!(
        forward::pipeline::rewrite_upstream_model(&no_model, "m"),
        no_model,
        "缺 model 字段时不插入"
    );
}
