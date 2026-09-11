//! `gateway-proxy` —— 代理节点解析 + 按 channel 选节点 + 出口 Client 租约
//!
//! HTTP CONNECT / SOCKS5 握手由 `reqwest`（workspace 已启用 `socks` feature）完成。
//! [`ProxyManager::acquire`] 按 `channel_key` 选出节点，返回已注入
//! [`ProxyNode::to_reqwest_proxy`] 的 `reqwest::Client`。
//!
//! reqwest 覆盖不了的协议（vless/vmess/ss/trojan）由 [`adapter`] 模块映射成 meow
//! `ProxyAdapter`，协议握手在 `dial_tcp` 内完成；`ProxyManager` 为它们产出 `ProxyClient::Adapter`。
//!
//! ## 文件分工
//!
//! - [`sharelink`] —— 分享链接方言解析器（vmess base64/ss legacy/SIP002）
//! - [`probe`] —— 主动探测（TCP 拨号判活，复用 meow ProxyHealth）
//! - [`node`] —— 代理节点：URL 解析与 `reqwest::Proxy` 映射（含 Vless/Vmess/Shadowsocks/Trojan 标记）
//! - [`adapter`] —— 节点 → meow `ProxyAdapter` 映射（协议握手在 dial_tcp 内完成）
//! - [`pool`] —— `ProxyPool`：按 channel_key 索引 + priority 分层选节点
//! - [`ssrf`] —— SSRF 防护：IP 字面量与 DNS 解析结果双重校验
//! - [`manager`] —— 租约 / 健康冷却 / per-node Client 缓存（跳过未支持 scheme）

pub mod manager;
pub mod node;
pub mod pool;
pub mod probe;
pub mod sharelink;
pub mod ssrf;

pub use probe::{ProbeResult, probe_node};
pub use sharelink::{ShareLinkBatch, parse_share_link, parse_share_links};

pub use manager::{Lease, ProxyManager};
pub use node::{BasicAuth, ProxyNode, ProxyScheme};
pub use pool::{ProxyPool, ProxySnapshot};
pub use ssrf::validate_url;

pub mod adapter;

pub use adapter::{adapter_for, tcp_metadata};
pub use meow_common::{ProxyAdapter, ProxyConn};
