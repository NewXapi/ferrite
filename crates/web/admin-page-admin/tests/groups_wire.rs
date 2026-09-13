//! GroupsPage 接线层纯函数单测:白名单解析/格式化 + DTO 形状。
//!
//! 不依赖网络/PG —— 只断言 `groups.rs` 暴露的纯函数行为,以及
//! `GroupUpsertRequest` 序列化形状与后端 `UpdateGroupRequest` 对齐。

use admin_page_admin::groups::{parse_whitelist, parse_whitelist_raw};
use contract::api::admin::{GroupDto, GroupUpsertRequest};
use serde_json::json;

#[test]
fn whitelist_raw_splits_and_trims() {
    assert_eq!(
        parse_whitelist_raw("gpt-4o, claude-3.5"),
        vec!["gpt-4o", "claude-3.5"]
    );
    assert_eq!(parse_whitelist_raw("  "), Vec::<String>::new());
    // 中文逗号/分号也认
    assert_eq!(
        parse_whitelist_raw("gpt;，claude；gemini"),
        vec!["gpt", "claude", "gemini"]
    );
    // 去重交给后端,这里只去空项
    assert_eq!(parse_whitelist_raw("a,,b,"), vec!["a", "b"]);
}

#[test]
fn whitelist_from_dto_round_trip() {
    // 后端 model_whitelist 是 JSON 字符串数组
    let v = json!(["gpt-4o", "claude-3.5"]);
    assert_eq!(parse_whitelist(&v), vec!["gpt-4o", "claude-3.5"]);

    // 空数组 / null / 非数组 → 空,不 panic
    assert_eq!(parse_whitelist(&json!([])), Vec::<String>::new());
    assert_eq!(parse_whitelist(&json!(null)), Vec::<String>::new());
    assert_eq!(parse_whitelist(&json!("oops")), Vec::<String>::new());
    // 非字符串项被过滤
    let mixed = json!([1, "ok", 2.5]);
    assert_eq!(parse_whitelist(&mixed), vec!["ok"]);
}

#[test]
fn upsert_request_serializes_camel_case() {
    let req = GroupUpsertRequest {
        name: "vip".into(),
        ratio: 1.2,
        model_whitelist: json!(["gpt-4o"]),
        remark: "VIP专线".into(),
    };
    let v = serde_json::to_value(&req).unwrap();
    assert_eq!(v["name"], "vip");
    assert_eq!(v["ratio"], 1.2);
    assert_eq!(v["modelWhitelist"], json!(["gpt-4o"]));
    assert_eq!(v["remark"], "VIP专线");
}

#[test]
fn group_dto_shape_matches_backend_view() {
    // 与后端 GroupView camelCase 对齐:后端字段缺省时 remark/时间戳不炸
    let raw = json!({
        "key": "1",
        "name": "default",
        "ratio": 1.0,
        "modelWhitelist": json!([]),
        "status": 1
    });
    let dto: GroupDto = serde_json::from_value(raw).unwrap();
    assert_eq!(dto.name, "default");
    assert_eq!(dto.ratio, 1.0);
    assert_eq!(dto.status, 1);
    // 后端没回 remark/时间戳时 default 到空串
    assert_eq!(dto.remark, "");
    assert_eq!(dto.created_at, "");
}
