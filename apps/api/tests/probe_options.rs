//! `parse_probe_options` 的 JSONB 解析与缺省逻辑测试。
//!
//! options 表的值是 JSONB，写坏（类型不符）不能炸探测循环——所有解析失败
//! 都落缺省。间隔下限 60s 是防误配打压机场的护栏。

use api::parse_probe_options;
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
