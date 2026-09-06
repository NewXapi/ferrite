//! `gateway-security` —— 过滤词扫描
//!
//! 纯过滤逻辑，不含 pipeline stage：配置里给一份过滤词表，对模型请求上下文
//! （输入）与响应（输出，含流式）扫描替换。
//!
//! ## 核心类型
//!
//! - [`WordFilter`]：一次性文本过滤，`aho-corasick` 自动机，无命中零分配（`Cow`）。
//! - [`StreamFilter`]：跨 chunk 流式过滤，hold 尾巴捕获跨 chunk 边界的词。
//!
//! ## 接线（后续 PR）
//!
//! 接线属 `forward` 域，本 crate 只交付逻辑：
//! - 请求侧：`forward::stage::ForwardStage::handle` 读 body 后过 [`WordFilter::filter`]。
//! - 响应侧：`forward::stream::pipe_chunk` 逐 chunk 过 [`StreamFilter::push`]，
//!   流结束调 [`StreamFilter::flush`]。
//!
//! ## 配置
//!
//! [`FilterConfig`] 由 `apps/gateway` 的 `GatewayConfig` 从 `[security]` 段读入：
//!
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
