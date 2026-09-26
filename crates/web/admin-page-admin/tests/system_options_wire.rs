//! 系统页 wire 形状断言:纯函数,不依赖网络。
//!
//! 覆盖两个纯函数:`format_option_value`(值→展示串)与 `option_editable`
//! (按值类型判断可编辑性)。DTO 解码形状由 `SystemOptionsPanel` 内的
//! `list_options_api` 调用在运行期覆盖,这里只做函数级断言。

use admin_page_admin::components::system_options::{format_option_value, option_editable};
use serde_json::json;

#[test]
fn format_option_value_scalars() {
    // 字符串原样,数字与布尔按文本展示
    assert_eq!(format_option_value(&json!("hi")), "hi");
    assert_eq!(format_option_value(&json!(42)), "42");
    assert_eq!(format_option_value(&json!(3.5)), "3.5");
    assert_eq!(format_option_value(&json!(true)), "true");
    // 空值统一占位符
    assert_eq!(format_option_value(&json!(null)), "—");
}

#[test]
fn format_option_value_nested_uses_json_text() {
    // 嵌套结构原样 JSON,不造数据
    let v = json!({"a": 1});
    assert_eq!(format_option_value(&v), "{\"a\":1}");
}

#[test]
fn option_editable_by_type() {
    // 数值与布尔可编辑
    assert!(option_editable(&json!(42)));
    assert!(option_editable(&json!(3.5)));
    assert!(option_editable(&json!(true)));
    // 字符串与嵌套结构只读
    assert!(!option_editable(&json!("hi")));
    assert!(!option_editable(&json!({"a": 1})));
    assert!(!option_editable(&json!(null)));
}
