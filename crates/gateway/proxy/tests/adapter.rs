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

#[test]
fn test_adapter_for_socks5() {
    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Socks5,
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
    let adapter_opt = adapter_for(&node);
    assert!(adapter_opt.is_some());
    let adapter = adapter_opt.unwrap();
    assert_eq!(adapter.adapter_type(), AdapterType::Socks5);
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
