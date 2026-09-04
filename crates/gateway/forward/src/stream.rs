//! 流式双向管道 — 上游 SSE → 扫描链 → 客户端。
//!
//! 链路 (每个 Bytes 块依次过三站, 全程零物化):
//! ```text
//! upstream chunk
//!   → protocol::SseScanner::push     (帧边界/keepalive/FirstToken 事件)
//!   → metering::StreamScanner::push  (usage 捕获 + token 计数)
//!   → client write                   (原始字节, 逐字保真)
//! ```
//!
//! 断开语义 (对齐 new-api scan_sse.go):
//! - 上游 EOF 干净 → 客户端正常收尾 (不追加 [DONE]);
//! - 上游截断 → 照样 flush 已收字节 + 报 SseEnd::Truncated;
//! - 客户端断开 → 取消上游 (AbortGuard), 已计 usage 照常结算。

use bytes::Bytes;
use futures_util::Stream;
use std::pin::Pin;
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

/// 管道输出 — 每块透传字节 + 检测信号 (供 metering/diagnostics 消费)。
#[derive(Debug, Clone)]
pub struct PipedChunk {
    /// 原样透传给客户端的字节。
    pub passthrough: Bytes,
    /// 扫描链检测到的事件 (FirstToken/Usage/Ping)。
    pub events: Vec<gateway_protocol_bridge::sse::SseEvent>,
}

/// SSE 扫描上下文 — 单次流式转发的扫描链状态。
///
/// V1 范围: 只挂 SseScanner; metering::StreamScanner 接入留 TODO(#530)。
/// 当前 `pipe_chunk` 接受 chunk 与 `&mut SseScanner`, 与 forward::stream
/// 的无状态函数签名分开 (本类型用于 pipeline / future 单元)。
pub struct SseContext {
    pub scanner: gateway_protocol_bridge::sse::SseScanner,
}

impl SseContext {
    pub fn new() -> Self {
        Self {
            scanner: gateway_protocol_bridge::sse::SseScanner::default(),
        }
    }
}

impl Default for SseContext {
    fn default() -> Self {
        Self::new()
    }
}

/// 把一块上游字节推过扫描链 — 透传 + 检测事件。
///
/// 当前 V1 行为:
/// - 始终透传 `chunk.clone()` (字节保真, 零物化)
/// - 调 `SseScanner::push` 拿检测事件
///
/// 非 SSE 内容 (text/plain, application/json 等) 当前 V1 也按 passthrough 处理
/// (扫描器自然空事件); protocol crate 已在 V1 范围把扫描器保持 idle-friendly。
///
/// TODO(#530): StreamScanner::push 串联; 当前仅一站 (SseScanner)。
pub fn pipe_chunk(ctx: &mut SseContext, chunk: &Bytes) -> PipedChunk {
    let (passthrough, events) = ctx.scanner.push(chunk);
    PipedChunk {
        passthrough,
        events,
    }
}

/// 终止扫描器并报终止原因。
///
/// 上游 EOF → 调 `SseScanner::finish()` 拿 `SseEnd`。
/// ponytail: 调用方必须消费返回值 (供 diagnostics / 计量); 丢弃 = 失信号。
pub fn finish(ctx: SseContext) -> gateway_protocol_bridge::sse::SseEnd {
    ctx.scanner.finish()
}

/// AbortGuard — 客户端断开时取消上游拉取。
///
/// V1 设计: `oneshot` + `CancellationToken` 双通道。
/// - `watcher`: 由上游响应流持有; 收到取消信号时立即 drop 拉取 (reqwest body
///   drop 即断连)
/// - `cancel()`: 客户端断开路径调用; 触发 watcher 唤醒 + 通知上游
///
/// 已收字节/usage 由 `pipe_chunk` 串行累积, 与取消无关 (消费已落内存的
/// 字节)。`on_drop` 钩子在 drop 时自动 cancel, 兜底泄漏路径。
#[derive(Debug)]
pub struct AbortGuard {
    cancel: CancellationToken,
    /// 通知上游响应流停止拉取的 oneshot (持有者负责 drop / await)。
    _stop: Option<oneshot::Sender<()>>,
}

impl AbortGuard {
    /// 新建 AbortGuard + 上游 stop oneshot。
    ///
    /// 返回 `(AbortGuard, StopReceiver)`: guard 留在调用方, receiver 交上游拉取。
    pub fn new() -> (Self, oneshot::Receiver<()>) {
        let cancel = CancellationToken::new();
        let (stop_tx, stop_rx) = oneshot::channel();
        (
            Self {
                cancel,
                _stop: Some(stop_tx),
            },
            stop_rx,
        )
    }

    /// 主动取消 (客户端断开 / 上游失败)。
    pub fn cancel(&self) {
        self.cancel.cancel();
    }

    /// 监听取消信号 — 返回的 future 在 cancel 触发时 resolve。
    pub fn cancelled(&self) -> tokio_util::sync::WaitForCancellationFuture<'_> {
        self.cancel.cancelled()
    }

    /// 是否已取消。
    pub fn is_cancelled(&self) -> bool {
        self.cancel.is_cancelled()
    }

    /// 派生子 token — 上游拉取子句独立取消。
    pub fn child_token(&self) -> CancellationToken {
        self.cancel.child_token()
    }
}

impl Default for AbortGuard {
    fn default() -> Self {
        let _ = oneshot::channel::<()>();
        Self::new().0
    }
}

impl Drop for AbortGuard {
    fn drop(&mut self) {
        // 兜底: guard 被 drop (请求终止) → 自动取消
        self.cancel.cancel();
    }
}

/// 把字节流包成"客户端断开即取消"的包装流。
///
/// `stop` 来自 `AbortGuard::new()` 的 `oneshot::Receiver`; 一旦收到停止信号
/// (或上游 guard 取消), 包装流立刻停止产出后续字节, 已产出的字节仍 flush 给
/// 调用方。
///
/// ponytail: 用 `tokio::select!` 在 `next()` 内抢 stop; 不另起 task (轻量、无 spawn 开销)。
pub fn abortable_stream<S>(inner: S, stop: oneshot::Receiver<()>) -> AbortableStream<S>
where
    S: Stream<Item = Result<Bytes, std::io::Error>> + Unpin,
{
    AbortableStream {
        inner,
        stop: Some(stop),
        done: false,
    }
}

/// `abortable_stream` 返回的包装流。
pub struct AbortableStream<S> {
    inner: S,
    stop: Option<oneshot::Receiver<()>>,
    done: bool,
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
        // 已标记 done → 直接 None (后续 poll 仍 None, 兼容下游重复 poll)。
        if self.done {
            return std::task::Poll::Ready(None);
        }

        // 抢 stop 信号 (oneshot::Receiver::poll 返回 Poll<Result<(), RecvError>>)
        if let Some(stop) = self.stop.as_mut() {
            match Pin::new(stop).poll(cx) {
                std::task::Poll::Ready(Ok(())) | std::task::Poll::Ready(Err(_)) => {
                    self.done = true;
                    self.stop = None;
                    return std::task::Poll::Ready(None);
                }
                std::task::Poll::Pending => {}
            }
        }

        // 抢 inner 下一 chunk
        match Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(None) => {
                self.done = true;
                std::task::Poll::Ready(None)
            }
            std::task::Poll::Ready(Some(item)) => std::task::Poll::Ready(Some(item)),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
