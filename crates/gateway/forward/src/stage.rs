//! `stage` — ForwardStage: 把转发管道接入 pipeline
//!
//! 从 ctx.route (DispatchStage 写入) + RequestCtx 的 body/path 组装 ForwardTask,
//! 调用 forward_once 发上游, 结果写入 ctx.upstream (非流式) 或直接回流。
//!
//! 健康回报: 每次尝试结果调 dispatch::run_retry_loop 已覆盖; 本 stage 只做
//! 单次转发 + 结果落 ctx。重试编排在 retry 循环 (dispatch::retry), 不在此处。

use crate::ForwardTask;
use crate::egress::ReqwestEgress;
use crate::stream::{self, pipe_chunk};
use async_trait::async_trait;
use bytes::Bytes;
use contract::error::NormalizedError;
use gateway_pipeline::ctx::{BodySource, UpstreamResponse};
use gateway_pipeline::{PipeStream, RequestCtx, Stage, StageError, StageOutcome};
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use gateway_proxy::ProxyManager;
use std::sync::Arc;
use std::time::Duration;

/// 转发 stage — 依赖 egress（测试 mock / 无代理）或 [`ProxyManager`] 租约 Client。
pub struct ForwardStage {
    egress: Arc<dyn crate::egress::Egress>,
    timeouts: crate::egress::Timeouts,
    /// 厂商协议注册表；空 = 透传。
    adaptors: Arc<AdaptorRegistry>,
    /// 生产路径注入；`None` 时使用 `egress`（测试 mock）。
    proxies: Option<Arc<ProxyManager>>,
}

impl ForwardStage {
    /// 使用注入的 [`crate::egress::Egress`]（测试或无代理池）。
    pub fn new(egress: Arc<dyn crate::egress::Egress>, adaptors: Arc<AdaptorRegistry>) -> Self {
        Self {
            egress,
            timeouts: crate::egress::Timeouts::default(),
            adaptors,
            proxies: None,
        }
    }

    /// 模型请求按 `SelectedRoute.channel_key` 租出口 Client。
    pub fn with_proxies(mut self, proxies: Arc<ProxyManager>) -> Self {
        self.proxies = Some(proxies);
        self
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
        let egress = lease
            .reqwest_client()
            .map(|client| ReqwestEgress::with_client((*client).clone(), Duration::from_secs(5)));
        // Connector 变体（SS/Trojan 等）暂未接入 forward 直连管道，
        // 返回 502 让 retry 层换候选。PR3 打通 connector→forward 桥。
        let Some(egress) = egress else {
            proxies.feedback(node_id, 502, false);
            return Err(contract::error::NormalizedError {
                code: contract::error::code::UPSTREAM_ERROR,
                status: 502,
                retryable: true,
                message: "proxy connector not yet bridged to forward".into(),
            });
        };
        let result =
            crate::pipeline::forward_once(task, &egress, &self.adaptors, &self.timeouts).await;
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

        // 候选由 DispatchStage 写入, 已解析完整 (secret / upstream_model /
        // provider_type / settings), forward 不查快照也不自造。
        let candidate = match &ctx.route {
            Some(r) => r.clone(),
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "dispatch stage did not run before forward"
                )));
            }
        };
        let provider_type = candidate.provider_type.clone();
        let extra_headers = crate::adapter::extra_headers_from_settings(&candidate.settings);
        // 流式由请求体的 `stream` 字段决定 —— 路径里的 "stream" 子串不是协议信号。
        let stream = body_wants_stream(&body);

        let task = ForwardTask {
            candidate,
            path: ctx.request.path.clone(),
            headers: vec![], // gate 阶段已清洗, 透传头由 apps/gateway 组装
            body,
            stream,
            provider_type,
            extra_headers,
        };

        let forwarded = self
            .forward_task(&task)
            .await
            .map_err(|e: NormalizedError| {
                StageError::Upstream(gateway_pipeline::UpstreamError::Status {
                    code: e.status,
                    body_preview: e.message.into_bytes(),
                })
            })?;

        // 非流式 → 收 body 写入 ctx.upstream; 流式 → 交回客户端。
        if task.stream {
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
            let channel_key = task.candidate.unit.channel_key.clone();
            let public_model = task.candidate.unit.public_model.clone();
            let upstream_model = task.candidate.upstream_model.clone();

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
