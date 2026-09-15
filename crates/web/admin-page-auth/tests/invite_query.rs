//! 注册页 invite query 解析契约: `?invite=<inviter user_key>` → 注册请求透传。
//!
//! 解析失败 (无参数 / 空值 / 格式异常) 一律返回 None, 注册流程不阻断;
//! 后端再对非法邀请码 warn 降级。这里只测前端解析, 不测后端归属。

use admin_page_auth::api::parse_invite_query;

#[test]
fn plain_invite_query() {
    assert_eq!(
        parse_invite_query("?invite=abc-123"),
        Some("abc-123".into())
    );
}

#[test]
fn invite_among_other_params() {
    assert_eq!(
        parse_invite_query("?utm=xa&invite=deadbeef&ref=home"),
        Some("deadbeef".into())
    );
}

#[test]
fn no_question_mark_prefix() {
    assert_eq!(parse_invite_query("invite=abc"), Some("abc".into()));
}

#[test]
fn missing_invite_returns_none() {
    assert_eq!(parse_invite_query("?utm=xa"), None);
    assert_eq!(parse_invite_query(""), None);
}

#[test]
fn empty_invite_value_returns_none() {
    // ?invite= (邀请码缺失) 视为无邀请, 不传空串给后端
    assert_eq!(parse_invite_query("?invite="), None);
    assert_eq!(parse_invite_query("?invite=&a=1"), None);
}

#[test]
fn first_invite_wins_when_repeated() {
    assert_eq!(
        parse_invite_query("?invite=first&invite=second"),
        Some("first".into())
    );
}

#[test]
fn invite_substring_keys_do_not_match() {
    // invitee / invited 之类前缀键不得误命中
    assert_eq!(parse_invite_query("?invitee=abc"), None);
}
