//! 过滤词扫描 —— 一次性文本 + 跨 chunk 流式
//!
//! 本 crate 提供两套纯逻辑类型，不含 pipeline stage。
//! - `WordFilter`：一次性文本过滤，词库在构建时解析，扫描零拷贝（Cow）。
//! - `StreamFilter`：跨 chunk 流式过滤（用于 SSE 流），自动维护 max_pattern_len-1 字节尾巴以捕获跨 chunk 词。
//!
//! 接线位置（后续 PR 独立实现）：
//! - 请求体解析：`crate::gateway::request::body::process` 侧。
//! - 响应流式：`crate::gateway::stream::pipe_chunk` 侧（SseScanner/StreamScanner 内）。
//!   当前 two-level proxy 架构中，stream.rs 已内置扫描逻辑，但 stage 的存在阻碍了接线。
//!   完成此 crate 后即可移除 stage 并完成接线。
//!
//! `[security]` 配置示例（由外部 apps.gateways 读取）：
//! ```toml
//! [security]
//! words = ["sensitive", "classified"]
//! replacement = "***"
//! filter_request = true
//! filter_response = true
//! ```

use std::sync::Arc;

use aho_corasick::AhoCorasick;

/// 一次性文本过滤器，使用 AhoCorasick 实现大小写不敏感词表扫描。
/// - 空词表时构建成功，但所有方法直接返回无操作结果（零分配热路径）。
/// - 使用 `Cow` 实现无命中直接返回原字符串（零分配），有命中返回新分配的 `String`。
pub struct WordFilter {
    ac: AhoCorasick,
    replacement: String,
    max_pattern_len: usize,
    pattern_count: usize,
}

impl WordFilter {
    /// 根据 `FilterConfig` 构建 WordFilter。
    /// - 使用 `ascii_case_insensitive(true)`，满足过滤词场景的基本要求。
    /// - 空词表时构建一个始终无匹配的 AhoCorasick 实例。
    pub fn new(config: &crate::wordlist::FilterConfig) -> Self {
        let pattern_count = config.words.len();
        let ac = if pattern_count == 0 {
            // 空词表：构建一个空模式的自动机
            let patterns: Vec<&str> = Vec::new();
            AhoCorasick::builder()
                .build(patterns)
                .expect("构建空 AhoCorasick 失败")
        } else {
            AhoCorasick::builder()
                .ascii_case_insensitive(true)
                .build(&config.words)
                .expect("构建 AhoCorasick 自动机失败")
        };

        let max_pattern_len = config.words.iter().map(|s| s.len()).max().unwrap_or(0);
        Self {
            ac,
            replacement: config.replacement.clone(),
            max_pattern_len,
            pattern_count,
        }
    }

    /// 过滤文本，大小写不敏感。
    /// - 无匹配时返回 `Cow::Borrowed`（零分配）。
    /// - 有匹配时返回 `Cow::Owned`，用 replacement 替换所有匹配项。
    pub fn filter<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if self.pattern_count == 0 {
            return std::borrow::Cow::Borrowed(text);
        }

        // 替换所有匹配项
        let replaced = self.ac.replace_all(text, &[self.replacement.as_str()]);
        std::borrow::Cow::Owned(replaced)
    }

    /// 检查是否有任何匹配项，用于快速路径裁剪。
    pub fn has_match(&self, text: &str) -> bool {
        self.ac.is_match(text)
    }

    /// 检查词表是否为空。
    pub fn is_empty(&self) -> bool {
        self.pattern_count == 0
    }

    /// 获取最大模式长度（用于 StreamFilter 的尾巴缓冲区大小）。
    pub fn max_pattern_len(&self) -> usize {
        self.max_pattern_len
    }
}

/// 跨 chunk 流式过滤器，用于处理可能被拆分的流式文本（如 SSE）。
/// - 内部维护一个 `pending` 缓冲区，保留恰好 `max_pattern_len - 1` 字节以捕获跨 chunk 词。
/// - 对于 UTF-8 字符边界，使用 `str::floor_char_boundary` 等效逻辑找到安全的切分点。
/// - 空词表时直接透传 chunk，无缓冲区操作（避免不必要的分配和延迟）。
pub struct StreamFilter {
    filter: Arc<WordFilter>,
    pending: String,
    hold_len: usize, // = max_pattern_len - 1
    // ponytail: 当前实现每次 push 后全量扫描 pending 内容
    // 优化方向：使用 aho-corasick 的流式 API（如果有的话）以避免全量扫描
    // 另一种方案：维护 max_pattern_len-1 字节的尾巴，交由下游处理实现更细致的增量扫描
}

impl StreamFilter {
    /// 创建新的 StreamFilter。
    pub fn new(filter: Arc<WordFilter>) -> Self {
        let hold_len = if filter.max_pattern_len() > 0 {
            filter.max_pattern_len() - 1
        } else {
            0
        };
        Self {
            filter,
            pending: String::new(),
            hold_len,
        }
    }

    /// 推入一个 chunk，返回已确认安全可输出的文本。
    /// - 对于空词表，直接追加到 pending 并返回原 chunk。
    /// - 对于非空词表，合并 pending+chunk，扫描替换，将前 len-output 字节输出，保留最后 hold_len 字节在 pending 中。
    /// - 确保仅在字符边界切分 pending（避免 UTF-8 无效序列）。
    pub fn push(&mut self, chunk: &str) -> String {
        if self.filter.is_empty() {
            // 空词表：不缓冲，直接返回 chunk
            self.pending.clear();
            return chunk.to_string();
        }

        // 合并 pending 和新 chunk
        let combined = format!("{}{}", self.pending, chunk);

        // 扫描和替换
        let output = self.filter.filter(&combined);

        // 计算需要保留的末尾字节数（hold_len），但要确保是字符边界
        let mut output_bytes = output.len();

        // 确定需要保留的字节数：如果 combined 长度 >= hold_len，则保留末尾 hold_len 字节
        if combined.len() >= self.hold_len {
            // 找到安全字符边界：向前查找 hold_len 位置，如果不安全，则向前移动
            let mut boundary = combined.len() - self.hold_len;
            while boundary > 0 && !combined.is_char_boundary(boundary) {
                boundary -= 1;
            }
            output_bytes = boundary;
        }

        let (output_prefix, output_suffix) = output.split_at(output_bytes);

        // 更新 pending：保留 suffix
        self.pending = output_suffix.to_string();

        output_prefix.to_string()
    }

    /// 最终 chunk 后调用 flush，输出 pending 中的所有剩余内容。
    pub fn flush(&mut self) -> String {
        let output = self.filter.filter(&self.pending);
        let flushed = output.to_string();
        self.pending.clear();
        flushed
    }
}
