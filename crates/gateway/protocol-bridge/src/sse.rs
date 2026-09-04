//! SSE 帧扫描器 — 逐字保真转发 + 事件检测。
//!
//! 保证 (对齐 new-api scan_sse.go):
//! - 转发字节与上游逐字一致 (含厂商私有帧), 不重排不丢空白;
//! - 不伪造 [DONE]: 上游截断就是截断, 报告 Truncated;
//! - 写侧有界: 下游背压时不无界缓冲上游 (TODO(#502): 水位线参数)。
//!
//! 与 metering::StreamScanner 的分工: 本类型只管**帧**
//! (event/data/id 边界、keepalive、终止), token 计数是 metering 的事。

use bytes::Bytes;

/// SSE 扫描器: 逐块喂入, 返回透传字节与检测到的事件。
#[derive(Debug, Default)]
pub struct SseScanner {
    /// 行缓冲残余 (跨 chunk 的不完整行)。
    _line_buf: Vec<u8>,
    /// 是否已报 FirstToken (幂等, 只报一次)。
    _saw_first_token: bool,
}

impl SseScanner {
    /// 喂入一块上游字节。
    ///
    /// 返回 (透传字节, 检测到的事件列表)。透传字节 = 输入原样 (零拷贝语义,
    /// 实现期保证), 事件是对同一段字节的并行观察。
    pub fn push(&mut self, chunk: &Bytes) -> (Bytes, Vec<SseEvent>) {
        let _ = (&mut self._line_buf, &mut self._saw_first_token);
        (chunk.clone(), Vec::new()) // TODO(#502): 行状态机实现
    }

    /// 上游断开: 报告终止原因。
    pub fn finish(self) -> SseEnd {
        SseEnd::Truncated // TODO(#502): 依据缓冲是否完整判定 Clean/Truncated
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    /// 检测到首个可见 token (含 tool-call delta) — TTFT 计量点。
    FirstToken,
    /// usage 字段出现 (OpenAI stream_options / Claude message_delta)。
    Usage,
    /// 纯 keepalive 帧。
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseEnd {
    Clean,
    Truncated,
    Errored,
}
