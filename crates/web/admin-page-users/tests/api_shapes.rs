//! users page 集成测试:api.rs 薄壳的数据形状不变量。
//! mock 换成真后端时,本测试保证薄壳签名与数据自洽性不变。

use page_users::api;

#[test]
fn users_have_unique_ids() {
    let users = api::fetch_users();
    assert!(!users.is_empty(), "至少一个用户");
    let mut ids: Vec<u32> = users.iter().map(|u| u.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), users.len(), "用户 id 唯一");
}

#[test]
fn users_quota_non_negative() {
    for u in api::fetch_users() {
        assert!(u.quota >= 0, "用户 {} 发放额度非负", u.username);
        assert!(u.used_quota >= 0, "用户 {} 消耗额度非负", u.username);
        assert!(
            u.used_quota <= u.quota.max(0),
            "用户 {} 消耗({}) 不应超过发放({})",
            u.username,
            u.used_quota,
            u.quota
        );
    }
}

#[test]
fn users_status_and_role_in_known_sets() {
    // api.rs 约定:status 1 启用 / 2 禁用; role 1/10/100
    for u in api::fetch_users() {
        assert!(
            matches!(u.status, 1 | 2),
            "用户 {} status={} 不在已知集合",
            u.username,
            u.status
        );
        assert!(
            matches!(u.role, 1 | 10 | 100),
            "用户 {} role={} 不在已知集合",
            u.username,
            u.role
        );
    }
}

#[test]
fn fetch_user_roundtrip() {
    let first = &api::fetch_users()[0];
    let got = api::fetch_user(first.id);
    assert!(got.is_some(), "fetch_user({}) 应命中", first.id);
    assert_eq!(got.unwrap().username, first.username);
    assert!(api::fetch_user(u32::MAX).is_none(), "不存在 id 返回 None");
}

#[test]
fn filter_labels_have_all_first() {
    // api.rs 文档约定:首项 "全部" 表示不过滤
    assert_eq!(api::fetch_groups()[0].0, "全部", "groups 首项 = 全部");
    assert_eq!(api::fetch_statuses()[0].0, "全部", "statuses 首项 = 全部");
    assert_eq!(api::fetch_roles()[0].0, "全部", "roles 首项 = 全部");
    // 全部项的 value 为空串/0 = 不过滤
    assert_eq!(api::fetch_groups()[0].1, "", "groups 全部项 value 为空");
    assert_eq!(api::fetch_statuses()[0].1, 0, "statuses 全部项 value = 0");
    assert_eq!(api::fetch_roles()[0].1, 0, "roles 全部项 value = 0");
}
