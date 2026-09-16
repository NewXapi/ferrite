//! keys 面板展示层纯函数不变量: `short_key` 长标识短显。
//!
//! 复制交互 (CreatedKeyView 明文复制 / ProfileItem 用户 ID 复制) 是 DOM 行为,
//! 测试环境无浏览器测不了; 这里锁定短显纯函数的边界语义。

#[test]
fn short_key_normal_uuid_shrinks_to_head_tail() {
    // 正常 UUID 36 位 → 前 4 + … + 后 4 = 9 字符
    use admin_page_account::usage_support::short_key;
    let uuid = "3f8a1c2e-9b4d-4e7a-8c1f-2d5b6a7e8f90";
    let got = short_key(uuid);
    assert_eq!(got, "3f8a…8f90");
    assert!(got.chars().count() < uuid.chars().count());
}

#[test]
fn short_key_short_input_returned_as_is() {
    // ≤ 12 字符原样返回: 短串截断反而难认 (前4…后4 比原文还长)
    use admin_page_account::usage_support::short_key;
    assert_eq!(short_key("sk-abcd1234"), "sk-abcd1234");
    assert_eq!(
        short_key("123456789012"),
        "123456789012",
        "恰好 12 位 = 原样"
    );
}

#[test]
fn short_key_longer_than_twelve_shrinks() {
    // 13 位起触发短显; 多字节字符按 char 计, 不按字节
    // (「这是一段超过十二个字符的中文标识」共 16 个 char, 前4 = 这是一段, 后4 = 中文标识)
    use admin_page_account::usage_support::short_key;
    assert_eq!(short_key("abcdefghijklm"), "abcd…jklm");
    assert_eq!(
        short_key("这是一段超过十二个字符的中文标识"),
        "这是一段…中文标识"
    );
}

#[test]
fn short_key_empty_string_stays_empty() {
    use admin_page_account::usage_support::short_key;
    assert_eq!(short_key(""), "", "空串原样返回, 不产生占位字符");
}
