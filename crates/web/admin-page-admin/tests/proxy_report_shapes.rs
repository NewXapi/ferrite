//! 代理节点运行态 report 响应的 serde 反序列化形状断言。
//!
//! 后端端点:`GET /api/proxy_nodes/report`(crates/api/admin-proxy/src/lib.rs
//! 的 `report` handler)——裸 JSON 信封 `{ "items": [...] }`,每行是
//! `ProxyNodeView`(camelCase)join 运行态 `stats`:
//! - disabled 行(或未被数据面装配的行)`stats` 恒为 null;
//! - `stats.lastDelayMs` 为 null 表示"从未探测/未装配",不是 0 延迟。
//!
//! 本文件用后端 handler 的真实字段名构造样例,断言页面消费的 DTO
//! (`admin_page_admin::system::ProxyReportResponse`)解码形状与后端一致,
//! 防止前后端字段名漂移导致面板静默归零或解码失败。

use admin_page_admin::system::{NodeRuntimeStats, ProxyNodeReportRow, ProxyReportResponse};

/// 后端真实形状:enabled 行带完整 stats + disabled 行 stats 为 null。
/// 字段名逐一对照 admin-proxy report handler 的 `json!` / ProxyNodeView。
#[test]
fn report_response_deserializes_backend_shape() {
    let raw = r#"{
        "items": [
            {
                "key": "0198a7b4-1111-7000-8000-000000000001",
                "name": "hkg-01",
                "urlMasked": "vless://***@hk.example.com:443?***",
                "channelKeys": ["openai", "claude"],
                "priority": 10,
                "enabled": true,
                "remark": "香港入口",
                "createdAt": "2026-09-12T08:00:00Z",
                "updatedAt": "2026-09-12T09:30:00Z",
                "stats": {
                    "inflight": 3,
                    "failureCount": 2,
                    "cooldownRemainingSecs": 45,
                    "lastDelayMs": 120
                }
            },
            {
                "key": "0198a7b4-2222-7000-8000-000000000002",
                "name": "sjp-02",
                "urlMasked": "ss://***@sg.example.com:8388?***",
                "channelKeys": ["openai"],
                "priority": 5,
                "enabled": false,
                "remark": "",
                "createdAt": "2026-09-12T08:05:00Z",
                "updatedAt": "2026-09-12T08:05:00Z",
                "stats": null
            }
        ]
    }"#;

    let resp: ProxyReportResponse =
        serde_json::from_str(raw).expect("report 裸 JSON 必须能按页面 DTO 解码");
    assert_eq!(resp.items.len(), 2, "信封 items 两条都应解析出来");

    let enabled = &resp.items[0];
    assert_eq!(enabled.key, "0198a7b4-1111-7000-8000-000000000001");
    assert_eq!(enabled.name, "hkg-01");
    assert_eq!(enabled.url_masked, "vless://***@hk.example.com:443?***");
    assert_eq!(
        enabled.channel_keys,
        vec!["openai".to_string(), "claude".to_string()]
    );
    assert_eq!(enabled.priority, 10);
    assert!(enabled.enabled);
    assert_eq!(enabled.remark, "香港入口");

    let stats = enabled.stats.as_ref().expect("enabled 行必须有运行态");
    assert_eq!(stats.inflight, 3);
    assert_eq!(stats.failure_count, 2);
    assert_eq!(stats.cooldown_remaining_secs, 45);
    assert_eq!(stats.last_delay_ms, Some(120), "探测过的节点延迟为具体值");

    let disabled = &resp.items[1];
    assert!(!disabled.enabled);
    assert!(
        disabled.stats.is_none(),
        "disabled 行没有运行时身份,stats 必须是 null 而不是全 0"
    );
}

/// `lastDelayMs: null` 与 `cooldownRemainingSecs: 0` 的诚实语义:
/// 从未探测的节点延迟是 None(显示占位符),未冷却的剩余秒数是 0。
#[test]
fn null_delay_and_zero_cooldown_keep_honest_semantics() {
    let raw = r#"{
        "items": [
            {
                "key": "k-1",
                "name": "never-probed",
                "urlMasked": "http://***@10.0.0.1:8080",
                "channelKeys": ["openai"],
                "priority": 1,
                "enabled": true,
                "remark": "",
                "stats": {
                    "inflight": 0,
                    "failureCount": 0,
                    "cooldownRemainingSecs": 0,
                    "lastDelayMs": null
                }
            }
        ]
    }"#;

    let resp: ProxyReportResponse = serde_json::from_str(raw).expect("null lastDelayMs 必须可解码");
    let stats: &NodeRuntimeStats = resp.items[0].stats.as_ref().expect("stats 存在");
    assert_eq!(stats.last_delay_ms, None, "null = 从未探测,不是 0ms");
    assert_eq!(stats.cooldown_remaining_secs, 0, "未冷却为 0(后端语义)");
    assert_eq!(stats.inflight, 0);
    assert_eq!(stats.failure_count, 0);
}

/// 字段名必须严格是后端的 camelCase 拼写:拼错(如 snake_case)的字段会被
/// serde 忽略并回落到 default(0),面板不崩但数据归零——此防御行为是
/// 有意的(见 system.rs 本地 DTO 注释),该测试把"形状漂移 = 静默归零"
/// 这个契约显式钉住,避免误以为 snake_case 也能解析。
#[test]
fn wrong_field_casing_falls_back_to_defaults() {
    let raw = r#"{
        "items": [
            {
                "key": "k-1",
                "name": "snake-cased",
                "url_masked": "http://***@10.0.0.1:8080",
                "channel_keys": ["openai"],
                "enabled": true,
                "stats": {
                    "inflight": 7,
                    "failure_count": 9,
                    "cooldown_remaining_secs": 30,
                    "last_delay_ms": 88
                }
            }
        ]
    }"#;

    let resp: ProxyReportResponse = serde_json::from_str(raw).expect("未知字段被忽略,解码不失败");
    let row = &resp.items[0];
    assert_eq!(row.url_masked, "", "snake_case url_masked 不是后端契约字段");
    assert!(
        row.channel_keys.is_empty(),
        "snake_case channel_keys 不被识别"
    );
    let stats = row.stats.as_ref().expect("stats 对象本身仍可解析");
    assert_eq!(
        stats.inflight, 7,
        "inflight 是单个小写词,camelCase 拼写不变"
    );
    assert_eq!(
        stats.failure_count, 0,
        "failureCount 才是后端字段,snake_case 被忽略"
    );
    assert_eq!(stats.cooldown_remaining_secs, 0);
    assert_eq!(stats.last_delay_ms, None);
}

/// 后端字段超集与缺省容忍:页面用不到的 createdAt/updatedAt 与未来新增
/// 字段被 serde 忽略;`stats` 字段整体缺省时回落 None 而不是解码失败。
#[test]
fn unknown_and_missing_fields_are_tolerated() {
    let raw = r#"{
        "items": [
            {
                "key": "k-1",
                "name": "minimal",
                "urlMasked": "socks5://***@10.0.0.2:1080",
                "channelKeys": ["claude"],
                "priority": 3,
                "enabled": true,
                "remark": "",
                "createdAt": "2026-09-12T08:00:00.123456789Z",
                "updatedAt": "2026-09-12T08:00:00.123456789Z",
                "someFutureField": { "nested": true }
            }
        ]
    }"#;

    let resp: ProxyReportResponse =
        serde_json::from_str(raw).expect("未知字段与纳秒时间戳必须被容忍");
    let row: &ProxyNodeReportRow = &resp.items[0];
    assert_eq!(row.name, "minimal");
    assert!(row.enabled);
    assert!(row.stats.is_none(), "stats 缺省回落 None,不构造假运行态");

    let empty: ProxyReportResponse = serde_json::from_str(r#"{ "items": [] }"#).expect("空列表");
    assert!(empty.items.is_empty(), "空 report 对应空态(无代理节点)");

    let bare: ProxyReportResponse = serde_json::from_str("{}").expect("items 缺省回落空列表");
    assert!(bare.items.is_empty(), "items 缺省时 DTO default 生效");
}
