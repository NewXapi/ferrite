//! 邀请链接拼装契约: `{origin}/register?invite={user_key}`。
//!
//! 链接由 rewards 面板从钱包 user_key 现拼 (无后端链接端点), 落地页原样回传
//! invite 参数。形状红则注册页收不到邀请码, 归属关系断裂。

use admin_page_account::api;

#[test]
fn invite_link_shape_is_origin_register_query() {
    let link = api::invite_link("https://ferrite.ai", "8b1c-uuid");
    assert_eq!(
        link, "https://ferrite.ai/register?invite=8b1c-uuid",
        "邀请链接 = origin + /register?invite=user_key"
    );
}

#[test]
fn invite_link_keeps_trailing_slash_origin_verbatim() {
    // origin 由浏览器 location.origin 返回 (无尾斜杠); 但即便调用方传了尾斜杠,
    // 也只是多一个斜杠, 注册页 query 解析按 &/= 切分不受影响 —— 断言逐字拼接,
    // 防止将来有人偷偷加 normalize 逻辑。
    let link = api::invite_link("https://ferrite.ai/", "abc");
    assert_eq!(link, "https://ferrite.ai//register?invite=abc");
}

#[test]
fn invite_link_uuid_safe_chars_unescaped() {
    // user_key 是 UUID (连字符), query value 里无需百分号编码。
    let link = api::invite_link("https://ferrite.ai", "11111111-2222-3333-4444-555555555555");
    assert_eq!(
        link,
        "https://ferrite.ai/register?invite=11111111-2222-3333-4444-555555555555"
    );
}
