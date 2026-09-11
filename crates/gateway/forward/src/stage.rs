//! `stage` — ForwardStage: 把转发管道接入 pipeline
//!
//! 两种驱动模式：
//! 1. **单次转发**（默认，DispatchStage → ForwardStage 链路）：从 ctx.route
//!    (DispatchStage 写入) 组装 ForwardTask，调 forward_once 发一次上游；
//! 2. **重试循环**（`with_retry` 注入 dispatch + RetryPolicy 后）：handle
//!    忽略 ctx.route，自己驱动 `dispatch::retry::run_retry_loop`
//!    （选候选 → 尝试 → 健康回报 → 排除已试 → 再选），获胜候选的响应经
//!    `commit_forwarded` 写入 ctx.upstream（非流式）或直接回流（流式）。
//!
//! 重试编排与健康回报都在 dispatch::retry 循环内完成；本 stage 只提供
//! 单次尝试闭包（纯 IO）与结果落 ctx。

use crate::ForwardTask;
use crate::egress::ReqwestEgress;
use crate::stream::{self, pipe_chunk};
use async_trait::async_trait;
use bytes::Bytes;
use contract::error::NormalizedError;
use dispatch::retry::{AttemptOutcome, run_retry_loop};
use dispatch::{Dispatch, DispatchError, FailureClass, RetryPolicy};
use gateway_pipeline::ctx::{BodySource, SelectedRoute, UpstreamResponse};
use gateway_pipeline::{PipeStream, RequestCtx, Stage, StageError, StageOutcome};
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_proxy::ProxyManager;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// 转发 stage — 依赖 egress（测试 mock / 无代理）或 [`ProxyManager`] 租约 Client。
pub struct ForwardStage {
    egress: Arc<dyn crate::egress::Egress>,
    timeouts: crate::egress::Timeouts,
    /// 厂商协议注册表；空 = 透传。
    adaptors: Arc<AdaptorRegistry>,
    /// 生产路径注入；`None` 时使用 `egress`（测试 mock）。
    proxies: Option<Arc<ProxyManager>>,
    /// `with_retry` 注入的调度器：`Some` 时 handle 驱动完整重试循环并忽略
    /// `ctx.route`；`None` 时保持单次转发语义（读 DispatchStage 写入的 route）。
    dispatch: Option<Arc<dyn Dispatch>>,
    /// 重试预算，仅 `dispatch` 为 `Some` 时生效。
    retry_policy: RetryPolicy,
}

/// meow `ProxyAdapter` → [`StreamDialer`] 适配。
///
/// `dial(host, port)` 语义对齐：meow 的 `Metadata.host/dst_port` 就是拨号目标，
/// 协议握手在 `dial_tcp` 内完成，返回流直接承载上层流量。
struct AdapterDialer(Arc<dyn gateway_proxy::ProxyAdapter>);

impl std::fmt::Debug for AdapterDialer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterDialer").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl crate::adapter_egress::StreamDialer for AdapterDialer {
    async fn dial(
        &self,
        host: &str,
        port: u16,
    ) -> std::io::Result<Box<dyn crate::adapter_egress::DialedStream>> {
        let meta = gateway_proxy::tcp_metadata(host, port);
        let conn = self
            .0
            .dial_tcp(&meta)
            .await
            .map_err(std::io::Error::other)?;
        Ok(Box::new(MeowConn(conn)))
    }
}

/// `Box<dyn ProxyConn>` → `DialedStream`：两个 trait bound 都是
/// AsyncRead + AsyncWrite + Unpin + Send + Sync，透传即可。
struct MeowConn(Box<dyn gateway_proxy::ProxyConn>);

impl tokio::io::AsyncRead for MeowConn {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for MeowConn {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

impl ForwardStage {
    /// 使用注入的 [`crate::egress::Egress`]（测试或无代理池）。
    pub fn new(egress: Arc<dyn crate::egress::Egress>, adaptors: Arc<AdaptorRegistry>) -> Self {
        Self {
            egress,
            timeouts: crate::egress::Timeouts::default(),
            adaptors,
            proxies: None,
            dispatch: None,
            retry_policy: RetryPolicy::default(),
        }
    }

    /// 模型请求按 `SelectedRoute.channel_key` 租出口 Client。
    pub fn with_proxies(mut self, proxies: Arc<ProxyManager>) -> Self {
        self.proxies = Some(proxies);
        self
    }

    /// 启用 failover 重试循环：handle 不再读 `ctx.route`（可与 DispatchStage
    /// 共存，但不依赖它），改为自己驱动 `dispatch::retry::run_retry_loop` —
    /// 每次尝试后向 `dispatch` 做健康回报，可重试失败换候选再来一轮。
    pub fn with_retry(mut self, dispatch: Arc<dyn Dispatch>, policy: RetryPolicy) -> Self {
        self.dispatch = Some(dispatch);
        self.retry_policy = policy;
        self
    }

    /// 按候选 + 已读请求体组装一次尝试的 [`ForwardTask`]。
    ///
    /// 字段组装逻辑与原单次路径一致：headers 留空（gate 已清洗，透传头由
    /// apps/gateway 组装），provider_type/extra_headers 从候选自带的完整解析取。
    fn build_task(
        candidate: &SelectedRoute,
        path: String,
        body: Bytes,
        stream: bool,
    ) -> ForwardTask {
        ForwardTask {
            candidate: candidate.clone(),
            path,
            headers: vec![],
            body,
            stream,
            provider_type: candidate.provider_type.clone(),
            extra_headers: crate::adapter::extra_headers_from_settings(&candidate.settings),
        }
    }

    async fn forward_task(&self, task: &ForwardTask) -> Result<crate::Forwarded, NormalizedError> {
        let Some(proxies) = self.proxies.as_ref() else {
            return crate::pipeline::forward_once(
                task,
                &*self.egress,
                &self.adaptors,
                &self.timeouts,
            )
            .await;
        };

        let lease = proxies.acquire(&task.candidate.unit.channel_key);
        let node_id = lease.node_id;
        // 出口二态：reqwest Client（direct/http/socks5）或协议适配器
        // （SS/Trojan/VLESS/VMess——meow dial_tcp 经 adapter_egress 桥成 HTTP）。
        let result = if let Some(adapter) = lease.adapter() {
            let egress = crate::adapter_egress::AdapterEgress::new(
                Arc::new(AdapterDialer(adapter)),
                Duration::from_secs(5),
            );
            crate::pipeline::forward_once(task, &egress, &self.adaptors, &self.timeouts).await
        } else if let Some(client) = lease.reqwest_client() {
            let egress = ReqwestEgress::with_client((*client).clone(), Duration::from_secs(5));
            crate::pipeline::forward_once(task, &egress, &self.adaptors, &self.timeouts).await
        } else {
            proxies.feedback(node_id, 502, false);
            return Err(contract::error::NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 502,
                retryable: true,
                message: "lease produced neither reqwest client nor adapter".into(),
            });
        };
        match &result {
            Ok(forwarded) => proxies.feedback(node_id, forwarded.status, false),
            Err(err) => {
                let transport_err = err.status == 502 || err.status == 504;
                proxies.feedback(node_id, err.status, transport_err);
            }
        }
        result
    }
}

#[async_trait]
impl Stage for ForwardStage {
    fn name(&self) -> &'static str {
        "forward"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        // 读 body (可重放: InMemory/OnDisk 都给出新 reader)。
        let body = match &ctx.request.body {
            BodySource::InMemory(b) => b.clone(),
            BodySource::OnDisk { path, len } => {
                let mut f = tokio::fs::File::open(path)
                    .await
                    .map_err(|e| StageError::Internal(anyhow::anyhow!("open body: {e}")))?;
                use tokio::io::AsyncReadExt;
                let mut buf = vec![0u8; *len as usize];
                f.read_exact(&mut buf)
                    .await
                    .map_err(|e| StageError::Internal(anyhow::anyhow!("read body: {e}")))?;
                Bytes::from(buf)
            }
        };
        // 流式由请求体的 `stream` 字段决定 —— 路径里的 "stream" 子串不是协议信号。
        let stream = body_wants_stream(&body);

        // with_retry 接线后自己驱动整段重试循环, 完全忽略 ctx.route。
        if let Some(dispatch) = self.dispatch.clone() {
            return self.handle_with_retry(ctx, dispatch, body, stream).await;
        }

        // 单次模式: 候选由 DispatchStage 写入, 已解析完整 (secret / upstream_model /
        // provider_type / settings), forward 不查快照也不自造。
        let candidate = match &ctx.route {
            Some(r) => r.clone(),
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "dispatch stage did not run before forward"
                )));
            }
        };
        let task = Self::build_task(&candidate, ctx.request.path.clone(), body, stream);

        let forwarded = self
            .forward_task(&task)
            .await
            .map_err(normalized_to_stage_error)?;

        self.commit_forwarded(ctx, forwarded, &task.candidate, task.stream)
            .await
    }
}

impl ForwardStage {
    /// 重试模式主循环：调 `dispatch::retry::run_retry_loop` 驱动
    /// 「选候选 → 单次尝试 → 健康回报 → 排除已试 → 再选」。
    ///
    /// 尝试闭包是纯 IO：成功把 `Forwarded` 存进 `result_slot`（循环结束后由
    /// 本函数取出提交），失败把 `NormalizedError` 暂存进 `error_slot` 并按
    /// `retryable` 返回 Retryable/Fatal。回报闭包把同一结果转给
    /// `Dispatch::report`，驱动 dispatch 侧健康状态机。
    async fn handle_with_retry(
        &self,
        ctx: &mut RequestCtx,
        dispatch: Arc<dyn Dispatch>,
        body: Bytes,
        stream: bool,
    ) -> Result<StageOutcome, StageError> {
        let group = ctx
            .token
            .as_ref()
            .map(|t| t.group.clone())
            .unwrap_or_default();
        // 模型名由 gate::model 提升到 ctx; 未解析出来无法选候选, 属装配错误。
        let model = match ctx.requested_model.clone() {
            Some(m) => m,
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "model not resolved before forward"
                )));
            }
        };
        let path = ctx.request.path.clone();

        // 获胜尝试的 Forwarded 与最近一次失败, 短临界区 std Mutex (不跨 await 持锁)。
        let result_slot: Arc<Mutex<Option<crate::Forwarded>>> = Arc::new(Mutex::new(None));
        let error_slot: Arc<Mutex<Option<NormalizedError>>> = Arc::new(Mutex::new(None));

        let dsel = Arc::clone(&dispatch);
        let drep = Arc::clone(&dispatch);
        let slot = Arc::clone(&result_slot);
        let eslot = Arc::clone(&error_slot);

        let loop_result = run_retry_loop(
            &group,
            &model,
            &self.retry_policy,
            move |g, m, exclude| dsel.select(g, m, exclude),
            move |candidate| {
                let task = Self::build_task(candidate, path.clone(), body.clone(), stream);
                let slot = Arc::clone(&slot);
                let eslot = Arc::clone(&eslot);
                async move {
                    match self.forward_task(&task).await {
                        Ok(forwarded) => {
                            let status = forwarded.status;
                            *lock(&slot) = Some(forwarded);
                            AttemptOutcome::Done { status }
                        }
                        Err(e) => {
                            let retryable = e.retryable;
                            *lock(&eslot) = Some(e);
                            if retryable {
                                AttemptOutcome::Retryable(FailureClass::Retryable)
                            } else {
                                AttemptOutcome::Fatal(FailureClass::Fatal)
                            }
                        }
                    }
                }
            },
            move |key, outcome| drep.report(key, outcome),
        )
        .await;

        match loop_result {
            Ok((attempt, outcome)) => {
                let forwarded = lock(&result_slot).take();
                match (forwarded, outcome) {
                    (Some(f), _) => {
                        self.commit_forwarded(ctx, f, &attempt.candidate, stream)
                            .await
                    }
                    // 理论上只有 Fatal 会走到这里: 循环以客户端错误终止, 没有
                    // 成功响应可提交, 用暂存的 NormalizedError 透传上游状态码。
                    (None, AttemptOutcome::Fatal(_)) => {
                        let e = lock(&error_slot).take().ok_or_else(|| {
                            StageError::Internal(anyhow::anyhow!("forward produced no result"))
                        })?;
                        Err(normalized_to_stage_error(e))
                    }
                    (None, _) => Err(StageError::Internal(anyhow::anyhow!(
                        "retry loop finished without a forwarded response"
                    ))),
                }
            }
            // 预算耗尽 / 无候选 / 快照未就绪 / 全限流的映射 (E1, 与 DispatchStage 的
            // 单次映射区分: 这里 RetriesExhausted → 502, RateLimited → 429)。
            Err(e) => Err(match e {
                DispatchError::RateLimited { .. } => StageError::RateLimited,
                DispatchError::RetriesExhausted { .. } => {
                    // 带上最后一个上游错误诊断 (ocr: 静态串丢失真实状态码, 排障困难)。
                    StageError::Upstream(match lock(&error_slot).take() {
                        Some(last) => gateway_pipeline::UpstreamError::Status {
                            code: last.status,
                            body_preview: format!(
                                "retry budget exhausted; last upstream: {}",
                                last.message
                            )
                            .into_bytes(),
                        },
                        None => gateway_pipeline::UpstreamError::Status {
                            code: 502,
                            body_preview: b"retry budget exhausted".to_vec(),
                        },
                    })
                }
                DispatchError::NoCandidate { .. } => StageError::NoRoute,
                DispatchError::SnapshotNotReady => StageError::NotReady,
            }),
        }
    }

    /// 提交一次成功尝试的上游响应。
    ///
    /// 非流式 → 收全 body 写入 `ctx.upstream`；流式 → 经 SseScanner +
    /// StreamScanner 扫描链 unfold 成 [`StageOutcome::Stream`] 直接回流。
    /// `candidate` 是获胜候选（流式上下文要它的 channel_key/upstream_model）。
    async fn commit_forwarded(
        &self,
        ctx: &mut RequestCtx,
        forwarded: crate::Forwarded,
        candidate: &SelectedRoute,
        stream: bool,
    ) -> Result<StageOutcome, StageError> {
        if stream {
            // 流式路径经 SseScanner + StreamScanner 扫描链，流结束时自动结算
            let user_key = ctx
                .token
                .as_ref()
                .map(|t| t.id.to_string())
                .unwrap_or_default();
            let token_key = ctx
                .token
                .as_ref()
                .map(|t| t.id.to_string())
                .unwrap_or_default();
            let channel_key = candidate.unit.channel_key.clone();
            let public_model = candidate.unit.public_model.clone();
            let upstream_model = candidate.upstream_model.clone();

            let mut sse_ctx = stream::SseContext::new();
            sse_ctx.user_key = user_key;
            sse_ctx.token_key = token_key;
            sse_ctx.channel_key = channel_key;
            sse_ctx.public_model = public_model;
            sse_ctx.upstream_model = upstream_model;

            // 上游的 content-type 原样回给客户端: SSE 客户端靠它判定按事件流读。
            let content_type = forwarded.content_type.clone();
            let mapped = futures_util::stream::unfold(
                (forwarded.body, sse_ctx),
                move |(mut s, mut ctx)| async move {
                    use futures_util::StreamExt;
                    match s.next().await {
                        Some(Ok(chunk)) => {
                            let out = pipe_chunk(&mut ctx, &chunk);
                            Some((Ok::<Bytes, std::io::Error>(out.passthrough), (s, ctx)))
                        }
                        Some(Err(e)) => Some((Err(e), (s, ctx))),
                        None => {
                            let _ = stream::finish(ctx);
                            None
                        }
                    }
                },
            );
            let stream: futures_util::stream::BoxStream<'static, Result<Bytes, std::io::Error>> =
                Box::pin(mapped);
            Ok(StageOutcome::Stream(PipeStream::with_content_type(
                axum::body::Body::from_stream(stream),
                content_type,
            )))
        } else {
            // 读取全部 body (非流式响应体通常较小)。
            use futures_util::TryStreamExt;
            let mut buf = Vec::new();
            let mut body_stream = forwarded.body;
            while let Some(chunk) = body_stream
                .try_next()
                .await
                .map_err(|e| StageError::Internal(anyhow::anyhow!("read upstream body: {e}")))?
            {
                buf.extend_from_slice(&chunk);
            }
            ctx.upstream = Some(UpstreamResponse {
                status: forwarded.status,
                body: Bytes::from(buf),
            });
            Ok(StageOutcome::Continue)
        }
    }
}

/// 取 Mutex 值； poisoning 只可能来自 panic 的传播路径，恢复内部值即可
/// （两个 slot 都是纯赋值，不存在逻辑不一致状态）。
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// NormalizedError → StageError::Upstream（透传上游状态码与消息预览）。
fn normalized_to_stage_error(e: NormalizedError) -> StageError {
    StageError::Upstream(gateway_pipeline::UpstreamError::Status {
        code: e.status,
        body_preview: e.message.into_bytes(),
    })
}

/// 请求体是否要求流式响应 —— 读顶层 `stream` 布尔字段。
///
/// OpenAI / Claude / Gemini(OpenAI 兼容层) 都用这个字段表达流式意图。
/// 非 JSON 或缺字段 → false（按非流式处理，整体收 body 再回）。
///
/// 不能用 URL 路径判断：`/v1/chat/completions` 带 `"stream": true` 是流式，
/// 而路径里出现 "stream" 子串并不代表客户端要 SSE。
fn body_wants_stream(body: &Bytes) -> bool {
    #[derive(serde::Deserialize)]
    struct StreamFlag {
        #[serde(default)]
        stream: bool,
    }
    serde_json::from_slice::<StreamFlag>(body)
        .map(|f| f.stream)
        .unwrap_or(false)
}
