//! 单次尝试转发管道 — forward 的核心编排。
//!
//! 管道 (一去一回):
//! ```text
//! ForwardTask
//!   → adapter::prepare (URL 拼接 / 鉴权头 / 渠道参数覆盖)
//!   → protocol::Registry::resolve (跨协议时转换请求体)
//!   → egress::execute (带超时发送)
//!   → [response]
//!       ├─ 非流式: 整体回传 + metering 结算
//!       └─ 流式:   stream::pipe (SseScanner → StreamScanner → 客户端)
//! ```
//!
//! 失败出口统一为 protocol::NormalizedError → dispatch::FailureClass。

use crate::ForwardTask;
use crate::adapter::{self, PreparedRequest};
use crate::egress::{Egress, ForwardedResponse};
use bytes::Bytes;
use contract::error::NormalizedError;
use std::sync::Arc;

/// 合并后的请求头 (adapter 鉴权 + 渠道覆盖 + 客户端已过滤头)。
pub fn merge_headers(
    prepared: &PreparedRequest,
    client_headers: &[(String, String)],
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> =
        Vec::with_capacity(2 + client_headers.len() + prepared.extra_headers.len());
    out.push(prepared.auth_header.clone());
    out.extend(prepared.extra_headers.iter().cloned());
    out.extend(client_headers.iter().cloned());
    out
}

/// 读完整响应 body — 仅非流式路径使用。
async fn read_all_body(resp: ForwardedResponse) -> Result<Bytes, std::io::Error> {
    use futures_util::TryStreamExt;
    let mut body = resp.into_body_stream();
    let mut buf = Vec::new();
    while let Some(chunk) = body.try_next().await? {
        buf.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buf))
}

/// 单次转发的核心编排 — prepare + execute, 不做协议转换 (留给 protocol-bridge)。
///
/// 流式 / 非流式统一返回 `Forwarded`: 非流式响应也按字节流形状给出 (单 chunk)。
///
/// `body_stream` 决定响应形态:
/// - 流式 (`stream=true`) → 直接接 `egress.execute` 的字节流
/// - 非流式 (`stream=false`) → `read_all_body` 一次性收集
///
/// TODO(#530): 流式路径在 stream 模块内挂 SseScanner / StreamScanner。
/// TODO(#334): 非流式响应也需走 metering 估算 prompt token。
pub async fn forward_once(
    task: &ForwardTask,
    egress: &dyn Egress,
    timeouts: &crate::egress::Timeouts,
) -> Result<crate::Forwarded, NormalizedError> {
    let prepared = adapter::prepare(
        &task.candidate,
        &task.path,
        &task.provider_type,
        task.extra_headers.clone(),
    );
    let merged = merge_headers(&prepared, &task.headers);

    // 请求体预扫描 (透传给 metering 估算用, 当前实现原样返回)。
    let body = capture_prompt_body(&task.body);

    let resp = egress
        .execute(&prepared.url, &merged, body, timeouts)
        .await?;

    let status = resp.status();
    let content_type = resp.content_type().to_string();

    let body_stream: std::pin::Pin<
        Box<dyn futures_util::Stream<Item = Result<Bytes, std::io::Error>> + Send>,
    > = if task.stream {
        // 流式: 直接接上游字节流, 客户端 Accept 决定上游按 SSE 推。
        // pipe_chunk 串联由 stream 模块处理; 这里只透传。
        Box::pin(resp.into_body_stream())
    } else {
        // 非流式: 整体读 body 后给单 chunk 流 (简化客户端消费路径)。
        // ponytail: 缓冲整 body 仅适用非流式场景 (常规 API 调用, body 较小);
        // 流式场景走上面分支。
        let bytes = match read_all_body(resp).await {
            Ok(b) => b,
            Err(e) => {
                return Err(NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: 502,
                    retryable: true,
                    message: format!("read upstream body: {e}"),
                });
            }
        };
        Box::pin(futures_util::stream::iter(std::iter::once(Ok(bytes))))
    };

    Ok(crate::Forwarded {
        status,
        body: body_stream,
        content_type,
    })
}

/// 管道执行 trait — apps/gateway 用 reqwest 实现。
pub trait Pipeline: Send + Sync {
    /// 执行一次转发尝试。不重试 (重试是 dispatch::retry 的事)。
    fn execute(
        &self,
        task: &ForwardTask,
        timeouts: &crate::egress::Timeouts,
    ) -> impl Future<Output = Result<crate::Forwarded, NormalizedError>> + Send;
}

/// 默认 reqwest 管道 — 持有 `Egress` 实例。
///
/// ponytail: 未来 apps/gateway 可以传入自定义 Egress (代理池/限速 Client);
/// 当前一个全局 client 已足够。
#[derive(Clone)]
pub struct ReqwestPipeline {
    egress: Arc<dyn Egress>,
}

impl std::fmt::Debug for ReqwestPipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReqwestPipeline").finish_non_exhaustive()
    }
}

impl ReqwestPipeline {
    pub fn new(egress: Arc<dyn Egress>) -> Self {
        Self { egress }
    }
}

impl Pipeline for ReqwestPipeline {
    async fn execute(
        &self,
        task: &ForwardTask,
        timeouts: &crate::egress::Timeouts,
    ) -> Result<crate::Forwarded, NormalizedError> {
        forward_once(task, self.egress.as_ref(), timeouts).await
    }
}

/// 请求体预扫描 — metering 估算 prompt token 的输入挂点。
///
/// 在 adapt_request 之后、发送之前调用一次 (适配后的体才是上游真实体)。
///
/// 当前 V1 范围: 透传语义, 不解析 (metering 接口未定型)。适配协议体的
/// transform 仍由 protocol-bridge 在 ForwardTask 进入 pipeline 前完成。
/// TODO(#334): 与 metering::estimate_prompt_tokens 对接。
pub fn capture_prompt_body(body: &bytes::Bytes) -> bytes::Bytes {
    body.clone()
}
