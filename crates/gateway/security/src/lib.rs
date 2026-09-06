//! `gateway-security` —— 过滤词扫描实现 (WordFilter + StreamFilter)
//!
//! 本 crate 提供**纯逻辑过滤**实现，不包含 pipeline stage。
//!
//! ## 核心类型
//!
//! - `WordFilter`：一次性文本过滤器，使用 AhoCorasick 实现大小写不敏感扫描。零分配热路径（Cow）。
//! - `StreamFilter`：跨 chunk 流式过滤器，用于 SSE 流，维护 max_pattern_len-1 字节尾巴以捕获跨 chunk 词。
//!
//! ## 配置
//!
//! 通过外部 crate（如 `apps.gateways`）从 `[security]` 配置段反序列化 `FilterConfig`，其中包括词列表、替换文本以及 filter_request/filter_response 开关。
//!
//! ## 接线位置 (后续 PR 独立实现)
//!
//! - **请求体解析**：`crate::gateway::request::body::process` 侧。
//! - **响应流式**：`crate::gateway::stream::pipe_chunk` 侧（SseScanner/StreamScanner 内）。
//!
//! 当前 two-level proxy 架构中，stream.rs 已内置扫描逻辑，但 stage 的存在阻碍了接线。
//! 完成此 crate 后即可移除 stage 并完成接线。
//!
//! `[security]` 配置示例 (由外部 apps.gateways 读取)：
//! ```toml
//! [security]
//! words = ["sensitive", "classified"]
//! replacement = "***"
//! filter_request = true
//! filter_response = true
//! ```

pub mod scan;
pub mod wordlist;

pub use scan::{StreamFilter, WordFilter};
pub use wordlist::FilterConfig;
