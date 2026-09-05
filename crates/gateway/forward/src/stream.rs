//! 流式双向管道 — 上游 SSE → 扫描链 → 客户端。

use bytes::Bytes;
use futures_util::{FutureExt, Stream, StreamExt};
use std::pin::Pin;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub struct PipedChunk {
    pub passthrough: Bytes,
    pub events: Vec<gateway_protocol_bridge::sse::SseEvent>,
}

<<<<<<< Updated upstream
pub struct SseContext {
    pub scanner: gateway_protocol_bridge::sse::SseScanner,
    pub token_scanner: metering::scanner::StreamScanner,
    /// 流式元数据（结算用）。
    pub user_key: String,
    pub token_key: String,
    pub channel_key: String,
    pub public_model: String,
    pub upstream_model: String,
    /// 定价表引用（可选，None = 不计费）。
    pub price_table: Option<std::sync::Arc<dyn metering::pricing::PriceTable>>,
=======
/// SSE 扫描上下文 — 单次流式转发的扫描链状态。
///
/// 链路: upstream chunk → SseScanner (帧边界) → StreamScanner (usage/token) → client.
pub struct SseContext {
    pub scanner: gateway_protocol_bridge::sse::SseScanner,
    pub token_scanner: metering::scanner::StreamScanner,
>>>>>>> Stashed changes
}

impl SseContext {
    pub fn new() -> Self {
        Self {
            scanner: gateway_protocol_bridge::sse::SseScanner::default(),
            token_scanner: metering::scanner::StreamScanner::new(),
<<<<<<< Updated upstream
            user_key: String::new(),
            token_key: String::new(),
            channel_key: String::new(),
            public_model: String::new(),
            upstream_model: String::new(),
            price_table: None,
=======
>>>>>>> Stashed changes
        }
    }
}

impl Default for SseContext {
    fn default() -> Self {
        Self::new()
    }
}

<<<<<<< Updated upstream
=======
/// 把一块上游字节推过扫描链 — 透传 + 检测事件。
>>>>>>> Stashed changes
pub fn pipe_chunk(ctx: &mut SseContext, chunk: &Bytes) -> PipedChunk {
    let (passthrough, events) = ctx.scanner.push(chunk);
    ctx.token_scanner.push(chunk);
    PipedChunk {
        passthrough,
        events,
    }
}

<<<<<<< Updated upstream
/// 终止扫描器，返回 SseEnd + TokenCounts，并在有 price_table 时自动结算。
pub fn finish(
    ctx: SseContext,
) -> (
    gateway_protocol_bridge::sse::SseEnd,
    metering::scanner::TokenCounts,
) {
    let end = ctx.scanner.finish();
    let counts = ctx.token_scanner.finish(0);

    if let Some(pt) = ctx.price_table.as_ref() {
        let hold = metering::ledger::Hold {
            id: 0,
            amount: 0,
            user_key: ctx.user_key.clone(),
            token_key: ctx.token_key.clone(),
        };
        let _event = metering::settle_event(
            counts,
            &hold,
            pt.as_ref(),
            &ctx.channel_key,
            "",
            &ctx.public_model,
            &ctx.upstream_model,
            0,
            0,
            200,
            None,
        );
    }

=======
/// 终止扫描器并报终止原因 + token 计数。
pub fn finish(ctx: SseContext) -> (gateway_protocol_bridge::sse::SseEnd, metering::scanner::TokenCounts) {
    let end = ctx.scanner.finish();
    // prompt 由请求体预扫得出, 这里传 0 作为占位
    let counts = ctx.token_scanner.finish(0);
>>>>>>> Stashed changes
    (end, counts)
}

#[derive(Debug)]
pub struct AbortGuard {
    cancel: CancellationToken,
    _stop_tx: Option<oneshot::Sender<()>>,
}

impl AbortGuard {
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let (tx, rx) = oneshot::channel();
        let guard = Self {
            cancel: CancellationToken::new(),
            _stop_tx: Some(tx),
        };
        (guard, rx)
    }

    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    pub async fn cancelled(&self) {
        self.cancel.cancelled().await;
    }
}

impl Default for AbortGuard {
    fn default() -> Self {
        Self {
            cancel: CancellationToken::new(),
            _stop_tx: None,
        }
    }
}

impl Drop for AbortGuard {
    fn drop(&mut self) {
        self.cancel.cancel();
    }
}

pub fn abortable_stream<S>(inner: S, stop: oneshot::Receiver<()>) -> AbortableStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    AbortableStream { inner, stop }
}

pub struct AbortableStream<S> {
    inner: S,
    stop: oneshot::Receiver<()>,
}

impl<S> Stream for AbortableStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    type Item = Result<Bytes, std::io::Error>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.inner.poll_next_unpin(cx) {
            std::task::Poll::Ready(item) => std::task::Poll::Ready(item),
            std::task::Poll::Pending => match self.stop.poll_unpin(cx) {
                std::task::Poll::Ready(_) => std::task::Poll::Ready(None),
                std::task::Poll::Pending => std::task::Poll::Pending,
            },
        }
    }
}
