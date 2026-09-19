//! forward 转发管道的行为测试 — adapter URL/头 / sanitize / egress / stream。

use bytes::Bytes;
use forward::ForwardTask;
use forward::adapter::{prepare, sanitize_client_headers};
use forward::egress::{Egress, ReqwestEgress, Timeouts};
use forward::stream::{AbortGuard, SseContext, finish, pipe_chunk};
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
