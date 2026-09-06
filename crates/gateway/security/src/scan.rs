//! 过滤词扫描 —— 一次性文本（[`WordFilter`]）+ 跨 chunk 流式（[`StreamFilter`]）
//!
//! 接线位置与配置格式见 crate 根文档。

use std::sync::Arc;

use aho_corasick::AhoCorasick;

/// 一次性文本过滤器：ASCII 大小写不敏感的词表扫描 + 替换。
///
/// 空词表构建成功但永不命中；无命中时 [`filter`](Self::filter) 返回
/// `Cow::Borrowed`（零分配热路径）。
pub struct WordFilter {
    ac: AhoCorasick,
    /// `replace_all` 要求每个模式一个替换串，这里全部指向同一个 `replacement`。
    replacements: Vec<String>,
    max_pattern_len: usize,
}

impl WordFilter {
    /// 从 [`FilterConfig`](crate::wordlist::FilterConfig) 构建。
    ///
    /// `ascii_case_insensitive(true)`：过滤词场景默认忽略 ASCII 大小写。
    /// CJK 无大小写概念，不受影响。
    ///
    /// 空词条被丢弃：`aho-corasick` 对空 pattern 在每个字符间隙都命中，
    /// 一个手误的 `words = [""]` 会把 `"a bad"` 替换成 `"XaX XbXaXdX"`，
    /// 并让 `is_match` 恒为 true、零分配快路径失效。配置是信任边界，必须挡。
    ///
    /// # Panics
    /// 词表使自动机超出 `aho-corasick` 的状态数上限时 panic。过滤词是安全功能，
    /// 启动期崩溃优于运行期静默不过滤——返回 `Result` 会让调用方有机会忽略错误
    /// 继续跑一个不过滤的网关。
    pub fn new(config: &crate::wordlist::FilterConfig) -> Self {
        let words: Vec<&str> = config
            .words
            .iter()
            .map(String::as_str)
            .filter(|w| !w.is_empty())
            .collect();
        let ac = AhoCorasick::builder()
            .ascii_case_insensitive(true)
            .build(&words)
            .expect("过滤词表构建 AhoCorasick 失败");
        Self {
            replacements: vec![config.replacement.clone(); words.len()],
            max_pattern_len: words.iter().map(|w| w.len()).max().unwrap_or(0),
            ac,
        }
    }

    /// 过滤文本（ASCII 大小写不敏感），命中处替换为配置的 `replacement`。
    ///
    /// 无命中或词表为空时返回 `Cow::Borrowed` —— 绝大多数请求不命中，这是热路径，
    /// 必须不分配。
    pub fn filter<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if self.replacements.is_empty() || !self.ac.is_match(text) {
            return std::borrow::Cow::Borrowed(text);
        }
        std::borrow::Cow::Owned(self.ac.replace_all(text, &self.replacements))
    }

    /// 流式过滤的 emit 边界：`text` 中可以安全输出的字节数。
    ///
    /// 规则：末尾 `max_pattern_len - 1` 字节可能是跨 chunk 词的前半，必须 hold；
    /// 但已经完整命中的部分不必再等，所以边界取"最后一个命中的 end"与
    /// "`len - hold`"的较大者。边界永不落在某个命中内部。
    ///
    /// 返回值已回退到 UTF-8 字符边界。
    fn emit_boundary(&self, text: &str) -> usize {
        let hold = self.max_pattern_len.saturating_sub(1);
        let by_len = text.len().saturating_sub(hold);
        let by_match = self.ac.find_iter(text).last().map_or(0, |m| m.end());
        let mut at = by_len.max(by_match);
        // 切片必须落在字符边界：中文一个字 3 字节，切中间会 panic。
        // by_match 来自命中 end，天然在边界上；只有 by_len 可能落在字符内部。
        while at > 0 && !text.is_char_boundary(at) {
            at -= 1;
        }
        at
    }

    /// 词表是否为空（调用方可整条跳过过滤）。
    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }

    /// 检查是否命中，用于只需判断的调用方。
    pub fn has_match(&self, text: &str) -> bool {
        self.ac.is_match(text)
    }
}

/// 跨 chunk 流式过滤器：处理被网络层切成任意长度的流式文本（如 SSE）。
///
/// 过滤词可能横跨 chunk 边界（`"my sec"` + `"ret here"`），单看任一 chunk 都不命中。
/// 因此 `push` 只输出**已确定不会再参与匹配**的前缀，其余留在 `pending` 里等下一个
/// chunk 拼接。切分在替换前的原文坐标系上做，只对要发出的前缀替换 —— 反过来
/// （先替换再按原文长度切）会因替换改变长度而切错位甚至越界 panic。
///
/// ponytail: hold 住尾巴会让这点内容延后一个 chunk 才发出；词表里有超长词时首字
/// 延迟随之变大。需要更精细就用 aho-corasick 的 `Automaton` 低阶 API 拿状态机
/// 位置，只 hold 真正处于匹配中途的字节。
pub struct StreamFilter {
    filter: Arc<WordFilter>,
    /// 尚未确认可输出的**原文**（未替换）。
    pending: String,
}

impl StreamFilter {
    /// 用共享的 [`WordFilter`] 创建流式过滤器。
    pub fn new(filter: Arc<WordFilter>) -> Self {
        Self {
            filter,
            pending: String::new(),
        }
    }

    /// 推入一个 chunk，返回已确认安全可输出的（过滤后）文本。
    ///
    /// 空词表时原样透传不缓冲，否则会凭空增加首字延迟。
    ///
    /// 契约：所有 `push` 返回值按序拼接再接上 [`flush`](Self::flush)，等于对完整
    /// 原文调一次 [`WordFilter::filter`]。单次返回多少字节是内部 hold 策略。
    pub fn push(&mut self, chunk: &str) -> String {
        if self.filter.is_empty() {
            return chunk.to_string();
        }

        self.pending.push_str(chunk);

        let at = self.filter.emit_boundary(&self.pending);
        if at == 0 {
            return String::new();
        }

        let tail = self.pending.split_off(at);
        let emit = std::mem::replace(&mut self.pending, tail);
        self.filter.filter(&emit).into_owned()
    }

    /// 流结束时输出 `pending` 里剩下的全部内容（过滤后）。
    ///
    /// 调用后状态复位，同一实例可用于下一条流。
    pub fn flush(&mut self) -> String {
        let out = self.filter.filter(&self.pending).into_owned();
        self.pending.clear();
        out
    }
}
