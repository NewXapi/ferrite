//! 分组局部更新体（`SetGroupStatusBody` / `UpdateGroupRatioBody`）的形状断言。
//!
//! 后端分组更新入口（admin-catalog groups.rs 的 update）对
//! `UpdateGroupRequest` 的列语义是 COALESCE：字段缺席 = 保持现值。启停切换
//! 与倍率滑条各只写一列，请求体必须**恰含** `{"status"}` / `{"ratio"}`
//! 一个 key——多带 `name`/`modelWhitelist`/`remark` 等键会把表单未管理的
//! 列静默覆盖，漏带则该列不生效。
//!
//! 与 `tests/channel_update_body.rs` 同风格：本文件只测纯序列化形状，
//! 不发网络请求；键集合与值往返即钉死的 wire 契约。

use admin_page_admin::api::{SetGroupStatusBody, UpdateGroupRatioBody};
use serde_json::{Value, json};

/// 序列化并转成对象视图（wire 语义：请求体必须是 JSON object）。
fn to_map<T: serde::Serialize>(body: &T) -> serde_json::Map<String, Value> {
    let v = serde_json::to_value(body).expect("请求体必须可序列化");
    match v {
        Value::Object(m) => m,
        other => panic!("请求体应为 JSON object，实际为 {other:?}"),
    }
}

/// 启停体的 key 集合必须恰为 `{"status"}`：单键 = 其余列缺席 → 后端
/// COALESCE 保持现值，卡片页只动 status 不会误伤 name/whitelist 等列。
#[test]
fn set_group_status_body_has_exactly_status_key() {
    let m = to_map(&SetGroupStatusBody { status: 1 });
    assert_eq!(
        m.keys().collect::<Vec<_>>(),
        vec!["status"],
        "启停体只允许携带 status 一个 key: {m:?}"
    );
}

/// 倍率体的 key 集合必须恰为 `{"ratio"}`：滑条只写 ratio 列，
/// 多带任何键都会在 COALESCE 语义下覆盖对应列。
#[test]
fn update_group_ratio_body_has_exactly_ratio_key() {
    let m = to_map(&UpdateGroupRatioBody { ratio: 0.8 });
    assert_eq!(
        m.keys().collect::<Vec<_>>(),
        vec!["ratio"],
        "倍率体只允许携带 ratio 一个 key: {m:?}"
    );
}

/// 值往返：status 1/2（后端枚举：1=启用, 2=停用）、ratio 滑条最小步长
/// 0.05 与常用优惠档 0.8 都必须原样序列化（不得被取整/截断）。
#[test]
fn body_values_round_trip() {
    for status in [1i16, 2i16] {
        let m = to_map(&SetGroupStatusBody { status });
        assert_eq!(
            m.get("status"),
            Some(&json!(status)),
            "status {status} 应原样出现在请求体中"
        );
    }
    for ratio in [0.05f64, 0.8f64] {
        let m = to_map(&UpdateGroupRatioBody { ratio });
        assert_eq!(
            m.get("ratio"),
            Some(&json!(ratio)),
            "ratio {ratio} 应原样出现在请求体中"
        );
    }
}
