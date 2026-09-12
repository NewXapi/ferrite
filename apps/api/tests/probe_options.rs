//! 探测循环纯函数测试：`parse_probe_options` 的 JSONB 解析与缺省逻辑、
//! `channel_probe_targets` 的每渠道目标解析（M3-C）。
//!
//! options 表的值是 JSONB，写坏（类型不符）不能炸探测循环——所有解析失败
//! 都落缺省。间隔下限 60s 是防误配打压机场的护栏。坏 base_url 行同理跳过。

use api::{channel_probe_targets, parse_probe_options};
use serde_json::json;

/// options 表没这两行 = 探测关闭（不插种子行是刻意设计）。
#[test]
fn missing_keys_default_to_disabled() {
    let opts = parse_probe_options(&[]);
    assert!(!opts.enabled);
    assert_eq!(opts.interval_secs, 300);
}

/// `proxy.probe_enabled = true` 打开探测。
#[test]
fn enabled_flag_turns_probe_on() {
    let rows = vec![("proxy.probe_enabled".to_string(), json!(true))];
    let opts = parse_probe_options(&rows);
    assert!(opts.enabled);
    assert_eq!(opts.interval_secs, 300, "interval 缺键用缺省");
}

/// 值类型写坏（bool 位置放了字符串）按缺省处理，不 panic。
#[test]
fn wrong_type_falls_back_to_default() {
    let rows = vec![
        ("proxy.probe_enabled".to_string(), json!("yes")),
        ("proxy.probe_interval_secs".to_string(), json!("fast")),
    ];
    let opts = parse_probe_options(&rows);
    assert!(!opts.enabled);
    assert_eq!(opts.interval_secs, 300);
}

/// 间隔下限 60s：误配 1s 会变成对机场的持续打压。
#[test]
fn interval_has_floor_of_sixty_seconds() {
    let rows = vec![
        ("proxy.probe_enabled".to_string(), json!(true)),
        ("proxy.probe_interval_secs".to_string(), json!(1)),
    ];
    let opts = parse_probe_options(&rows);
    assert!(opts.enabled);
    assert_eq!(opts.interval_secs, 60);

    let rows = vec![("proxy.probe_interval_secs".to_string(), json!(600))];
    assert_eq!(parse_probe_options(&rows).interval_secs, 600, "合法值不动");
}

/// 无关键被忽略，不影响解析。
#[test]
fn unrelated_keys_are_ignored() {
    let rows = vec![("site.registration_enabled".to_string(), json!(true))];
    let opts = parse_probe_options(&rows);
    assert_eq!(opts, api::ProbeOptions::default());
}

/// 合法行 → `host:443`：path/query 丢弃只取 host；无显式端口一律落 443
/// （近似维持现状，http 渠道也探 443）；显式 `:443` 与缺省等价。
#[test]
fn channel_targets_take_host_443() {
    let rows = vec![
        (
            "cla".to_string(),
            "https://api.example.com/v1?x=1".to_string(),
        ),
        ("clb".to_string(), "http://other.example.org".to_string()),
        ("clc".to_string(), "https://std.example.com:443".to_string()),
    ];
    assert_eq!(
        channel_probe_targets(&rows),
        vec![
            ("cla".to_string(), "api.example.com:443".to_string()),
            ("clb".to_string(), "other.example.org:443".to_string()),
            ("clc".to_string(), "std.example.com:443".to_string()),
        ]
    );
}

/// 解析失败 / 无 host / 显式非 443 端口的行跳过，好行按原顺序保留。
/// 探测恒拨 443：给 8443 渠道探 443 是探错目标，跳过好于误报。
#[test]
fn channel_targets_skip_bad_rows() {
    let rows = vec![
        ("garbage".to_string(), "not a url".to_string()),
        ("nohost".to_string(), "mailto:a@example.com".to_string()),
        ("emptyhost".to_string(), "file:///tmp/x".to_string()),
        (
            "oddport".to_string(),
            "https://api.example.com:8443".to_string(),
        ),
        ("good".to_string(), "https://api.example.com".to_string()),
    ];
    assert_eq!(
        channel_probe_targets(&rows),
        vec![("good".to_string(), "api.example.com:443".to_string())]
    );
}

/// IPv6 字面量与 userinfo 型 base_url 的解析钉住（OCR 审查采纳项）。
///
/// url crate 对 IPv6 的 `host_str()` **保留方括号**（`[2001:db8::1]`），
/// 拼出的目标 `[2001:db8::1]:443` 正是 probe.rs `split_host_port` 需要的形态
/// （它按最后一个冒号切 host/port，IPv6 必须靠方括号界定）。userinfo 只取 host，
/// 凭据不进目标字符串——这条把两段链路对 IPv6/凭据的约定钉在一起。
#[test]
fn channel_targets_ipv6_and_userinfo() {
    let rows = vec![
        (
            "ipv6".to_string(),
            "https://[2001:db8::1]:443/v1".to_string(),
        ),
        (
            "authed".to_string(),
            "https://user:secret@api.example.com/v1".to_string(),
        ),
    ];
    let got = channel_probe_targets(&rows);
    assert_eq!(got.len(), 2);
    assert_eq!(
        got[0],
        ("ipv6".to_string(), "[2001:db8::1]:443".to_string())
    );
    assert_eq!(
        got[1],
        ("authed".to_string(), "api.example.com:443".to_string())
    );
    // 凭据不得出现在探测目标里
    assert!(!got.iter().any(|(_, t)| t.contains("secret")));
}

/// 无 enabled 渠道 = 空输出（循环本轮零次探测，不 panic）。
#[test]
fn channel_targets_empty_input_is_empty() {
    assert!(channel_probe_targets(&[]).is_empty());
}
