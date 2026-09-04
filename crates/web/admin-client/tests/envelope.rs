//! Envelope 反序列化形状不变量(从 src/lib.rs 内联测试迁出)。

use client::Envelope;

#[test]
fn envelope_success_with_data() {
    let json = r#"{"success":true,"message":"ok","data":42}"#;
    let env: Envelope<i32> = serde_json::from_str(json).unwrap();
    assert!(env.success);
    assert_eq!(env.message, "ok");
    assert_eq!(env.data, Some(42));
}

#[test]
fn envelope_success_false() {
    let json = r#"{"success":false,"message":"error","data":null}"#;
    let env: Envelope<serde_json::Value> = serde_json::from_str(json).unwrap();
    assert!(!env.success);
    assert_eq!(env.message, "error");
}

#[test]
fn envelope_null_data_with_unit() {
    let json = r#"{"success":true,"message":"ok","data":null}"#;
    let env: Envelope<()> = serde_json::from_str(json).unwrap();
    assert!(env.success);
    assert_eq!(env.data, None);
}
