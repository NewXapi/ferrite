//! `stage` — ForwardStage: 把转发管道接入 pipeline
//!
//! 从 ctx.route (DispatchStage 写入) + RequestCtx 的 body/path 组装 ForwardTask,
//! 调用 forward_once 发上游, 结果写入 ctx.upstream (非流式) 或直接回流。
//!
//! 健康回报: 每次尝试结果调 dispatch::run_retry_loop 已覆盖; 本 stage 只做
//! 单次转发 + 结果落 ctx。重试编排在 retry 循环 (dispatch::retry), 不在此处。

use crate::ForwardTask;
use async_trait::async_trait;
use bytes::Bytes;
use contract::error::NormalizedError;
use gateway_pipeline::ctx::{BodySource, UpstreamResponse};
use gateway_pipeline::{PipeStream, RequestCtx, Stage, StageError, StageOutcome};
use gateway_protocol_bridge::adaptor::AdaptorRegistry;
use std::sync::Arc;

/// 转发 stage — 依赖 egress (reqwest 出口) 与路径模板前缀。
pub struct ForwardStage {
    egress: Arc<dyn crate::egress::Egress>,
    timeouts: crate::egress::Timeouts,
    /// 厂商协议注册表；空 = 透传。
    adaptors: Arc<AdaptorRegistry>,
}

impl ForwardStage {
    pub fn new(egress: Arc<dyn crate::egress::Egress>, adaptors: Arc<AdaptorRegistry>) -> Self {
        Self {
            egress,
            timeouts: crate::egress::Timeouts::default(),
            adaptors,
        }
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

        let (base_url, _channel_id) = match &ctx.route {
            Some(r) => (r.base_url.clone(), r.channel_id),
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "dispatch stage did not run before forward"
                )));
            }
        };

        // 用 path 推导 provider_type (V1: apps/gateway 组装时从 ChannelRecord
        // 注入; 此处缺省 openai 透传, 跨协议转换留给 protocol-bridge)。
        let provider_type = "openai";

        let task = ForwardTask {
            candidate: dispatch::candidate::Candidate {
                unit: contract::records::RouteUnitRecord {
                    meta: contract::records::SyncMeta {
                        key: "forward-stage".into(),
                        schema_version: 1,
                        logical_version: 1,
                        origin: "gateway".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    group: ctx
                        .token
                        .as_ref()
                        .map(|t| t.group.clone())
                        .unwrap_or_default(),
                    public_model: ctx.request.path.clone(),
                    channel_key: String::new(),
                    key_index: 0,
                    upstream_model: String::new(),
                    priority: 0,
                    weight: 0,
                    status: 1,
                },
                secret: String::new(), // apps/gateway 组装时从 snapshot 注入
                base_url,
                upstream_model: String::new(),
            },
            path: ctx.request.path.clone(),
            headers: vec![], // gate 阶段已清洗, 透传头由 apps/gateway 组装
            body,
            stream: ctx.request.path.contains("stream"),
            provider_type: provider_type.into(),
            extra_headers: vec![],
        };

        let forwarded = crate::pipeline::forward_once(&task, &*self.egress, &self.adaptors, &self.timeouts)
            .await
            .map_err(|e: NormalizedError| {
                StageError::Upstream(gateway_pipeline::UpstreamError::Status {
                    code: e.status,
                    body_preview: e.message.into_bytes(),
                })
            })?;

        // 非流式 → 收 body 写入 ctx.upstream; 流式 → 交回客户端。
        if task.stream {
            // 直通: 流式响应直接回客户端, 保留上游字节流。
            // Forwarded.body 已是 'static (Box::pin), 直接 map_err 换错误类型。
            let stream = futures_util::stream::TryStreamExt::map_err(forwarded.body, |e| {
                std::io::Error::other(e.to_string())
            });
            let stream: futures_util::stream::BoxStream<'static, Result<Bytes, std::io::Error>> =
                Box::pin(stream);
            Ok(StageOutcome::Stream(PipeStream::new(
                axum::body::Body::from_stream(stream),
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
