//! users page 接口薄壳不变量测试。
//!
//! 筛选项(label 枚举)是 UI 约定:状态/角色固定枚举需保证首项为「全部」
//! 且不参与过滤;分组自 2026-09 起改走后端 `GET /api/group`,静态兜底
//! 只剩「全部」一项(真实分组由 `list_groups_api` 异步注入)。

use admin_page_users::api;

#[test]
fn filter_labels_have_all_first() {
    // 状态 / 角色:首项 "全部" 表示不过滤
    assert_eq!(api::fetch_statuses()[0].0, "全部", "statuses 首项 = 全部");
    assert_eq!(api::fetch_roles()[0].0, "全部", "roles 首项 = 全部");
    assert_eq!(api::fetch_statuses()[0].1, 0, "statuses 全部项 value = 0");
    assert_eq!(api::fetch_roles()[0].1, 0, "roles 全部项 value = 0");
    // 分组:静态兜底只剩「全部」(value 空串 = 不过滤),其余来自后端
    assert_eq!(api::fetch_groups()[0].0, "全部", "groups 首项 = 全部");
    assert_eq!(api::fetch_groups()[0].1, "", "groups 全部项 value 为空");
    assert_eq!(api::fetch_groups().len(), 1, "groups 静态兜底仅 1 项");
}

#[test]
fn create_user_request_serializes_camel_case() {
    // camelCase wire:后端 CreateUserRequest 逐字段对齐
    let req = api::CreateUserRequest {
        username: "zhangna".into(),
        password: "hunter2pass".into(),
        email: Some("z@example.com".into()),
        role: 10,
        quota: 5_000_000,
        groups: vec!["default".into(), "vip".into()],
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(json.contains("\"username\":\"zhangna\""), "{json}");
    assert!(json.contains("\"role\":10"), "{json}");
    assert!(json.contains("\"quota\":5000000"), "{json}");
    assert!(json.contains("\"groups\":[\"default\",\"vip\"]"), "{json}");
    // 邮箱缺省时不发送字段(后端 Option 收 None)
    let req2 = api::CreateUserRequest {
        username: "x".into(),
        password: "hunter2pass".into(),
        email: None,
        role: 1,
        quota: 0,
        groups: vec!["default".into()],
    };
    let json2 = serde_json::to_string(&req2).unwrap();
    assert!(!json2.contains("email"), "缺省 email 不应序列化: {json2}");
}

#[test]
fn create_user_request_omits_empty_groups() {
    // 空分组不发送字段 —— 后端缺省 ["default"],显式空会清空分组
    let req = api::CreateUserRequest {
        username: "x".into(),
        password: "hunter2pass".into(),
        email: None,
        role: 1,
        quota: 0,
        groups: Vec::new(),
    };
    let json = serde_json::to_string(&req).unwrap();
    assert!(!json.contains("groups"), "空 groups 不应序列化: {json}");
}
