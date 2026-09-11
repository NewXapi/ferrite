//! `adapter` —— [`ProxyNode`] 到 meow 出口适配器的映射层。
//!
//! 协议装配交给 [`meow_config::proxy_parser::parse_proxy`]：它吃一份 clash 风格
//! 的配置 map，自己组装协议（ss/trojan/vless/vmess/hysteria2/anytls/snell）、
//! 传输链（ws/grpc/h2/httpupgrade）与 TLS（含 REALITY 与 uTLS 指纹）。
//!
//! 本模块只做**字段映射**：`ProxyNode` → clash map。协议与传输的组合爆炸由
//! meow-config 吸收，这里不写任何 `TransportChain::push` 或 `TlsConfig` 拼装
//! ——那正是上一版 400 行手写分支的来源。
//!
//! scheme → auth 字段语义（与 `ss://` 解析约定一致，见 PR #81）：
//! - Shadowsocks：`auth.user` = cipher 方法名，`auth.pass` = 密码
//! - VLESS / VMess：`auth.user` = UUID，`auth.pass` = 备注（VMess 的 security）
//! - Trojan / Hysteria2 / AnyTLS / Snell：密码在 `auth.user`（`trojan://pass@host`）

use std::collections::HashMap;
use std::sync::Arc;

use meow_common::{ConnType, DnsMode, Metadata, Network, ProxyAdapter};
use serde_yaml::Value as Yaml;
use smol_str::SmolStr;

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
/// - `Direct` / `Http` / `Socks5` 返回 `None`：前两者由 reqwest 直接处理
///   （`ProxyNode::to_reqwest_proxy`），调用方走 `ProxyClient::Reqwest`
/// - 配置非法（缺认证、UUID 格式错、cipher 不识别、REALITY 参数错…）：
///   `tracing::warn!` 后返回 `None`，调用方回落直连——节点配置错误不该 panic 网关。
///   具体判定由 meow-config 完成，错误信息原样透传到日志
pub fn adapter_for(node: &ProxyNode) -> Option<Arc<dyn ProxyAdapter>> {
    // http/socks5 由 reqwest 的 socks feature 处理，不进 meow；direct 无适配器
    let proxy_type = match node.scheme {
        ProxyScheme::Direct | ProxyScheme::Http | ProxyScheme::Socks5 => return None,
        ProxyScheme::Shadowsocks => "ss",
        ProxyScheme::Trojan => "trojan",
        ProxyScheme::Vless => "vless",
        ProxyScheme::Vmess => "vmess",
        ProxyScheme::Hysteria2 => "hysteria2",
        ProxyScheme::AnyTls => "anytls",
        ProxyScheme::Snell => "snell",
    };
    let config = clash_config(node, proxy_type);
    match meow_config::proxy_parser::parse_proxy(&config) {
        Ok(proxy) => Some(proxy),
        Err(e) => {
            tracing::warn!(
                host = %node.host,
                port = node.port,
                scheme = ?node.scheme,
                error = %e,
                "节点配置非法，回落直连"
            );
            None
        }
    }
}

/// `ProxyNode` → clash 风格配置 map（`parse_proxy` 的输入）。
///
/// 键名遵循 clash/mihomo 约定，与 meow-config 的解析一一对应；缺省项一律不写入，
/// 让 meow-config 用它自己的缺省（例如 `udp: false`、`skip-cert-verify: false`）。
fn clash_config(node: &ProxyNode, proxy_type: &str) -> HashMap<String, Yaml> {
    let mut c = HashMap::new();
    let mut put = |k: &str, v: Yaml| {
        c.insert(k.to_string(), v);
    };
    put("name", str_val(&format!("node-{}-{proxy_type}", node.id)));
    put("type", str_val(proxy_type));
    put("server", str_val(&node.host));
    put("port", Yaml::Number(node.port.into()));

    // 认证：parse_url 把 URL userinfo 拆成 user/pass——`scheme://pass@host` 形态的
    // 协议（trojan/hysteria2/anytls/snell/vless/vmess）密码或 UUID 都落在 user，
    // pass 为空；只有 `ss://cipher:pass@` 两个字段都有值。
    // clash 侧键名：ss→cipher+password，vless/vmess→uuid，snell→psk，其余→password。
    if let Some(auth) = &node.auth {
        match proxy_type {
            "ss" => {
                put("cipher", str_val(&auth.user));
                put("password", str_val(&auth.pass));
            }
            "vless" | "vmess" => put("uuid", str_val(&auth.user)),
            "snell" => put("psk", str_val(&auth.user)),
            _ => put("password", str_val(&auth.user)),
        }
        // VMess 的加密方式复用 auth.pass（旧约定），空值让 meow 取缺省 auto
        if proxy_type == "vmess" && !auth.pass.is_empty() {
            put("cipher", str_val(&auth.pass));
        }
    }

    let Some(o) = node.opts.as_ref() else {
        return c;
    };

    // TLS 与 REALITY
    if let Some(sni) = o.sni.as_deref().filter(|s| !s.is_empty()) {
        put("tls", Yaml::Bool(true));
        put("sni", str_val(sni));
        put("servername", str_val(sni));
    }
    if o.insecure {
        put("skip-cert-verify", Yaml::Bool(true));
    }
    if let Some(pbk) = o.pbk.as_deref().filter(|s| !s.is_empty()) {
        // REALITY 必须在 TLS 之上；分享链接常只给 pbk 不给 security=tls
        put("tls", Yaml::Bool(true));
        let mut reality = serde_yaml::Mapping::new();
        reality.insert(str_val("public-key"), str_val(pbk));
        if let Some(sid) = o.sid.as_deref().filter(|s| !s.is_empty()) {
            reality.insert(str_val("short-id"), str_val(sid));
        }
        put("reality-opts", Yaml::Mapping(reality));
        // meow-config 要求 REALITY 必须带 client-fingerprint，缺省用 chrome
        put(
            "client-fingerprint",
            str_val(o.fingerprint.as_deref().unwrap_or("chrome")),
        );
    } else if let Some(fp) = o.fingerprint.as_deref().filter(|s| !s.is_empty()) {
        put("client-fingerprint", str_val(fp));
    }

    // VLESS XTLS flow
    if let Some(flow) = o.flow.as_deref().filter(|s| !s.is_empty()) {
        put("flow", str_val(flow));
    }

    // WebSocket 传输：path 存在即视为 ws（与分享链接惯例一致）
    if let Some(path) = o.ws_path.as_deref().filter(|s| !s.is_empty()) {
        put("network", str_val("ws"));
        let mut ws = serde_yaml::Mapping::new();
        ws.insert(str_val("path"), str_val(path));
        let host = o
            .ws_host
            .as_deref()
            .or(o.sni.as_deref())
            .unwrap_or(&node.host);
        let mut headers = serde_yaml::Mapping::new();
        headers.insert(str_val("Host"), str_val(host));
        ws.insert(str_val("headers"), Yaml::Mapping(headers));
        put("ws-opts", Yaml::Mapping(ws));
    }

    // Snell 版本与混淆
    if let Some(v) = o.version.as_deref().filter(|s| !s.is_empty()) {
        // clash 用数字版本（3/4/5），链接里常写 v5
        let digits: String = v.chars().filter(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u64>() {
            put("version", Yaml::Number(n.into()));
        }
    }
    if let Some(obfs) = o.obfs.as_deref().filter(|s| !s.is_empty()) {
        let mut m = serde_yaml::Mapping::new();
        m.insert(str_val("mode"), str_val(obfs));
        if let Some(uri) = o.obfs_uri.as_deref().filter(|s| !s.is_empty()) {
            m.insert(str_val("host"), str_val(uri));
        }
        put("obfs-opts", Yaml::Mapping(m));
    }
    c
}

fn str_val(s: &str) -> Yaml {
    Yaml::String(s.to_string())
}
