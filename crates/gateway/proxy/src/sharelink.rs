//! `sharelink` —— 分享链接方言解析器
//!
//! `parse_url` 是标准 URL 解析器，吃不下两种主流方言：
//! - VMess v2rayN：`vmess://` + base64(JSON)，字段 `v/ps/add/port/id/aid/scy/net/type/host/path/tls/sni/fp/alpn`
//! - SS legacy：`ss://` + base64(`method:password`) `@host:port#备注`
//! - SS SIP002：`ss://` + base64(`method:password`) 或明文 `@host:port`
//!
//! 混进 `parse_url` 会让它从 URL 解析器退化成方言分派器（vmess 的 base64 载荷会被
//! 当成 host 解析），故独立成模块：按 scheme 派给专用解析器，其余 scheme 原样委托
//! `ProxyNode::parse_url`。
//!
//! 映射约定（与 `adapter.rs` 头部的 auth 语义一致，写错会导致装配后认证失败）：
//!
//! - `id` → `auth.user`（UUID）
//! - `scy` → `auth.pass`（VMess security / cipher）
//! - `add` → `host`，`port` → 端口（可能是数字或字符串两种形态）
//! - `net=ws` + `path` → `VlessOpts::ws_path`，`host` → `VlessOpts::ws_host`
//! - `sni` → `VlessOpts::sni`，`fp` → `VlessOpts::fingerprint`
//! - `net=grpc`/`h2`/`httpupgrade`：传输层由 meow-config 装配，本模块不拼 TransportChain
//! - `aid`（alterId）：VMess AEAD 之后已废弃，meow-config 不吃这个键，解析后丢弃

use crate::node::{ParseError, ProxyNode};
use serde::Deserialize;
use serde_json::Value as JsonValue;

/// v2rayN `vmess://` base64 载荷的 JSON 结构。
///
/// 字段名是 v2rayN 的历史缩写，用 serde rename 映射；`port`/`aid` 在不同客户端
/// 里可能是数字或字符串，故用 serde_json::Value 收后自行解析（rustdoc 要写清这点）。
#[derive(Debug, Clone, Deserialize)]
pub struct VmessShareLink {
    /// 协议版本，固定 "2"
    #[serde(rename = "v")]
    pub v: String,
    /// 备注/标签
    #[serde(rename = "ps")]
    pub ps: Option<String>,
    /// 地址 (host)
    #[serde(rename = "add")]
    pub add: String,
    /// 端口，可能是数字或字符串
    #[serde(rename = "port")]
    pub port: JsonValue,
    /// 用户 ID / UUID
    #[serde(rename = "id")]
    pub id: String,
    /// alter ID (aid)；到 auth.pass 映射
    #[serde(rename = "aid")]
    pub aid: JsonValue,
    /// 安全类型 (scy)；到 auth.pass 映射
    #[serde(rename = "scy")]
    pub scy: String,
    /// 网络 (net)；ws/grpc/h2/httpupgrade -> 传输层由 meow-config 装配
    #[serde(rename = "net")]
    pub net: String,
    /// 类型 (type)；ws/grpc/h2/httpupgrade 的传输层，由 meow-config 装配
    #[serde(rename = "type")]
    pub type_: String,
    /// host 头 (host)
    #[serde(rename = "host")]
    pub host: Option<String>,
    /// 路径 (path)；ws 层，映射到 VlessOpts::ws_path
    #[serde(rename = "path")]
    pub path: Option<String>,
    /// 是否 TLS (tls)；true = 需要 TLS
    #[serde(rename = "tls")]
    pub tls: Option<bool>,
    /// SNI (sni)
    #[serde(rename = "sni")]
    pub sni: Option<String>,
    /// uTLS 指纹 (fp)
    #[serde(rename = "fp")]
    pub fp: Option<String>,
    /// ALPN (alpn)
    #[serde(rename = "alpn")]
    pub alpn: Option<Vec<String>>,
}

/// 一批分享链接的解析结果：成功与失败分开归集。
///
/// 为什么不是 `Vec<Result<..>>`：调用方（订阅/批量导入）要分别统计与展示，
/// 拆开省一次 partition。failures 的第一元素是**掩码后**的原始行（含凭据不能进日志）。
pub struct ShareLinkBatch {
    pub nodes: Vec<ProxyNode>,
    pub failures: Vec<(String, String)>, // (掩码后原始行, 失败原因)
}

/// 解析单条分享链接 → `ProxyNode`。
/// vmess:// 走 base64-JSON 方言，ss:// 走 legacy/SIP002 双形态，其余 scheme 委托 `ProxyNode::parse_url`。
pub fn parse_share_link(link: &str) -> Result<ProxyNode, ParseError> {
    // 实现时按 scheme 分派到下面三个 helper；此处的引用同时固定了骨架期的调用图。
    let _ = (link, parse_vmess_dialect, parse_ss_dialect, mask_link);
    todo!("TODO(#111): 按 scheme 分派 vmess/ss 方言，其余委托 ProxyNode::parse_url")
}

/// 多行批量解析（一行一条，忽略空行与 `#` 注释行）。单行失败不影响整批。
pub fn parse_share_links(text: &str) -> ShareLinkBatch {
    let _ = text;
    todo!("TODO(#111): 实现批量分享链接解析")
}

/// vmess base64 载荷 → ProxyNode（内部；rustdoc 说明 aid/scy 到 VlessOpts/auth 的映射约定）
fn parse_vmess_dialect(link: &str) -> Result<ProxyNode, ParseError> {
    let _ = link;
    todo!("TODO(#111): 实现 vmess 方言解析")
}

/// ss legacy(`ss://base64(method:pass)@host:port`) 与 SIP002 双形态 → ProxyNode
fn parse_ss_dialect(link: &str) -> Result<ProxyNode, ParseError> {
    let _ = link;
    todo!("TODO(#111): 实现 ss 方言解析")
}

/// 掩码一行分享链接用于日志/错误回显：只留 scheme 与 host 骨架。
/// 与 `admin_proxy::mask_url` 同目的但那个是私有的，且这里要处理非法 URL 的原始行。
fn mask_link(link: &str) -> String {
    let _ = link;
    todo!("TODO(#111): 实现链接掩码")
}
