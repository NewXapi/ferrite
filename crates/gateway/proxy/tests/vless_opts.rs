//! VLESS 传输层选项：URL query → [`VlessOpts`] → meow 适配器。
//!
//! Vision flow 与 REALITY 参数从 `vless://` 的 query 读；配置错误必须 warn +
//! 回落直连（返回 `None`），不能 panic 网关——节点 URL 来自用户配置。

use gateway_proxy::adapter::adapter_for;
use gateway_proxy::node::{ProxyNode, ProxyScheme};

/// 64 个 hex 字符的合法 REALITY 公钥（内容任意，只要长度对）。
const PBK: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8";
const UUID: &str = "11111111-2222-3333-4444-555555555555";

fn vless_node(query: &str) -> ProxyNode {
    let url = format!("vless://{UUID}@example.com:443{query}");
    let mut node = ProxyNode::parse_url(&url).expect("vless URL 必须能解析");
    node.channel_keys = vec!["openai".to_string()];
    node
}

/// query 里的 flow / sni / pbk / sid 必须落进 `VlessOpts`。
#[test]
fn query_fills_vless_opts() {
    let node = vless_node(&format!(
        "?flow=xtls-rprx-vision&sni=cdn.example.com&pbk={PBK}&sid=01ab"
    ));
    let opts = node.vless.expect("vless 节点必须带 VlessOpts");
    assert_eq!(opts.flow.as_deref(), Some("xtls-rprx-vision"));
    assert_eq!(opts.sni.as_deref(), Some("cdn.example.com"));
    assert_eq!(opts.pbk.as_deref(), Some(PBK));
    assert_eq!(opts.sid.as_deref(), Some("01ab"));
}

/// 非 VLESS 节点不该带 `VlessOpts`——避免其他 scheme 误读 query。
#[test]
fn non_vless_has_no_opts() {
    let node = ProxyNode::parse_url("socks5://127.0.0.1:7890?flow=xtls-rprx-vision").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Socks5);
    assert!(node.vless.is_none());
}

/// Vision + REALITY 的完整节点必须构造出适配器（TLS 层 + flow 都要接上）。
#[test]
fn vision_reality_builds_adapter() {
    let node = vless_node(&format!(
        "?flow=xtls-rprx-vision&sni=cdn.example.com&pbk={PBK}&sid=01"
    ));
    assert!(
        adapter_for(&node).is_some(),
        "Vision + REALITY 节点必须能构造 meow 适配器"
    );
}

/// 纯 VLESS（无 sni/pbk）也要能构造：不挂 TLS 层，明文 TCP 出口。
#[test]
fn plain_vless_builds_adapter() {
    assert!(adapter_for(&vless_node("")).is_some());
}

/// pbk 长度不对 → warn + 回落直连，绝不 panic。
#[test]
fn short_pbk_falls_back_to_direct() {
    let node = vless_node("?pbk=dead&sni=cdn.example.com");
    assert!(
        adapter_for(&node).is_none(),
        "非法 pbk 必须回落直连而不是构造出错误的 REALITY 层"
    );
}

/// sid 超过 8 字节（16 hex）→ 同样回落直连。
#[test]
fn oversized_sid_falls_back_to_direct() {
    let node = vless_node(&format!("?pbk={PBK}&sid=00112233445566778899"));
    assert!(adapter_for(&node).is_none(), "sid 超长必须回落直连");
}

/// 废弃/未知 flow 必须回落直连而**不是**静默降级为非 Vision。
///
/// 旧实现 warn 后当普通 VLESS 用——用户以为有 Vision 保护，实际没有。
/// meow-config 按 ADR-0002 Class A 硬拒（`xtls-rprx-direct` / `xtls-rprx-splice`
/// 是 xray 已废弃的不安全 flow；未知值可能跳过预期的安全处理）。
#[test]
fn deprecated_or_unknown_flow_falls_back_to_direct() {
    for flow in ["xtls-rprx-direct", "xtls-rprx-splice", "not-a-flow"] {
        let node = vless_node(&format!("?flow={flow}"));
        assert!(
            adapter_for(&node).is_none(),
            "flow={flow} 必须回落直连，不能静默当非 Vision 用"
        );
    }
}
