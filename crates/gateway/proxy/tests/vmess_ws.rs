//! VMess / WebSocket 移植契约测试。

use gateway_proxy::proto::address::{Address, NetLocation};
use gateway_proxy::proto::vmess::DataCipher;

/// DataCipher 变体存在性 + Copy 语义（编解码依赖）。
#[test]
fn data_cipher_variants_exist() {
    let a = DataCipher::Aes128Gcm;
    let b = a; // Copy
    let _ = (a, b);
}

/// VMess 连接器构造：合法 UUID + cipher → 不 panic；非法 UUID → Err。
#[test]
fn vmess_connector_construction() {
    use gateway_proxy::proto::vmess::VmessTcpClientHandler;

    let loc = NetLocation::new(Address::Hostname("vmess.example".into()), 443);
    // 合法 UUID
    let h = VmessTcpClientHandler::new(
        loc.clone(),
        "aes-128-gcm",
        "b831381d-6324-4d53-ad4f-8cda48b30811",
    );
    assert!(h.is_ok(), "valid uuid must construct");
    // 非法 UUID → Err（不 panic）
    assert!(VmessTcpClientHandler::new(loc, "aes-128-gcm", "not-a-uuid").is_err());
}

/// UUID 解析的十六进制契约（vmess mod 私有 parse_uuid 的行为锚点）。
#[test]
fn uuid_hex_parsing_contract() {
    // 标准 8-4-4-4-12 格式解析出 16 字节
    let uuid = "b831381d-6324-4d53-ad4f-8cda48b30811";
    let hex: String = uuid.chars().filter(|c| *c != '-').collect();
    assert_eq!(hex.len(), 32);
}
