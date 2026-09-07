//! `gateway-proxy` —— 代理节点解析 + 按 channel 选节点 + 出口 Client 租约
//!
//! HTTP CONNECT / SOCKS5 握手由 `reqwest`（workspace 已启用 `socks` feature）完成。
//! [`ProxyManager::acquire`] 按 `channel_key` 选出节点，返回已注入
//! [`ProxyNode::to_reqwest_proxy`] 的 `reqwest::Client`。
//!
//! reqwest 覆盖不了的协议（vless/vmess/ss/trojan）由 [`proto`] 模块提供
//! 客户端协议链（shoes MIT 移植）；`ProxyManager` 在协议实现落地前跳过这类节点。
//!
//! ## 文件分工
//!
//! - [`node`] —— 代理节点：URL 解析与 `reqwest::Proxy` 映射（含 Vless/Vmess/Shadowsocks/Trojan 标记）
//! - [`proto`] —— 协议基础设施：AsyncStream、Address、StreamReader、ProxyConnector（shoes 移植）
//! - [`pool`] —— `ProxyPool`：按 channel_key 索引 + priority 分层选节点
//! - [`ssrf`] —— SSRF 防护：IP 字面量与 DNS 解析结果双重校验
//! - [`manager`] —— 租约 / 健康冷却 / per-node Client 缓存（跳过未支持 scheme）

pub mod manager;
pub mod node;
pub mod pool;
pub mod proto;
pub mod ssrf;

pub use manager::{Lease, ProxyManager};
pub use node::{BasicAuth, ProxyNode, ProxyScheme};
pub use pool::ProxyPool;
pub use ssrf::validate_url;

pub use proto::{
    Address, AsyncStream, NetLocation, ProxyConnector, ResolvedLocation, StreamReader,
};
