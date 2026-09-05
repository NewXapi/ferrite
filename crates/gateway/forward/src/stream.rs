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

pub struct SseContext {
    pub scanner: gateway_protocol_bridge::sse::SseScanner,
    pub token_scanner: metering::scanner::StreamScanner,
}

impl SseContext {
    pub fn new() -> Self {
        Self {
            scanner: gateway_protocol_bridge::sse::SseScanner::default(),
            token_scanner: metering::scanner::StreamScanner::new(),
        }
    }
}

impl Default for SseContext {
    fn default() -> Self {
        Self::new()
    }
}

pub fn pipe_chunk(ctx: &mut SseContext, chunk: &Bytes) -> PipedChunk {
    let (passthrough, events) = ctx.scanner.push(chunk);
    ctx.token_scanner.push(chunk);
    PipedChunk {
        passthrough,
        events,
    }
}

pub fn finish(
    ctx: SseContext,
) -> (
    gateway_protocol_bridge::sse::SseEnd,
    metering::scanner::TokenCounts,
) {
    let end = ctx.scanner.finish();
    let counts = ctx.token_scanner.finish(0);
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
