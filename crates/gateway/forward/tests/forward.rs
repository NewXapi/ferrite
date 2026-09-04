//! forward 转发管道的行为测试 — adapter URL/头 / sanitize / egress / stream。

use bytes::Bytes;
use forward::ForwardTask;
use forward::adapter::{prepare, sanitize_client_headers};
use forward::egress::{Egress, ReqwestEgress, Timeouts};
use forward::stream::{AbortGuard, SseContext, finish, pipe_chunk};
use std::time::Duration;

use contract::records::{RouteUnitRecord, SyncMeta};
use dispatch::candidate::Candidate;

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
    }
}

// ---------- adapter ----------

#[test]
fn adapter_builds_url_per_provider() {
    // openai → base + /v1 + path
    let c = candidate("openai");
    let p = prepare(&c, "/chat/completions", "openai", vec![]);
    assert_eq!(p.url, "https://upstream.example/v1/chat/completions");
    assert_eq!(p.auth_header.0, "Authorization");
    assert_eq!(p.auth_header.1, "Bearer sk-openai-secret");

    // claude → base + /v1/messages
    let c = candidate("claude");
    let p = prepare(&c, "/messages", "claude", vec![]);
    assert!(p.url.ends_with("/v1/messages"));
    assert_eq!(p.auth_header.0, "x-api-key");
    assert_eq!(p.auth_header.1, "sk-claude-secret");

    // gemini → base + /v1beta/...
    let c = candidate("gemini");
    let p = prepare(&c, "/models/gemini-pro:generateContent", "gemini", vec![]);
    assert!(p.url.contains("/v1beta/models/gemini-pro"));
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
    assert!(
        out.events.is_empty(),
        "桩扫描器当前无事件 (protocol TODO#502)"
    );
    let _end = finish(ctx); // 扫描器完成信号必须被消费
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
    };
    let clone = task.clone();
    assert_eq!(clone.path, task.path);
    assert_eq!(clone.body, task.body);
    assert_eq!(clone.candidate.secret, task.candidate.secret);
}
