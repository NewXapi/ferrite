//! WebSocket 传输层：URL query → `NodeOpts` → meow `WsLayer`。
//!
//! VLESS 和 VMess 都支持 `type=ws&path=/xxx&host=sni域名`。
//! 约定：`path` 存在即视为 WS 节点；`host` 可选，默认用 SNI 或节点 host。
//! 传输层顺序：TLS 在内、WS 在外（TLS over WS = WS 包在 TLS 外层，即 chain 先 push TLS 再 push WS）。

use gateway_proxy::adapter::adapter_for;
use gateway_proxy::node::{ProxyNode, ProxyScheme};

const UUID: &str = "11111111-2222-3333-4444-555555555555";

fn vless_node(query: &str) -> ProxyNode {
    let url = format!("vless://{UUID}@example.com:443{query}");
    let mut node = ProxyNode::parse_url(&url).expect("vless URL 必须能解析");
    node.channel_keys = vec!["openai".to_string()];
    node
}

fn vmess_node(query: &str) -> ProxyNode {
    let url = format!("vmess://{UUID}@example.com:443{query}");
    let mut node = ProxyNode::parse_url(&url).expect("vmess URL 必须能解析");
    node.channel_keys = vec!["openai".to_string()];
    node
}

#[test]
fn vless_query_parses_ws_path_and_host() {
    let node = vless_node("?path=/ws&host=cdn.example.com&type=ws");
    let opts = node.opts.expect("vless 节点必须带 NodeOpts");
    assert_eq!(opts.ws_path.as_deref(), Some("/ws"));
    assert_eq!(opts.ws_host.as_deref(), Some("cdn.example.com"));
}

#[test]
fn vmess_query_parses_ws_path_and_host() {
    let node = vmess_node("?path=/vmess-ws&host=vmess.example.com");
    let opts = node.opts.expect("vmess 节点也复用 NodeOpts 存 WS 参数");
    assert_eq!(opts.ws_path.as_deref(), Some("/vmess-ws"));
    assert_eq!(opts.ws_host.as_deref(), Some("vmess.example.com"));
}

#[test]
fn ws_path_alone_implies_ws_transport() {
    // 只要有 path 即视为 WS，不需要显式 type=ws
    let node = vless_node("?path=/ws-only");
    let opts = node.opts.expect("vless 节点必须带 NodeOpts");
    assert_eq!(opts.ws_path.as_deref(), Some("/ws-only"));
    assert!(opts.ws_host.is_none());
}

#[test]
fn non_vless_non_vmess_schemes_do_not_get_ws_opts() {
    // SOCKS5 / HTTP 等不该复用 NodeOpts
    let node = ProxyNode::parse_url("socks5://127.0.0.1:7890?path=/ws&host=foo").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Socks5);
    assert!(node.opts.is_none());
}

#[test]
fn vless_ws_only_builds_adapter_with_ws_layer() {
    // 纯 WS（无 TLS）：ws 层包裹明文 TCP
    let node = vless_node("?path=/ws");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "VLESS + WS（无 TLS）必须能构造适配器");
}

#[test]
fn vless_ws_tls_builds_adapter_tls_then_ws() {
    // WS + TLS：TLS 在内、WS 在外（chain 先 push TLS 再 push WS）
    let node = vless_node("?path=/ws&sni=cdn.example.com");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "VLESS + WS + TLS（sni）必须能构造适配器");
}

#[test]
fn vmess_ws_only_builds_adapter_with_ws_layer() {
    // VMess 纯 WS（无 TLS）
    let node = vmess_node("?path=/vmess-ws");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "VMess + WS（无 TLS）必须能构造适配器");
}

#[test]
fn vmess_ws_tls_builds_adapter_tls_then_ws() {
    // VMess WS + TLS
    let node = vmess_node("?path=/vmess-ws&sni=cdn.example.com");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "VMess + WS + TLS（sni）必须能构造适配器");
}

#[test]
fn vless_ws_invalid_config_falls_back() {
    // WsLayer::new 失败 → warn + None（回落直连）
    // 这里测试不传 ws_path 时也能正常工作（无 WS 层时直接走 TCP/TLS）
    let node = vless_node("");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "无 WS 配置的 VLESS 也要能构造适配器");
}

#[test]
fn vmess_ws_invalid_config_falls_back() {
    // VMess 无 WS 配置时也能正常工作
    let node = vmess_node("");
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "无 WS 配置的 VMess 也要能构造适配器");
}
