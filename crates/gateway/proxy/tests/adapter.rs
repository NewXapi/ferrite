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
        priority: 0,
    };
    let adapter_opt = adapter_for(&node);
    assert!(adapter_opt.is_none());
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
        priority: 0,
    };
    let adapter_opt = adapter_for(&node);
    assert!(adapter_opt.is_none());
}
