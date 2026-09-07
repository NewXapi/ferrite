// Copyright (c) 2026 ferrite
// Licensed under the MIT License
// ported from shoes (MIT)

#[cfg(test)]
mod tests {
    use gateway_proxy::proto::ProxyConnector as _;
    use gateway_proxy::proto::proxy_connector::ProxyConnector;
    use gateway_proxy::proto::shadowsocks::ShadowsocksCipher;
    use gateway_proxy::proto::shadowsocks::shadowsocks_tcp_handler::ShadowsocksTcpHandler;
    use gateway_proxy::proto::trojan::TrojanTcpClientHandler;
    use gateway_proxy::{Address, NetLocation};

    #[tokio::test]
    async fn test_socks_addr_roundtrip() {
        use gateway_proxy::proto::socks_addr::{read_location_direct, write_location_to_vec};
        use gateway_proxy::{Address as A, NetLocation as NL};

        let locations = vec![
            NL::new(A::Ipv4(std::net::Ipv4Addr::new(127, 0, 0, 1)), 8080),
            NL::new(A::Ipv6(std::net::Ipv6Addr::LOCALHOST), 443),
            NL::new(A::Hostname("example.com".to_string()), 80),
        ];

        for loc in locations {
            let bytes = write_location_to_vec(&loc);
            let mut cursor = std::io::Cursor::new(bytes);
            let read_loc = read_location_direct(&mut cursor).await.unwrap();
            assert_eq!(loc, read_loc);
        }
    }

    #[tokio::test]
    async fn test_ss_handler_constructs() {
        // ShadowsocksTcpHandler::new_client 构建路径验证：
        // cipher 解析 → key 派生 → handler 持有完整凭据。
        // proxy_location 通过 ProxyConnector trait 调用。
        let cipher = ShadowsocksCipher::try_from("aes-128-gcm").unwrap();
        let handler = ShadowsocksTcpHandler::new_client(
            NetLocation::new(Address::Hostname("proxy.example.com".to_string()), 8388),
            cipher,
            "test_password",
            false,
        );
        let loc = handler.proxy_location();
        assert_eq!(loc.to_string(), "proxy.example.com:8388");
    }

    #[tokio::test]
    async fn test_trojan_handler_constructs() {
        // TrojanTcpClientHandler::new 构建路径验证：
        // 密码 SHA224 哈希 + 服务器地址持有。
        let handler = TrojanTcpClientHandler::new(
            NetLocation::new(Address::Hostname("proxy.example.com".to_string()), 443),
            "test_password",
        );
        let loc = handler.proxy_location();
        assert_eq!(loc.to_string(), "proxy.example.com:443");
    }
}
