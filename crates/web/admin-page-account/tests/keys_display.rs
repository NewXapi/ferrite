//! keys 面板展示层纯函数不变量: `short_key` 长标识短显、`used_pct` 进度条口径。
//!
//! 复制交互 (CreatedKeyView 明文复制 / ProfileItem 用户 ID 复制) 与进度条
//! DOM 渲染是浏览器行为, 测试环境测不了; 这里锁定纯函数的边界语义。

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

// ---- used_pct: KeyCard 进度条口径 (与 admin-page-users data.rs 同语义) ----

#[test]
fn used_pct_basic_ratios() {
    use admin_page_account::usage_support::used_pct;
    // 正常比例四舍五入; 超 50% 转琥珀 / 超 90% 转红的阈值由渲染层按返回值判断
    assert_eq!(used_pct(500_000, 250_000), 50, "恰好一半 = 50%");
    assert_eq!(used_pct(500_000, 499_999), 100, "99.9998% 四舍五入到 100");
    assert_eq!(used_pct(500_000, 600_000), 100, "超用 clamp 到 100, 不超界");
    assert_eq!(used_pct(500_000, 0), 0, "未使用 = 0%");
}

#[test]
fn used_pct_zero_or_negative_quota_is_zero() {
    use admin_page_account::usage_support::used_pct;
    // quota <= 0 (无限额度 / 未设限额 / 脏数据) 一律 0:
    // 既避免除零 panic, 也让渲染层以「进度条不渲染」为约定
    assert_eq!(used_pct(0, 0), 0, "quota=0 (未设限额) = 0%");
    assert_eq!(used_pct(0, 999_999), 0, "quota=0 且有用量仍 = 0%, 不除零");
    assert_eq!(used_pct(-1, 100), 0, "负数 quota (脏数据) = 0%");
}
