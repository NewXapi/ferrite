//! users page 接口薄壳不变量测试。
//! 真实后端接入后,筛选项(label 枚举)仍是 UI 约定,需保证首项为「全部」且不参与过滤。

use admin_page_users::api;

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
