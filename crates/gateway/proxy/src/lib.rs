//! `gateway-proxy` —— 出口代理池（hyper 客户端 + SOCKS5 / HTTP 代理 + SSRF 防护）
//!
//! 提供**唯一的网络出口能力**给 `gateway-forward` 使用。
//!
//! ## 文件分工
//!
//! - [`pool`] —— `ProxyPool`：代理节点池
//! - [`node`] —— `ProxyNode`：解析后的代理节点
//! - [`dialer`] —— `Dialer` trait + 多种实现
//! - [`ssrf`] —— SSRF 防护
//! - [`stage`] —— `ProxyStage`：接入 pipeline

pub mod pool;
pub mod node;
pub mod dialer;
pub mod ssrf;
pub mod stage;

pub use pool::ProxyPool;
pub use node::{ProxyNode, ProxyScheme, BasicAuth};
pub use dialer::Dialer;
pub use ssrf::validate_url;
pub use stage::ProxyStage;
