use gateway_proxy::adapter::{adapter_for, tcp_metadata};
use gateway_proxy::node::{BasicAuth, ProxyNode, ProxyScheme};
use meow_common::adapter_type::ConnType;
use meow_common::{AdapterType, Network};

#[test]
fn test_tcp_metadata_host_port() {
    let meta = tcp_metadata("example.com", 8080);
    assert_eq!(meta.host, "example.com");
    assert_eq!(meta.dst_port, 8080);
    assert_eq!(meta.conn_type, ConnType::Http);
    assert_eq!(meta.network, Network::Tcp);
}

/// http/socks5 由 reqwest（workspace `socks` feature）处理，`manager` 也只为它们
/// 产出 `ProxyClient::Reqwest`——所以 `adapter_for` 必须返回 `None`。
/// 旧实现会为它们造一个从未被用过的 meow 适配器（死代码）。
#[test]
fn test_adapter_for_reqwest_schemes_have_no_adapter() {
    for scheme in [ProxyScheme::Http, ProxyScheme::Socks5] {
        let node = ProxyNode {
            id: 1,
            scheme,
            host: "proxy.example.com".to_string(),
            port: 1080,
            auth: Some(BasicAuth {
                user: "user".to_string(),
                pass: "pass".to_string(),
            }),
            channel_keys: vec!["test-channel".to_string()],
            vless: None,
            priority: 0,
        };
        assert!(
            adapter_for(&node).is_none(),
            "{scheme:?} 该走 reqwest 而非 meow 适配器"
        );
    }
}

#[test]
fn test_adapter_for_vless_invalid_uuid() {
    let node = ProxyNode {
        id: 2,
        scheme: ProxyScheme::Vless,
        host: "proxy.example.com".to_string(),
        port: 443,
        auth: Some(BasicAuth {
            user: "invalid-uuid".to_string(),
            pass: "".to_string(),
        }),
        channel_keys: vec!["test-channel".to_string()],
        vless: None,
        priority: 0,
    };
    let adapter_opt = adapter_for(&node);
    assert!(adapter_opt.is_none());
}

/// 合法 UUID 的 VLESS 节点走 require_uuid → VlessAdapter 的 happy path。
/// 与 invalid-UUID 用例互为镜像：一个证明拒绝路径，一个证明接受路径。
#[test]
fn test_adapter_for_vless_valid_uuid() {
    let node = ProxyNode {
        id: 4,
        scheme: ProxyScheme::Vless,
        host: "proxy.example.com".to_string(),
        port: 443,
        auth: Some(BasicAuth {
            user: "11111111-2222-3333-4444-555555555555".to_string(),
            pass: String::new(),
        }),
        vless: None,
        channel_keys: vec!["test-channel".to_string()],
        priority: 0,
    };
    let adapter = adapter_for(&node);
    assert!(adapter.is_some(), "合法 UUID 的 vless 必须构造出适配器");
    assert_eq!(adapter.unwrap().adapter_type(), AdapterType::Vless);
}

#[test]
fn test_adapter_for_direct() {
    let node = ProxyNode {
        id: 3,
        scheme: ProxyScheme::Direct,
        host: "".to_string(),
        port: 0,
        auth: None,
        channel_keys: vec!["test-channel".to_string()],
        vless: None,
        priority: 0,
    };
    let adapter_opt = adapter_for(&node);
    assert!(adapter_opt.is_none());
}

/// trojan 密码在 user 字段（`trojan://pass@host`），clash 侧必须映射到 password 键。
/// OCR 抓的回归：一度错取 pass 字段导致 trojan 节点全部空密码。
#[test]
fn trojan_password_lands_in_clash_password() {
    let node = ProxyNode::parse_url("trojan://sekret@example.com:443").expect("parse");
    let adapter = adapter_for(&node).expect("trojan 节点必须能构造适配器");
    // 适配器构造成功 = meow-config 收到了非空 password（否则 parse_proxy 报
    // "missing password" 返回 Err → adapter_for 为 None）
    assert_eq!(
        adapter.name(),
        "node-0-trojan",
        "parse_url 产出的节点 id=0，name 由 id 派生"
    );
}
