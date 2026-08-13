//! `enum Format`：MVP 仅 OpenAI。后续按 docs/09-roadmap.md §1.1 扩展。

/// 入站/出站的报文格式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    OpenAI,
}
