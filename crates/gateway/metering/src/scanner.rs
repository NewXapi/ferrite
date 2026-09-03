//! 流式 token 扫描器 — 挂在 forward::stream 管道里。
//!
//! 工作方式: 每收到一块 Bytes, `push()` 透传 (由 stream 模块负责), 本类型
//! 增量解析:
//! - 上游给 usage 字段 (OpenAI stream_options / Claude message_delta) → 直接采信;
//! - 没有 usage 的流 → finish 时按 estimate 模块估算 completion 侧。

/// 流结束时的最终计数。
#[derive(Debug, Clone, Copy, Default)]
pub struct TokenCounts {
    pub prompt: u64,
    pub completion: u64,
    pub cached: u64,
}

pub struct StreamScanner {
    /// 上游直接给的 usage (once, 保真)。
    _upstream_usage: Option<TokenCounts>,
    /// 无 usage 时的增量字符计数 (按 CJK/Latin 分类累计)。
    _char_classes: [u64; 6],
}

impl StreamScanner {
    pub fn new() -> Self {
        Self { _upstream_usage: None, _char_classes: [0; 6] }
    }

    /// 增量解析一块字节 (透传由 forward::stream 负责)。
    /// TODO(#332): SSE 事件回调对接 (Usage 事件 → 采信上游 usage)。
    pub fn push(&mut self, _chunk: &bytes::Bytes) {
        // TODO(#332)
    }

    /// 流结束, 产出最终计数。prompt 由请求体预扫得出 (estimate::prompt_tokens)。
    pub fn finish(self, prompt: u64) -> TokenCounts {
        let _ = prompt;
        TokenCounts::default() // TODO(#332): 无 usage 时估算 completion
    }
}
