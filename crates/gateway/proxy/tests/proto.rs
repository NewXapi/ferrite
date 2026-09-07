use gateway_proxy::proto::{Address, NetLocation, StreamReader};

/// 覆盖 Address / NetLocation / StreamReader 核心契约。
/// `Address::from`：IPv4/IPv6/主机名解析。
/// shoes 原版语义：解析失败的串几乎都落为 Hostname；只有
/// 含 `:` 且被切分的形态才可能报错。此处对齐原版行为。
#[tokio::test]
async fn address_from_parses_variants() {
    // IPv4 地址应成功解析为 Address::Ipv4
    let ipv4 = Address::from("192.168.1.1").unwrap();
    assert!(matches!(ipv4, Address::Ipv4(_)));

    // IPv6 地址应成功解析为 Address::Ipv6
    let ipv6 = Address::from("2001:db8::1").unwrap();
    assert!(matches!(ipv6, Address::Ipv6(_)));

    // 主机名应解析为 Address::Hostname
    let hostname = Address::from("example.com").unwrap();
    assert!(matches!(hostname, Address::Hostname(_)));

    // 非数字串（含..）落为 Hostname（原版行为：不报错）
    let hostname2 = Address::from("invalid..address").unwrap();
    assert!(matches!(hostname2, Address::Hostname(_)));

    // 含 `:` 的非法串也落为 Hostname（原版行为）
    let hostname3 = Address::from("not:an:ipv6").unwrap();
    assert!(matches!(hostname3, Address::Hostname(_)));
}

/// `NetLocation::from_str`：带端口解析、无端口默认值。
/// shoes 原版不支持裸 IPv6（`::1` 会被 rfind(':') 错误切分）；
/// IPv6 目标必须显式端口且形如 `2001:db8::1:443`——原版同样不支持，
/// 所以这里只测 IPv4/主机名 + 端口语义，IPv6 走 Address::Ipv6 直构。
#[tokio::test]
async fn net_location_from_str_handles_port_variants() {
    // 带端口的 IPv4 地址应正确解析端口
    let loc1 = NetLocation::from_str("192.168.1.1:8080", None).unwrap();
    assert_eq!(loc1.port(), 8080);
    assert_eq!(loc1.address().to_string(), "192.168.1.1");

    // 无端口时使用默认端口
    let loc2 = NetLocation::from_str("example.com", Some(443)).unwrap();
    assert_eq!(loc2.port(), 443);

    // 无端口且无默认 → 报错
    assert!(NetLocation::from_str("example.com", None).is_err());

    // IPv6 直构（跳过 from_str 的字符串路径）
    let loc3 = NetLocation::new(Address::Ipv6("::1".parse().unwrap()), 80);
    assert!(matches!(loc3.address(), Address::Ipv6(_)));
    assert_eq!(loc3.port(), 80);
}

/// `StreamReader::read_line`：CRLF 结尾正常解析，bare LF 报错
#[tokio::test]
async fn stream_reader_read_line_crlf_and_bare_lf_error() {
    use std::io::Cursor;

    let mut data = Cursor::new(b"hello\r\nworld\r\nbare\nline");
    let mut reader = StreamReader::new();

    // 正常 CRLF 行应成功读取
    let line1 = reader.read_line(&mut data).await.unwrap();
    assert_eq!(line1, "hello");

    let line2 = reader.read_line(&mut data).await.unwrap();
    assert_eq!(line2, "world");

    // bare LF 应报错
    let err = reader.read_line(&mut data).await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    assert!(err.to_string().contains("CRLF"));
}

/// `StreamReader::read_u8/peek_slice/consume/unparsed_data`：字节级操作正确性
#[tokio::test]
async fn stream_reader_byte_operations() {
    use std::io::Cursor;

    let mut data = Cursor::new(b"hello");
    let mut reader = StreamReader::new();

    // read_u8 应正确读取单个字节
    let b = reader.read_u8(&mut data).await.unwrap();
    assert_eq!(b, b'h');

    // peek_slice 应查看数据但不消耗
    let peeked = reader.peek_slice(&mut data, 2).await.unwrap();
    assert_eq!(peeked, b"el");

    // consume 应消耗 peek 过的数据
    reader.consume(2);

    // unparsed_data 应返回剩余未解析数据（Cursor 一次全部进缓冲）
    let unparsed = reader.unparsed_data();
    assert_eq!(unparsed, b"lo");

    // 缓冲耗尽后再读 → EOF（读 3 字节超过剩余 2 字节）
    let err = reader.read_slice(&mut data, 3).await.unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::ConnectionAborted);
}
