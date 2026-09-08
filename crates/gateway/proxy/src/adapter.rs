//! `adapter` —— [`ProxyNode`] 到 meow 出口适配器的映射层。
//!
//! 出口协议实现交给 [`meow_proxy`](https://crates.io/crates/meow-proxy)
//! （MIT）：每个节点由 [`adapter_for`] 构造成 `Arc<dyn ProxyAdapter>`，
//! `dial_tcp(&Metadata)` 返回已完成协议握手的双向流。本模块只做纯映射，
//! 不拨号、不做健康反馈（那是 `manager` 的事）。
//!
//! scheme → auth 字段语义（与旧 `ss://` 解析约定一致，见 PR #81）：
//! - Shadowsocks：`auth.user` = cipher 方法名，`auth.pass` = 密码
//! - VLESS / VMess：`auth.user` = UUID，`auth.pass` = 备注（Vmess 的 security）

use std::sync::Arc;

use meow_common::{ConnType, DnsMode, Metadata, Network, ProxyAdapter};
use meow_proxy::vmess::header::Security as VmessSecurity;
use meow_proxy::{
    HttpAdapter, ShadowsocksAdapter, Socks5Adapter, TransportChain, TrojanAdapter, VlessAdapter,
    VmessAdapter,
};
use smol_str::SmolStr;
use uuid::Uuid;

use crate::node::{ProxyNode, ProxyScheme};

/// 构造指向 `host:port` 的 TCP 出口 [`Metadata`]。
///
/// meow 的 `Metadata` 没有实现 `Default`，这里按出站拨号所需的最小集填充：
/// 目的地信息（`host` / `dst_port`）必填，其余字段是入站侧才会用到的
/// 来源统计（src_geo_ip / process / in_user 等），出口方向一律空值。
pub fn tcp_metadata(host: &str, port: u16) -> Metadata {
    Metadata {
        network: Network::Tcp,
        conn_type: ConnType::Http,
        src_ip: None,
        dst_ip: None,
        src_port: 0,
        dst_port: port,
        host: SmolStr::from(host),
        dns_mode: DnsMode::Normal,
        process: SmolStr::default(),
        process_path: SmolStr::default(),
        uid: None,
        dscp: None,
        src_geo_ip: Vec::new(),
        dst_geo_ip: Vec::new(),
        sniff_host: SmolStr::default(),
        in_name: SmolStr::default(),
        in_port: 0,
        in_user: None,
        special_proxy: SmolStr::default(),
    }
}

/// 为节点构造 meow 出口适配器。
///
/// # 返回
/// - `Direct` 永远返回 `None`（调用方回落 reqwest 直连）
/// - 认证缺失 / UUID 非法 / cipher 不识别：`tracing::warn!` 后返回 `None`，
///   调用方回落直连——节点配置错误不应该 panic 网关
///
/// # 参数语义
/// 见模块注释的 scheme → auth 约定。`tls` / `udp` / `skip_verify` 均取保守
/// 缺省（关）：需要时随节点配置扩展，先把 TCP 出口打通。
pub fn adapter_for(node: &ProxyNode) -> Option<Arc<dyn ProxyAdapter>> {
    let name = format!("node-{}-{:?}", node.id, node.scheme).to_lowercase();
    match node.scheme {
        ProxyScheme::Direct => None,
        ProxyScheme::Http => {
            let auth = node.auth.as_ref().map(|a| (a.user.clone(), a.pass.clone()));
            Some(Arc::new(HttpAdapter::new(
                &name,
                &node.host,
                node.port,
                auth,
                false,
                false,
                Vec::new(),
            )))
        }
        ProxyScheme::Socks5 => {
            let auth = node.auth.as_ref().map(|a| (a.user.clone(), a.pass.clone()));
            Some(Arc::new(Socks5Adapter::new(
                &name, &node.host, node.port, auth, false, false,
            )))
        }
        ProxyScheme::Shadowsocks => {
            let auth = require_auth(node)?;
            // auth.user = cipher 方法名，auth.pass = 密码（PR #81 约定）
            match ShadowsocksAdapter::new(
                &name, &node.host, node.port, &auth.pass, &auth.user, false, None, None,
            ) {
                Ok(a) => Some(Arc::new(a)),
                Err(e) => {
                    tracing::warn!(
                        host = %node.host,
                        port = node.port,
                        cipher = %auth.user,
                        error = %e,
                        "SS cipher 不识别，节点回落直连"
                    );
                    None
                }
            }
        }
        ProxyScheme::Trojan => {
            let auth = require_auth(node)?;
            Some(Arc::new(TrojanAdapter::new(
                &name, &node.host, node.port, &auth.pass,
                // sni 缺省 = 服务器地址本身（rustls 会用 server name 校验）
                &node.host, false, false,
            )))
        }
        ProxyScheme::Vless => {
            let uuid = require_uuid(node)?;
            Some(Arc::new(VlessAdapter::new(
                &name,
                &node.host,
                node.port,
                uuid,
                None, // flow：XTLS-Vision 待节点配置扩展
                false,
                TransportChain::empty(),
            )))
        }
        ProxyScheme::Vmess => {
            let uuid = require_uuid(node)?;
            // meow 的 Security 没有 auto：未知取值按 AES-128-GCM 处理
            // （与旧 manager 的回落一致），不匹配的字符串 warn 提示配置拼写。
            let security = match node.auth.as_ref().map(|a| a.pass.as_str()) {
                Some("chacha20-poly1305") | Some("chacha20-ietf-poly1305") => {
                    VmessSecurity::ChaCha20Poly1305
                }
                Some("none") => VmessSecurity::None,
                Some(unknown @ ("auto" | "")) => {
                    tracing::debug!(security = unknown, "VMess security 缺省按 aes-128-gcm");
                    VmessSecurity::Aes128Gcm
                }
                Some(other) => {
                    tracing::warn!(security = other, "未知 VMess security，按 aes-128-gcm 处理");
                    VmessSecurity::Aes128Gcm
                }
                None => VmessSecurity::Aes128Gcm,
            };
            Some(Arc::new(VmessAdapter::new(
                &name,
                &node.host,
                node.port,
                uuid,
                security,
                false,
                TransportChain::empty(),
            )))
        }
    }
}

/// 取节点认证；缺失时 warn 并返回 `None`（`?` 提前退出整个映射）。
fn require_auth(node: &ProxyNode) -> Option<&crate::node::BasicAuth> {
    match &node.auth {
        Some(a) if !a.user.is_empty() || !a.pass.is_empty() => Some(a),
        _ => {
            tracing::warn!(
                host = %node.host,
                port = node.port,
                scheme = ?node.scheme,
                "节点缺少认证信息，回落直连"
            );
            None
        }
    }
}

/// 解析 `auth.user` 为 16 字节 UUID；非法时 warn 并返回 `None`。
///
/// 为什么不 panic：UUID 来自用户配置（`[[proxy_nodes]]`），配置错误走
/// 「回落直连 + 日志」，不能炸网关进程。
fn require_uuid(node: &ProxyNode) -> Option<[u8; 16]> {
    let auth = require_auth(node)?;
    match Uuid::parse_str(&auth.user) {
        Ok(u) => Some(*u.as_bytes()),
        Err(e) => {
            tracing::warn!(
                host = %node.host,
                port = node.port,
                uuid = %auth.user,
                error = %e,
                "节点 UUID 非法，回落直连"
            );
            None
        }
    }
}
