//! 网关渠道健康 wire DTO 形状不变量(与后端 `admin-observe/gateway_health.rs`
//! 的 `GatewayHealthView` 对账):
//! - camelCase 字段名 (后端 `#[serde(rename_all = "camelCase")]`);
//! - `state` 为 snake_case 字符串枚举 (cooling / slow_start / ok);
//! - 未知 state 由 `Default` 兜底为 `Ok` (不 panic、不误报异常态);
//! - join 失败字段 (channelKey / channelName / publicModel) 缺省 = None。

use client::{GatewayHealthView, HealthItemState};

#[test]
fn gateway_health_view_full_item() {
    let json = r#"{
        "items": [{
            "unitKey": "cc03/1:gpt-4",
            "channelKey": "00000000-0000-0000-0000-00000000cc03",
            "channelName": "OpenAI 官方",
            "publicModel": "gpt-4",
            "state": "cooling",
            "lastCoolingOutcome": "throttled",
            "remainingCooldownMs": 45000,
            "slowStartFactor": 0.25
        }]
    }"#;
    let view: GatewayHealthView = serde_json::from_str(json).unwrap();
    assert_eq!(view.items.len(), 1);
    let item = &view.items[0];
    assert_eq!(item.unit_key, "cc03/1:gpt-4");
    assert_eq!(
        item.channel_key.as_deref(),
        Some("00000000-0000-0000-0000-00000000cc03")
    );
    assert_eq!(item.channel_name.as_deref(), Some("OpenAI 官方"));
    assert_eq!(item.public_model.as_deref(), Some("gpt-4"));
    assert_eq!(item.state, HealthItemState::Cooling);
    assert_eq!(item.last_cooling_outcome.as_deref(), Some("throttled"));
    assert_eq!(item.remaining_cooldown_ms, 45000);
    assert!((item.slow_start_factor - 0.25).abs() < f64::EPSILON);
}

#[test]
fn state_enum_deserializes_snake_case() {
    for (s, want) in [
        ("cooling", HealthItemState::Cooling),
        ("slow_start", HealthItemState::SlowStart),
        ("ok", HealthItemState::Ok),
    ] {
        let v: HealthItemState = serde_json::from_value(serde_json::json!(s)).unwrap();
        assert_eq!(v, want, "state {s}");
        // 轮询判定只认异常态
        assert_eq!(
            v.is_abnormal(),
            want != HealthItemState::Ok,
            "is_abnormal {s}"
        );
    }
}

#[test]
fn unknown_state_falls_back_to_ok() {
    // 后端新增枚举值时前端不 panic、不误报异常,降级为健康态。
    let v: HealthItemState = serde_json::from_value(serde_json::json!("degraded")).unwrap();
    assert_eq!(v, HealthItemState::Ok);
}

#[test]
fn missing_join_fields_are_none() {
    // unit 已从渠道目录移除:join 字段全部缺省,None;条目本身保留。
    let json = r#"{
        "items": [{
            "unitKey": "dead/0:llama",
            "state": "ok",
            "remainingCooldownMs": 0,
            "slowStartFactor": 1.0
        }]
    }"#;
    let view: GatewayHealthView = serde_json::from_str(json).unwrap();
    let item = &view.items[0];
    assert!(item.channel_key.is_none());
    assert!(item.channel_name.is_none());
    assert!(item.public_model.is_none());
    assert!(item.last_cooling_outcome.is_none());
    assert_eq!(item.state, HealthItemState::Ok);
}

#[test]
fn empty_items_is_normal_state() {
    // 正常态:健康表无记录,items 空数组,DTO 正常解码 (不视为错误)。
    let view: GatewayHealthView = serde_json::from_str(r#"{"items": []}"#).unwrap();
    assert!(view.items.is_empty());
}

#[test]
fn default_view_is_empty() {
    // client.get 的信封解码路径 (data: null) 依赖 Default 兜底。
    let view = GatewayHealthView::default();
    assert!(view.items.is_empty());
}
