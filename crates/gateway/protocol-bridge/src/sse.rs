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
    line_buf: Vec<u8>,
    /// 是否已报 FirstToken (幂等, 只报一次)。
    saw_first_token: bool,
    /// 是否收到过 [DONE]。
    saw_done: bool,
    /// 行数计数。
    line_count: u64,
}

impl SseScanner {
    /// 喂入一块上游字节。
    pub fn push(&mut self, chunk: &Bytes) -> (Bytes, Vec<SseEvent>) {
        let mut events = Vec::new();
        self.line_count += 1;

        let mut full = self.line_buf.clone();
        full.extend_from_slice(chunk);

        let mut lines: Vec<&[u8]> = Vec::new();
        let mut start = 0;
        for (i, &b) in full.iter().enumerate() {
            if b == b'\n' {
                lines.push(&full[start..i]);
                start = i + 1;
            }
        }
        if start < full.len() {
            self.line_buf = full[start..].to_vec();
        } else {
            self.line_buf.clear();
        }

        for line in lines {
            let line = if line.ends_with(b"\r") {
                &line[..line.len() - 1]
            } else {
                line
            };

            if line.is_empty() {
                continue;
            }

            if line.starts_with(b":") {
                events.push(SseEvent::Ping);
                continue;
            }

            if let Some(rest) = line.strip_prefix(b"data") {
                let rest = if let Some(r) = rest.strip_prefix(b": ") {
                    r
                } else if let Some(r) = rest.strip_prefix(b":") {
                    r
                } else {
                    continue;
                };
                let rest = rest.trim_ascii_start();

                if rest.starts_with(b"[DONE]") {
                    self.saw_done = true;
                    continue;
                }

                if !self.saw_first_token {
                    self.saw_first_token = true;
                    events.push(SseEvent::FirstToken);
                }

                if rest.windows(7).any(|w| w == b"\"usage\"") {
                    events.push(SseEvent::Usage);
                }
            }
        }

        (chunk.clone(), events)
    }

    /// 上游断开: 报告终止原因。
    pub fn finish(self) -> SseEnd {
        if self.saw_done {
            SseEnd::Clean
        } else {
            SseEnd::Truncated
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SseEvent {
    FirstToken,
    Usage,
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SseEnd {
    Clean,
    Truncated,
    Errored,
}
