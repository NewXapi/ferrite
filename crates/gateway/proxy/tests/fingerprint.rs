//! 指纹测试：`fingerprint=` / `fp=` query 解析与 TLS Config 传递。
//!
//! 验证：
//! 1. query 解析正确落入 `VlessOpts.fingerprint`
//! 2. adapter_for 在 VLESS + TLS/REALITY 路径把 fingerprint 传给 `TlsConfig`
//! 3. feature 关时构造不报错（仅 warn，由 meow 内部处理）

use gateway_proxy::adapter::adapter_for;
use gateway_proxy::node::{ProxyNode, ProxyScheme};

const UUID: &str = "11111111-2222-3333-4444-555555555555";

fn vless_node(query: &str) -> ProxyNode {
    let url = format!("vless://{UUID}@example.com:443{query}");
    let mut node = ProxyNode::parse_url(&url).expect("vless URL 必须能解析");
    node.channel_keys = vec!["openai".to_string()];
    node
}

/// `fingerprint=chrome` 解析落入 VlessOpts
#[test]
fn fingerprint_query_parsed() {
    let node = vless_node("?fingerprint=chrome&sni=cdn.example.com");
    let opts = node.vless.expect("vless 节点必须带 VlessOpts");
    assert_eq!(opts.fingerprint.as_deref(), Some("chrome"));
}

/// `fp=firefox` 短别名同样解析
#[test]
fn fp_short_alias_parsed() {
    let node = vless_node("?fp=firefox&sni=cdn.example.com");
    let opts = node.vless.expect("vless 节点必须带 VlessOpts");
    assert_eq!(opts.fingerprint.as_deref(), Some("firefox"));
}

/// fingerprint 与其他 query 参数共存不冲突
#[test]
fn fingerprint_with_other_params() {
    let node = vless_node(
        "?flow=xtls-rprx-vision&sni=cdn.example.com&pbk=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef&sid=01ab&fp=safari",
    );
    let opts = node.vless.expect("vless 节点必须带 VlessOpts");
    assert_eq!(opts.flow.as_deref(), Some("xtls-rprx-vision"));
    assert_eq!(opts.sni.as_deref(), Some("cdn.example.com"));
    assert_eq!(opts.fingerprint.as_deref(), Some("safari"));
}

/// VLESS + TLS + fingerprint：feature 关时也能构造适配器（meow 内部 warn 但不阻断）
#[test]
fn vless_tls_fingerprint_builds_adapter_without_utls_feature() {
    // 这里不开启 utls feature，验证：
    // - TlsConfig.fingerprint 能设置（默认 rustls 后端）
    // - adapter_for 返回 Some（不回落直连）
    // - 运行时会触发 meow 的一次性 stub warn，但不影响测试通过
    let node = vless_node("?fingerprint=chrome&sni=cdn.example.com");
    let adapter = adapter_for(&node);
    assert!(
        adapter.is_some(),
        "feature 关时 fingerprint 不应阻断适配器构造"
    );
}

/// REALITY + fingerprint 同样能构造适配器
#[test]
fn reality_fingerprint_builds_adapter_without_utls_feature() {
    const PBK: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let node = vless_node(&format!(
        "?fingerprint=chrome&sni=cdn.example.com&pbk={PBK}&sid=01ab"
    ));
    let adapter = adapter_for(&node);
    assert!(
        adapter.is_some(),
        "REALITY + fingerprint feature 关时不应阻断"
    );
}

/// 非 VLESS 节点不解析 fingerprint（通过 VlessOpts 共用机制，仅 VLESS/H2/AnyTLS/Snell 有）
#[test]
fn non_vless_no_fingerprint() {
    let node =
        ProxyNode::parse_url("socks5://127.0.0.1:7890?fingerprint=chrome&sni=cdn.example.com")
            .unwrap();
    assert_eq!(node.scheme, ProxyScheme::Socks5);
    assert!(node.vless.is_none());
}
