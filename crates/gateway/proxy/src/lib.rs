//! `gateway-proxy` —— 代理节点解析 + 按 channel 选节点 + SSRF 校验
//!
//! ponytail: 本 crate 不拨号。`reqwest`（workspace 已启用 `socks` feature）原生
//! 支持 HTTP CONNECT 与 SOCKS5 握手，所以出口代理只需在构造
//! `forward::ReqwestEgress` 的 `reqwest::Client` 时把 [`ProxyNode::to_reqwest_proxy`]
//! 的结果喂给 `ClientBuilder::proxy()`。自己实现 dialer 会绕过 reqwest 的连接池。
//!
//! ## 文件分工
//!
//! - [`node`] —— 代理节点：URL 解析与 `reqwest::Proxy` 映射
//! - [`pool`] —— `ProxyPool`：按 channel 索引 + priority 分层选节点
//! - [`ssrf`] —— SSRF 防护：IP 字面量与 DNS 解析结果双重校验
pub mod node;
pub mod pool;
pub mod ssrf;

pub use node::{BasicAuth, ProxyNode, ProxyScheme};
pub use pool::ProxyPool;
pub use ssrf::validate_url;
