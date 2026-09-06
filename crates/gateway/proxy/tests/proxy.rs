//! proxy 测试套件
//!
//! 设计原则：每个测试验证一个明确的行为契约，注释解释：
//! - 测什么行为
//! - 为什么是这个预期
//! - 对应生产场景哪个环节

use gateway_proxy::node::{BasicAuth, ProxyNode, ProxyScheme};
use gateway_proxy::pool::{ProxyPool, ProxySnapshot};
use gateway_proxy::ssrf::{SsrfError, check_ip, validate_resolved, validate_url};
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use url::Url;

/// `parse_url`：各 scheme 正确映射 + 默认端口 + 认证提取 + 非法 scheme 报错
#[test]
fn parse_url_maps_schemes_and_defaults() {
    // http -> Http, 默认 8080
    let node = ProxyNode::parse_url("http://proxy.example.com").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Http);
    assert_eq!(node.host, "proxy.example.com");
    assert_eq!(node.port, 8080);
    assert!(node.auth.is_none());
    // https -> Http (都是 HTTP 代理隧道), 默认 8080
    let node = ProxyNode::parse_url("https://proxy.example.com").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Http);
    assert_eq!(node.port, 8080);
    // socks5 -> Socks5, 默认 1080
    let node = ProxyNode::parse_url("socks5://proxy.example.com").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Socks5);
    assert_eq!(node.port, 1080);
    // socks5h -> Socks5 (同 socks5, h 代表远程解析 DNS)
    let node = ProxyNode::parse_url("socks5h://proxy.example.com").unwrap();
    assert_eq!(node.scheme, ProxyScheme::Socks5);
    assert_eq!(node.port, 1080);
    // 显式端口优先于默认
    let node = ProxyNode::parse_url("http://proxy.example.com:3128").unwrap();
    assert_eq!(node.port, 3128);
    // 认证提取：user:pass
    let node = ProxyNode::parse_url("http://user:pass@proxy.example.com:3128").unwrap();
    assert_eq!(node.auth.as_ref().unwrap().user, "user");
    assert_eq!(node.auth.as_ref().unwrap().pass, "pass");
    // 认证：只有 user, 无 pass -> pass = ""
    let node = ProxyNode::parse_url("http://user@proxy.example.com:3128").unwrap();
    assert_eq!(node.auth.as_ref().unwrap().user, "user");
    assert_eq!(node.auth.as_ref().unwrap().pass, "");
    // 特殊字符：`Url::username()` 返回 percent-encoded 原文，parse_url 负责解码。
    // 不解码会让含 `@` 的用户名变成 `user%40domain`，代理认证失败。
    let node = ProxyNode::parse_url("http://user%40domain:p%2Fss@proxy.example.com").unwrap();
    assert_eq!(node.auth.as_ref().unwrap().user, "user@domain");
    assert_eq!(node.auth.as_ref().unwrap().pass, "p/ss");
    // 非法 scheme
    let err = ProxyNode::parse_url("ftp://proxy.example.com").unwrap_err();
    assert!(err.to_string().contains("unsupported proxy scheme"));
    // 缺失 host：url crate 在解析层就拒绝空 host，只断言报错不钉措辞。
    assert!(ProxyNode::parse_url("http://:3128").is_err());
    assert!(ProxyNode::parse_url("not-a-url").is_err());
}

/// `to_reqwest_proxy`：Direct 返回 None，其余返回 Proxy + 认证走 basic_auth
#[test]
fn to_reqwest_proxy_returns_correct_variant() {
    // Direct -> None
    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Direct,
        host: "".into(),
        port: 0,
        auth: None,
        channel_ids: vec![],
        priority: 0,
    };
    assert!(node.to_reqwest_proxy().unwrap().is_none());

    // Http
    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Http,
        host: "proxy.example.com".into(),
        port: 3128,
        auth: Some(BasicAuth {
            user: "u".into(),
            pass: "p".into(),
        }),
        channel_ids: vec![],
        priority: 0,
    };
    let proxy = node.to_reqwest_proxy().unwrap().unwrap();
    // 无法直接检查内部 URL，只能确认构建成功；实际拨号由 forward 处理
    drop(proxy);

    // Socks5
    let node = ProxyNode {
        id: 1,
        scheme: ProxyScheme::Socks5,
        host: "proxy.example.com".into(),
        port: 1080,
        auth: None,
        channel_ids: vec![],
        priority: 0,
    };
    let proxy = node.to_reqwest_proxy().unwrap().unwrap();
    drop(proxy);
}

/// `pick`：priority 分层 — 只从最高层选；空 channel 返回 None；install 整体替换
#[test]
fn pick_respects_priority_layers_and_replaces_on_install() {
    let pool = ProxyPool::new();
    let mut rng = StdRng::seed_from_u64(7);
    // 初始空
    assert!(pool.pick(1, &mut rng).is_none());

    // 安装两个节点，channel 1：prio 10 和 5
    let snap = ProxySnapshot {
        nodes: vec![
            ProxyNode {
                id: 1,
                scheme: ProxyScheme::Direct,
                host: "a".into(),
                port: 1,
                auth: None,
                channel_ids: vec![1],
                priority: 5,
            },
            ProxyNode {
                id: 2,
                scheme: ProxyScheme::Direct,
                host: "b".into(),
                port: 2,
                auth: None,
                channel_ids: vec![1],
                priority: 10,
            },
        ],
    };
    pool.install(snap);
    // 低优先层永不参与：不管 rng 怎么走，只能是 prio 10 的 id=2
    for _ in 0..20 {
        let picked = pool.pick(1, &mut rng).unwrap();
        assert_eq!(
            picked.id, 2,
            "only highest priority layer (10) should be picked"
        );
    }

    // 未配置代理的 channel
    assert!(pool.pick(999, &mut rng).is_none());

    // 重新 install：整体替换，旧索引（含 id=2）不残留
    let snap = ProxySnapshot {
        nodes: vec![ProxyNode {
            id: 3,
            scheme: ProxyScheme::Direct,
            host: "c".into(),
            port: 3,
            auth: None,
            channel_ids: vec![1],
            priority: 5,
        }],
    };
    pool.install(snap);
    for _ in 0..10 {
        assert_eq!(pool.pick(1, &mut rng).unwrap().id, 3);
    }
}

/// `pick`：同 priority 层内随机 — 注入固定种子后结果确定可复现。
///
/// 与 `dispatch::selector` 同一约定：rng 由调用方注入，测试不依赖全局随机源，
/// 所以这里能断言"两个节点都出现过"而不是概率性期望。
#[test]
fn pick_random_within_same_priority_layer() {
    let pool = ProxyPool::new();
    let snap = ProxySnapshot {
        nodes: vec![
            ProxyNode {
                id: 10,
                scheme: ProxyScheme::Direct,
                host: "a".into(),
                port: 1,
                auth: None,
                channel_ids: vec![1],
                priority: 10,
            },
            ProxyNode {
                id: 20,
                scheme: ProxyScheme::Direct,
                host: "b".into(),
                port: 2,
                auth: None,
                channel_ids: vec![1],
                priority: 10,
            },
        ],
    };
    pool.install(snap);

    let mut rng = StdRng::seed_from_u64(42);
    let mut ids: Vec<i64> = (0..50)
        .map(|_| pool.pick(1, &mut rng).unwrap().id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(
        ids,
        vec![10, 20],
        "both same-priority nodes must be reachable"
    );
}

/// SSRF: `check_ip` 拦截 loopback / private / link-local / multicast / unspecified / CGNAT
#[test]
fn check_ip_blocks_reserved_ranges() {
    // loopback
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))),
        Err(SsrfError::Loopback)
    ));
    assert!(matches!(
        check_ip(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1))),
        Err(SsrfError::Loopback)
    ));

    // private IPv4 (10/8, 172.16/12, 192.168/16)
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
        Err(SsrfError::PrivateIp)
    ));
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))),
        Err(SsrfError::PrivateIp)
    ));
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))),
        Err(SsrfError::PrivateIp)
    ));

    // link-local IPv4 (169.254/16)
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))),
        Err(SsrfError::LinkLocal)
    ));

    // multicast
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(224, 0, 0, 1))),
        Err(SsrfError::Multicast)
    ));
    assert!(matches!(
        check_ip(IpAddr::V6(Ipv6Addr::new(0xff00, 0, 0, 0, 0, 0, 0, 0))),
        Err(SsrfError::Multicast)
    ));

    // unspecified
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))),
        Err(SsrfError::Unspecified)
    ));
    assert!(matches!(
        check_ip(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0))),
        Err(SsrfError::Unspecified)
    ));

    // CGNAT 100.64.0.0/10
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))),
        Err(SsrfError::PrivateIp)
    ));
    assert!(matches!(
        check_ip(IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))),
        Err(SsrfError::PrivateIp)
    ));
    // CGNAT 边界外应放行
    assert!(check_ip(IpAddr::V4(Ipv4Addr::new(100, 63, 255, 255))).is_ok());
    assert!(check_ip(IpAddr::V4(Ipv4Addr::new(100, 128, 0, 0))).is_ok());

    // 公网 IP 放行
    assert!(check_ip(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))).is_ok());
    assert!(check_ip(IpAddr::V6(Ipv6Addr::from_str("2606:4700::1111").unwrap())).is_ok());
}

/// `::ffff:127.0.0.1` (IPv4-mapped IPv6) 必须被拦截 — 这是回归测试
#[test]
fn ipv4_mapped_ipv6_loopback_is_blocked() {
    // ::ffff:127.0.0.1
    let mapped = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001));
    assert!(matches!(check_ip(mapped), Err(SsrfError::Loopback)));
    // 另一个 loopback mapped
    let mapped2 = IpAddr::V6(Ipv6Addr::from_str("::ffff:127.0.0.1").unwrap());
    assert!(matches!(check_ip(mapped2), Err(SsrfError::Loopback)));
    // 非 loopback mapped 应放行（只要不是私有/环回）
    let mapped_public = IpAddr::V6(Ipv6Addr::from_str("::ffff:1.1.1.1").unwrap());
    assert!(check_ip(mapped_public).is_ok());
}

/// `validate_url`：仅校验 IP 字面量；域名不做 DNS 解析，放行（调用方须后续 validate_resolved）
#[test]
fn validate_url_ip_literals_checked_domains_skipped() {
    // IP 字面量被校验
    let url = Url::parse("http://127.0.0.1/").unwrap();
    assert!(matches!(validate_url(&url), Err(SsrfError::Loopback)));
    let url = Url::parse("http://1.1.1.1/").unwrap();
    assert!(validate_url(&url).is_ok());

    // 域名放行（不解析 DNS）
    let url = Url::parse("http://internal.example.com/").unwrap();
    assert!(validate_url(&url).is_ok());
    // 调用方必须 DNS 解析后再调用 validate_resolved
}

/// `validate_resolved`：混合列表里有一个内网 IP 就整体报错
#[test]
fn validate_resolved_fails_if_any_bad_ip() {
    let addrs = vec![
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),  // 公网
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)), // 内网
    ];
    let err = validate_resolved(&addrs).unwrap_err();
    assert!(matches!(err, SsrfError::PrivateIp));

    // 全公网 -> 返回第一个
    let addrs = vec![
        IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
        IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
    ];
    let first = validate_resolved(&addrs).unwrap();
    assert_eq!(first, IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)));

    // 空列表 -> Dns 错误
    let err = validate_resolved(&[]).unwrap_err();
    assert!(matches!(err, SsrfError::Dns(_)));
}
