//! 邀请链接拼装契约: `{origin}/register?invite={code}`。
//!
//! 链接由 rewards 面板从钱包 aff_code/user_key 现拼 (无后端链接端点), 落地页
//! 原样回传 invite 参数。形状红则注册页收不到邀请码, 归属关系断裂。

use admin_page_account::api;

#[test]
fn invite_link_falls_back_to_user_key_without_aff_code() {
    // aff_code 未生成 (老用户/钱包未加载完) → 回落 user_key, 旧链接形态不变。
    let link = api::invite_link("https://ferrite.ai", "8b1c-uuid", None);
    assert_eq!(
        link, "https://ferrite.ai/register?invite=8b1c-uuid",
        "无短码时邀请链接 = origin + /register?invite=user_key"
    );
}

#[test]
fn invite_link_prefers_aff_code_over_user_key() {
    // 有短码用短码: 注册页回传的 invite 值是短码, 后端 resolve_invite_code
    // 查 auth_users.aff_code 解析 (UUID 分支先兜旧链接)。
    let link = api::invite_link("https://ferrite.ai", "8b1c-uuid", Some("aB3x9Q"));
    assert_eq!(
        link, "https://ferrite.ai/register?invite=aB3x9Q",
        "短码必须覆盖 user_key 进链接"
    );
}

#[test]
fn invite_link_treats_empty_aff_code_as_absent() {
    // 后端 aff_code 可能是空串 (理论脏态) 而非 None: 空串当没有, 不造出
    // `?invite=` 空值链接 (落地页 parse 对空值返回 None, 链接就废了)。
    let link = api::invite_link("https://ferrite.ai", "8b1c-uuid", Some(""));
    assert_eq!(link, "https://ferrite.ai/register?invite=8b1c-uuid");
}

#[test]
fn invite_link_keeps_trailing_slash_origin_verbatim() {
    // origin 由浏览器 location.origin 返回 (无尾斜杠); 但即便调用方传了尾斜杠,
    // 也只是多一个斜杠, 注册页 query 解析按 &/= 切分不受影响 —— 断言逐字拼接,
    // 防止将来有人偷偷加 normalize 逻辑。
    let link = api::invite_link("https://ferrite.ai/", "abc", None);
    assert_eq!(link, "https://ferrite.ai//register?invite=abc");
}

#[test]
fn invite_link_uuid_safe_chars_unescaped() {
    // user_key 是 UUID (连字符), query value 里无需百分号编码。
    let link = api::invite_link(
        "https://ferrite.ai",
        "11111111-2222-3333-4444-555555555555",
        None,
    );
    assert_eq!(
        link,
        "https://ferrite.ai/register?invite=11111111-2222-3333-4444-555555555555"
    );
}
