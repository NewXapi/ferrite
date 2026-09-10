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
    AnytlsAdapter, HttpAdapter, Hy2Adapter, Hy2Options, ShadowsocksAdapter, SnellAdapter,
    SnellObfs, SnellVersion, Socks5Adapter, TransportChain, TrojanAdapter, VlessAdapter,
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
            let tls = node.vless.as_ref(); // trojan 复用 query 的 sni/insecure
            // TrojanAdapter 暂不支持 fingerprint（meow 实现限制），仅 vless+tls/reality 路径生效
            let sni = tls
                .and_then(|o| o.sni.clone())
                .unwrap_or_else(|| node.host.clone());
            let skip_verify = tls.is_some_and(|o| o.insecure);
            Some(Arc::new(TrojanAdapter::new(
                &name,
                &node.host,
                node.port,
                &auth.pass,
                &sni,
                skip_verify,
                false,
            )))
        }
        ProxyScheme::Vless => {
            let uuid = require_uuid(node)?;
            let flow = match node.vless.as_ref().and_then(|o| o.flow.as_deref()) {
                Some("xtls-rprx-vision") => Some(meow_proxy::VlessFlow::XtlsRprxVision),
                Some(other) => {
                    tracing::warn!(flow = other, "未知 VLESS flow，按非 Vision 处理");
                    None
                }
                None => None,
            };
            // 传输层：配置了 sni 或 pbk 才挂 TLS 层；pbk 存在即 REALITY 握手
            // 传输层：配置了 sni 或 pbk 才挂 TLS 层；pbk 存在即 REALITY 握手
            let vless = node.vless.as_ref();
            let need_tls = vless.is_some_and(|o| o.sni.is_some() || o.pbk.is_some());
            let mut chain = TransportChain::empty();
            if need_tls {
                let sni = vless
                    .and_then(|o| o.sni.clone())
                    .unwrap_or_else(|| node.host.clone());
                let mut cfg = meow_transport::tls::TlsConfig::new(sni.clone());
                if let Some(pbk) = vless.and_then(|o| o.pbk.clone()) {
                    match decode_reality(&pbk, vless.and_then(|o| o.sid.clone())) {
                        Ok(r) => cfg.reality = Some(r),
                        Err(e) => {
                            tracing::warn!(
                                host = %node.host,
                                port = node.port,
                                error = %e,
                                "REALITY 参数非法，节点回落直连"
                            );
                            return None;
                        }
                    }
                }
                if let Some(fp) = vless.and_then(|o| o.fingerprint.clone()) {
                    cfg.fingerprint = Some(fp);
                }
                match meow_transport::tls::TlsLayer::new(&cfg) {
                    Ok(layer) => chain.push(Box::new(layer)),
                    Err(e) => {
                        tracing::warn!(
                            host = %node.host,
                            port = node.port,
                            sni = %sni,
                            error = %e,
                            "TLS 层构造失败，节点回落直连"
                        );
                        return None;
                    }
                }
            }
            // Add WebSocket layer if ws_path is configured
            if let Some(ws_path) = vless.and_then(|o| o.ws_path.clone()) {
                // meow 要求 host_header 必填（ADR-0001：transport 不自行推断）；
                // 缺省回落 sni，再回落节点 host
                let ws_host = vless
                    .and_then(|o| o.ws_host.clone())
                    .or_else(|| vless.and_then(|o| o.sni.clone()))
                    .unwrap_or_else(|| node.host.clone());
                let ws_cfg = meow_transport::ws::WsConfig {
                    path: ws_path,
                    host_header: Some(ws_host),
                    ..Default::default()
                };
                match meow_transport::ws::WsLayer::new(ws_cfg) {
                    Ok(layer) => chain.push(Box::new(layer)),
                    Err(e) => {
                        tracing::warn!(
                            host = %node.host,
                            port = node.port,
                            error = %e,
                            "WebSocket 层构造失败，节点回落直连"
                        );
                        return None;
                    }
                }
            }
            Some(Arc::new(VlessAdapter::new(
                &name, &node.host, node.port, uuid, flow, false, chain,
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
            let vless = node.vless.as_ref();
            let mut chain = TransportChain::empty();
            // Add WebSocket layer if ws_path is configured
            if let Some(ws_path) = vless.and_then(|o| o.ws_path.clone()) {
                let ws_host = vless.and_then(|o| o.ws_host.clone()).unwrap_or_else(|| {
                    vless
                        .and_then(|o| o.sni.clone())
                        .unwrap_or_else(|| node.host.clone())
                });
                let mut ws_cfg = meow_transport::ws::WsConfig {
                    path: ws_path,
                    ..Default::default()
                };
                ws_cfg.host_header = Some(ws_host);
                match meow_transport::ws::WsLayer::new(ws_cfg) {
                    Ok(layer) => chain.push(Box::new(layer)),
                    Err(e) => {
                        tracing::warn!(
                            host = %node.host,
                            port = node.port,
                            error = %e,
                            "WebSocket 层构造失败，节点回落直连"
                        );
                        return None;
                    }
                }
            }
            Some(Arc::new(VmessAdapter::new(
                &name, &node.host, node.port, uuid, security, false, chain,
            )))
        }
        ProxyScheme::Hysteria2 => {
            let auth = require_auth(node)?;
            let opts = node.vless.as_ref();
            // Hy2 是 QUIC/UDP 承载：meow 在 dial_tcp 内自管 UDP socket，TCP 出口路径可用
            let hy2 = Hy2Options {
                name: name.clone(),
                server: node.host.clone(),
                port: node.port,
                password: auth.pass.clone(),
                sni: opts.and_then(|o| o.sni.clone()),
                skip_cert_verify: opts.is_some_and(|o| o.insecure),
                udp: false,
                up_bps: 0,
                down_bps: 0,
                obfs: None,
                obfs_password: None,
                ports: None,
                hop_interval: None,
                fingerprint: None,
                fast_open: false,
            };
            match Hy2Adapter::new(hy2) {
                Ok(a) => Some(Arc::new(a)),
                Err(e) => {
                    tracing::warn!(host = %node.host, port = node.port, error = %e, "Hysteria2 参数非法，节点回落直连");
                    None
                }
            }
        }
        ProxyScheme::AnyTls => {
            let auth = require_auth(node)?;
            let opts = node.vless.as_ref();
            let sni = opts.and_then(|o| o.sni.clone());
            match AnytlsAdapter::new(
                &name,
                &node.host,
                node.port,
                &auth.pass,
                sni.as_deref(),
                opts.is_some_and(|o| o.insecure),
                false,
            ) {
                Ok(a) => Some(Arc::new(a)),
                Err(e) => {
                    tracing::warn!(host = %node.host, port = node.port, error = %e, "AnyTLS 参数非法，节点回落直连");
                    None
                }
            }
        }
        ProxyScheme::Snell => {
            let auth = require_auth(node)?;
            let opts = node.vless.as_ref();
            let version = match opts.and_then(|o| o.version.as_deref()) {
                Some("v3") => SnellVersion::V3,
                Some("v4") => SnellVersion::V4,
                // 缺省 v5（当前主流），未知值 warn 后同样按 v5
                other => {
                    if let Some(v) = other {
                        tracing::warn!(version = v, "未知 Snell version，按 v5 处理");
                    }
                    SnellVersion::V5
                }
            };
            let obfs = match opts.and_then(|o| o.obfs.as_deref()) {
                Some("http") => SnellObfs::Http {
                    host: opts.and_then(|o| o.obfs_uri.clone()).unwrap_or_default(),
                },
                Some("tls") => SnellObfs::Tls {
                    server: opts.and_then(|o| o.obfs_uri.clone()).unwrap_or_default(),
                },
                _ => SnellObfs::None,
            };
            match SnellAdapter::new(
                &name, &node.host, node.port, &auth.pass, obfs, version, false, false,
            ) {
                Ok(a) => Some(Arc::new(a)),
                Err(e) => {
                    tracing::warn!(host = %node.host, port = node.port, error = %e, "Snell 参数非法，节点回落直连");
                    None
                }
            }
        }
    }
}
fn require_auth(node: &ProxyNode) -> Option<&crate::node::BasicAuth> {
    // SS 的 user= 密码方法、pass= 密码，两个都必须有；Trojan/VLESS/VMess 的
    // 密码/UUID 都在 user 字段（trojan://pass@host、vless://uuid@host），
    // pass 允许为空——之前把 Trojan 也算成双字段协议，合法节点被拒。
    let require_both = matches!(node.scheme, ProxyScheme::Shadowsocks);
    match &node.auth {
        Some(a) if !require_both || (!a.user.is_empty() && !a.pass.is_empty()) => Some(a),
        Some(a) => {
            // 只填了一半（比如 SS 只给了 cipher 没给密码）：按缺失处理，
            // 否则会拖到 adapter 构造深处才报出难懂的错。
            tracing::warn!(
                host = %node.host,
                port = node.port,
                scheme = ?node.scheme,
                has_user = !a.user.is_empty(),
                has_pass = !a.pass.is_empty(),
                "节点认证信息不完整，回落直连"
            );
            None
        }
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

/// 把节点配置里的 REALITY 参数解码成 meow 的 [`RealityConfig`]。
///
/// - `pbk`：X25519 公钥。**xray 分享链接里是 43 字符 base64url（无 padding）**，
///   也兼容 64 字符 hex；两者解码后都必须是 32 字节，否则报错
/// - `sid`：short id，hex 0-16 字符（0-8 字节），解码后**前对齐**补零到
///   8 字节（与 xray 的 SNI 拼接约定一致）；缺省全 0
fn decode_reality(
    pbk: &str,
    sid: Option<String>,
) -> std::result::Result<meow_transport::tls::RealityConfig, String> {
    // 先试 hex（64 字符常规配置），失败再试 base64url 43 字符（xray 分享链接）：
    // base64url 字母表与 hex 有交集，仅凭字符集无法区分，用「长度 + 解码成功」判定。
    let decode32 = |s: &str, what: &str| -> Result<[u8; 32], String> {
        let bytes = if s.len() == 64 {
            hex::decode(s).map_err(|e| format!("{what} 含非法 hex 字符: {e}"))?
        } else if s.len() == 43 {
            use base64::Engine as _;
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(s)
                .map_err(|e| format!("{what} base64url 解码失败: {e}"))?
        } else {
            return Err(format!(
                "{what} 长度非法：应为 64 hex 或 43 base64url 字符，实际 {}",
                s.len()
            ));
        };
        bytes
            .try_into()
            .map_err(|b: Vec<u8>| format!("{what} 必须 32 字节，实际 {} 字节", b.len()))
    };
    let public_key = decode32(pbk, "pbk")?;
    let mut short_id = [0u8; 8];
    if let Some(sid) = sid.filter(|s| !s.is_empty()) {
        let bytes = hex::decode(sid.as_str()).map_err(|e| format!("sid 含非法 hex 字符: {e}"))?;
        if bytes.len() > 8 {
            return Err(format!(
                "sid 最多 8 字节（16 个 hex 字符），实际 {} 字节",
                bytes.len()
            ));
        }
        short_id[..bytes.len()].copy_from_slice(&bytes);
    }
    Ok(meow_transport::tls::RealityConfig {
        public_key,
        short_id,
        support_x25519_mlkem768: false,
    })
}
