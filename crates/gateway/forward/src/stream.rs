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

/// 管道输出 — 每块透传字节 + 检测信号 (供 metering/diagnostics 消费)。
pub struct PipedChunk {
    /// 原样透传给客户端的字节。
    pub passthrough: Bytes,
    /// 扫描链检测到的事件 (FirstToken/Usage/Ping)。
    pub events: Vec<protocol::sse::SseEvent>,
}

/// 把一块上游字节推过扫描链。
/// TODO(#530): SseScanner 与 StreamScanner 的串联实现; 纯转发场景
/// (非 SSE) 直接 passthrough。
pub fn pipe_chunk(_chunk: &Bytes) -> PipedChunk {
    PipedChunk { passthrough: Bytes::new(), events: Vec::new() }
}

/// AbortGuard — 客户端断开时取消上游拉取。
/// TODO(#531): tokio::select! + CancellationToken 模式; 断开时已收 usage 仍结算。
pub struct AbortGuard {
    _priv: (),
}
