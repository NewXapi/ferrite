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
    /// 流式元数据（结算用）。
    pub user_key: String,
    pub token_key: String,
    pub channel_key: String,
    pub public_model: String,
    pub upstream_model: String,
    /// 请求分组名（进 `PriceTable::lookup`），默认空串。
    pub group: String,
    /// 分组倍率（GroupRecord.rate_multiplier），库层默认 1.0，
    /// 真实值由 apps 侧注入。
    pub group_ratio: f64,
    /// 定价表引用（可选，None = 不计费）。
    pub price_table: Option<std::sync::Arc<dyn metering::pricing::PriceTable>>,
}

impl SseContext {
    pub fn new() -> Self {
        Self {
            scanner: gateway_protocol_bridge::sse::SseScanner::default(),
            token_scanner: metering::scanner::StreamScanner::new(),
            user_key: String::new(),
            token_key: String::new(),
            channel_key: String::new(),
            public_model: String::new(),
            upstream_model: String::new(),
            group: String::new(),
            group_ratio: 1.0,
            price_table: None,
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

/// 终止扫描器，返回 SseEnd + TokenCounts；`price_table` 为 `Some` 时**真正**
/// 结算并把产物返回给调用方（`None` = 不计费，与未计费时代行为一致）。
///
/// `status_code` / `error` 由调用方注入终态：流正常读完传 `200, None`；
/// 中途出错（上游断流）传 `500, Some(摘要)`，用已累积的计数结算，避免
/// 泄漏已产生 token 的账单。
pub fn finish(
    ctx: SseContext,
    status_code: u16,
    error: Option<&str>,
) -> (
    gateway_protocol_bridge::sse::SseEnd,
    metering::scanner::TokenCounts,
    Option<contract::records::UsageEventRecord>,
) {
    let end = ctx.scanner.finish();
    let counts = ctx.token_scanner.finish(0);

    let event = ctx.price_table.as_ref().map(|pt| {
        // 库层不做预扣（prehold 属 admission 轨），amount=0 表"事后结算非
        // 预扣"：settle_event 只消费 hold 的归因键 user_key/token_key，cost
        // 由 counts × 价格计算，amount 字段不参与本次结算。
        let hold = metering::ledger::Hold {
            id: 0,
            amount: 0,
            user_key: ctx.user_key.clone(),
            token_key: ctx.token_key.clone(),
        };
        metering::settle_event(
            counts,
            &ctx.group,
            ctx.group_ratio,
            &hold,
            pt.as_ref(),
            &ctx.channel_key,
            "",
            &ctx.public_model,
            &ctx.upstream_model,
            0,
            0,
            status_code,
            error,
        )
    });

    (end, counts, event)
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
