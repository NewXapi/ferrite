//! pipeline 编排核心的行为测试：StageOutcome 三态流转、错误短路、router 集成。

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gateway_pipeline::{
    PipeStream, Pipeline, RequestCtx, RequestMeta, Stage, StageError, StageOutcome,
};
use std::net::IpAddr;
use std::sync::Arc;
use uuid::Uuid;

// ---------- 测试辅助 ----------

/// 构造最小 RequestMeta（body 为空）
fn meta(path: &str) -> RequestMeta {
    RequestMeta {
        method: "POST".to_string(),
        path: path.to_string(),
        headers: axum::http::HeaderMap::new(),
        body: gateway_pipeline::BodySource::InMemory(bytes::Bytes::new()),
        client_ip: IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
        request_id: Uuid::now_v7(),
        inbound_protocol: gateway_pipeline::ProtocolKind::OpenAI,
    }
}

/// 记录被调用顺序的桩 stage — 行为由 `Behavior` 决定, 避免持有
/// `StageOutcome`（内含非 Send 的 axum Body）破坏 Stage: Send+Sync。
enum Behavior {
    Continue,
    ShortCircuit(u16),
    Stream,
}

struct SpyStage {
    name: &'static str,
    behavior: Behavior,
    calls: Arc<std::sync::atomic::AtomicU32>,
}

#[async_trait::async_trait]
impl Stage for SpyStage {
    fn name(&self) -> &'static str {
        self.name
    }
    async fn handle(&self, _ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        self.calls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok(match self.behavior {
            Behavior::Continue => StageOutcome::Continue,
            Behavior::ShortCircuit(status) => StageOutcome::ShortCircuit(
                axum::http::Response::builder()
                    .status(status)
                    .body(Body::empty())
                    .unwrap(),
            ),
            Behavior::Stream => StageOutcome::Stream(PipeStream::new(Body::empty())),
        })
    }
}

// ---------- StageOutcome 流转 ----------

#[test]
fn pipeline_continue_runs_all_stages() {
    let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let pipe = Pipeline::new()
        .push(SpyStage {
            name: "a",
            behavior: Behavior::Continue,
            calls: calls.clone(),
        })
        .push(SpyStage {
            name: "b",
            behavior: Behavior::Continue,
            calls: calls.clone(),
        });

    let ctx = RequestCtx::new(meta("/v1/chat/completions"));
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(pipe.run(ctx));
    // Continue 全过但无产出 → 内部错误（pipeline 无法凭空造响应）
    assert!(matches!(result, Err(StageError::Internal(_))));
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn pipeline_short_circuit_stops_chain() {
    let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let pipe = Pipeline::new()
        .push(SpyStage {
            name: "first",
            behavior: Behavior::Continue,
            calls: calls.clone(),
        })
        .push(SpyStage {
            name: "stop",
            behavior: Behavior::ShortCircuit(401),
            calls: calls.clone(),
        })
        .push(SpyStage {
            name: "never",
            behavior: Behavior::Continue,
            calls: calls.clone(),
        });

    let ctx = RequestCtx::new(meta("/v1/chat/completions"));
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(pipe.run(ctx));
    let resp = result.expect("ShortCircuit 应返回 Ok");
    assert_eq!(resp.status(), 401);
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 2);
}

#[test]
fn pipeline_stream_terminates_immediately() {
    let calls = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let pipe = Pipeline::new()
        .push(SpyStage {
            name: "first",
            behavior: Behavior::Stream,
            calls: calls.clone(),
        })
        .push(SpyStage {
            name: "never",
            behavior: Behavior::Continue,
            calls: calls.clone(),
        });

    let ctx = RequestCtx::new(meta("/v1/chat/completions"));
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(pipe.run(ctx));
    assert!(result.is_ok(), "Stream 应直接返回响应");
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
}

/// 流式响应必须带 `content-type`。
///
/// SSE 客户端（OpenAI / Anthropic SDK）靠 `text/event-stream` 判定要按事件流读；
/// 缺了它客户端会把响应当普通 body 一次收完，流式体验退化成阻塞等待。
/// 回归：`PipeStream::into_response` 曾用 `Response::new` 直接包 body，丢掉了头。
#[test]
fn pipe_stream_response_carries_content_type() {
    let sse = PipeStream::new(axum::body::Body::from("data: {}\n\n"));
    let resp = sse.into_response();
    assert_eq!(
        resp.headers().get(http::header::CONTENT_TYPE).unwrap(),
        "text/event-stream",
        "默认构造应给 SSE 类型"
    );

    // 上游的类型原样带回（非 SSE 的流式响应也要保真）。
    let json = PipeStream::with_content_type(
        axum::body::Body::from("{}"),
        "application/json; charset=utf-8",
    );
    assert_eq!(
        json.into_response()
            .headers()
            .get(http::header::CONTENT_TYPE)
            .unwrap(),
        "application/json; charset=utf-8"
    );
}

// ---------- 错误短路 ----------

struct ErrStage;

#[async_trait::async_trait]
impl Stage for ErrStage {
    fn name(&self) -> &'static str {
        "err"
    }
    async fn handle(&self, _ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        Err(StageError::Unauthenticated("bad key".into()))
    }
}

#[test]
fn pipeline_error_stops_chain() {
    let pipe = Pipeline::new().push(ErrStage);
    let ctx = RequestCtx::new(meta("/v1/chat/completions"));
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(pipe.run(ctx));
    assert!(matches!(result, Err(StageError::Unauthenticated(_))));
}

// ---------- RequestCtx::from_axum ----------

#[test]
fn from_axum_extracts_meta_and_protocol() {
    let req = Request::builder()
        .method("POST")
        .uri("/v1/messages")
        .header("content-type", "application/json")
        .header("x-forwarded-for", "203.0.113.9")
        .body(Body::from(r#"{"model":"claude-3-5"}"#))
        .unwrap();
    let ctx = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(RequestCtx::from_axum(req))
        .unwrap();
    assert_eq!(ctx.request.method, "POST");
    assert_eq!(ctx.request.path, "/v1/messages");
    assert_eq!(
        ctx.request.client_ip,
        "203.0.113.9".parse::<IpAddr>().unwrap()
    );
    assert_eq!(
        ctx.request.inbound_protocol,
        gateway_pipeline::ProtocolKind::Anthropic
    );
    // body 已读取为 InMemory
    assert!(matches!(
        ctx.request.body,
        gateway_pipeline::BodySource::InMemory(_)
    ));
}

#[test]
fn detect_protocol_paths() {
    let cases = [
        (
            "/v1/chat/completions",
            gateway_pipeline::ProtocolKind::OpenAI,
        ),
        ("/v1/messages", gateway_pipeline::ProtocolKind::Anthropic),
        ("/v1/responses", gateway_pipeline::ProtocolKind::OpenAIResp),
        (
            "/v1beta/models/gemini-2.5-pro",
            gateway_pipeline::ProtocolKind::Gemini,
        ),
        (
            "/v1/chat/completions?model=gpt-4o",
            gateway_pipeline::ProtocolKind::OpenAI,
        ),
    ];
    for (path, expected) in cases {
        let req = Request::builder()
            .method("POST")
            .uri(path)
            .body(Body::empty())
            .unwrap();
        let ctx = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(RequestCtx::from_axum(req))
            .unwrap();
        assert_eq!(ctx.request.inbound_protocol, expected, "path={path}");
    }
}

// ---------- router 集成 ----------

#[test]
fn error_to_response_maps_status() {
    let resp = gateway_pipeline::error_to_response(StageError::QuotaExhausted {
        remaining: 0,
        required: 10,
    });
    assert_eq!(resp.status(), StatusCode::PAYMENT_REQUIRED);

    let resp = gateway_pipeline::error_to_response(StageError::NoRoute);
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let resp = gateway_pipeline::error_to_response(StageError::Unauthenticated("no key".into()));
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}
