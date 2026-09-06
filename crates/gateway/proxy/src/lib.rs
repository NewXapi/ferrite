//! `gateway-proxy` —— 代理节点解析 + 按 channel 选节点 + 出口 Client 租约
//!
//! HTTP CONNECT / SOCKS5 握手由 `reqwest`（workspace 已启用 `socks` feature）完成。
//! [`ProxyManager::acquire`] 按 `channel_key` 选出节点，返回已注入
//! [`ProxyNode::to_reqwest_proxy`] 的 `reqwest::Client`。自己实现 dialer 会绕过
//! reqwest 的连接池。vless/vmess/ss/trojan 握手不在本 crate。
//!
//! ## 文件分工
//!
//! - [`node`] —— 代理节点：URL 解析与 `reqwest::Proxy` 映射
//! - [`pool`] —— `ProxyPool`：按 channel_key 索引 + priority 分层选节点
//! - [`ssrf`] —— SSRF 防护：IP 字面量与 DNS 解析结果双重校验
//! - [`manager`] —— 租约 / 健康冷却 / per-node Client 缓存
pub mod manager;
pub mod node;
pub mod pool;
pub mod ssrf;

pub use manager::{Lease, ProxyManager};
pub use node::{BasicAuth, ProxyNode, ProxyScheme};
pub use pool::ProxyPool;
pub use ssrf::validate_url;
