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
    /// 是否已截断：[DONE] 之后的帧一律丢弃，不再入队。
    done_cutoff: bool,
    /// 行数计数。
    line_count: u64,
    /// 当前帧内累积的 `data:` 负载（多行以换行连接，符合 SSE 规范）。
    frame_data: Vec<String>,
    /// 已凑齐（遇到空行分隔符）的完整帧负载队列。
    ready_frames: Vec<String>,
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
                // 空行 = 帧分隔符：把累积的 data 负载封成一个完整帧。
                if !self.frame_data.is_empty() {
                    self.ready_frames.push(self.frame_data.join("\n"));
                    self.frame_data.clear();
                }
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
                    // [DONE] 是流终止符：同 chunk 里它之后的数据帧（SSE 规范不允许
                    // 但真实上游会犯）不得再进 ready_frames，否则会被取走编码、
                    // 让客户端在终止帧后收到内容。截断后续所有帧。
                    self.done_cutoff = true;
                    continue;
                }
                // 已终止的流不再接收任何数据帧。
                if self.done_cutoff {
                    continue;
                }

                if !self.saw_first_token {
                    self.saw_first_token = true;
                    events.push(SseEvent::FirstToken);
                }

                if rest.windows(7).any(|w| w == b"\"usage\"") {
                    events.push(SseEvent::Usage);
                }

                // 逐字保真的前提下额外留一份负载副本，供协议转换消费。
                self.frame_data
                    .push(String::from_utf8_lossy(rest).into_owned());
            }
        }

        (chunk.clone(), events)
    }

    /// 取出目前已凑齐的完整帧负载（`data:` 行的内容，多行以 `\n` 连接）。
    ///
    /// 用于跨格式转换：调用方拿到的是**完整帧**，不必自己处理跨 chunk 的半行——
    /// 这正是旧 `ClaudeCodec::adapt_response` 用「chunk 内有无 `data:` 行」猜
    /// 边界、把半行当 JSON 解析失败后静默丢弃的病根。
    ///
    /// 取走后队列清空；未遇到空行分隔符的尾部数据留在内部，等下一块补齐。
    pub fn take_data_frames(&mut self) -> Vec<String> {
        std::mem::take(&mut self.ready_frames)
    }

    /// 取出尾部未终结的帧（流已结束，不会再有补齐的机会）。
    ///
    /// 上游若最后一帧没跟空行就断开（SSE 规范不要求结尾必须有空行），该帧会一直
    /// 留在 `frame_data` 里；不 flush 就会丢掉最后一帧——对流式转换而言可能是整个
    /// 响应的收尾帧（`message_delta` / `finish_reason`）。
    pub fn flush_pending_frame(&mut self) -> Vec<String> {
        if self.frame_data.is_empty() {
            return Vec::new();
        }
        let pending = self.frame_data.join("\n");
        self.frame_data.clear();
        vec![pending]
    }

    /// 上游是否已显式发过 `[DONE]`。
    ///
    /// 跨格式转换用：目标格式编码器据此决定 `finish()` 是否还要补终止帧
    /// （上游已发过就不能再补，否则客户端收到两次流终止）。
    pub fn saw_done(&self) -> bool {
        self.saw_done
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
