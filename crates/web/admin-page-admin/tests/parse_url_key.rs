//! `parse_url_key` 的形态测试:URL+Key 从粘贴文本中提取(4 种格式 + 阈值边界)。
//! 纯函数,单测放这里(不依赖 Dioxus runtime)。

use admin_page_admin::parse_url_key;

fn ok(url: &str, key: &str) -> Option<(String, String)> {
    Some((url.to_string(), key.to_string()))
}

#[test]
fn two_lines_bare_url_then_bare_key() {
    let text = "https://api.openai.com/v1\nsk-test1234567890abcdefghij\n";
    assert_eq!(
        parse_url_key(text),
        ok("https://api.openai.com/v1", "sk-test1234567890abcdefghij")
    );
}

#[test]
fn key_equals_value_form() {
    let text = "url = https://api.openai.com/v1\napi_key = sk-test1234567890abcdefghij\n";
    assert_eq!(
        parse_url_key(text),
        ok("https://api.openai.com/v1", "sk-test1234567890abcdefghij")
    );
}

#[test]
fn pipe_separated_single_line() {
    let text = "https://api.openai.com/v1 | sk-test1234567890abcdefghij";
    assert_eq!(
        parse_url_key(text),
        ok("https://api.openai.com/v1", "sk-test1234567890abcdefghij")
    );
}

#[test]
fn json_envelope_extracted() {
    let text = r#"base_url = https://x.com
key = sk-test1234567890abcdefghij"#;
    assert_eq!(
        parse_url_key(text),
        ok("https://x.com", "sk-test1234567890abcdefghij")
    );
}

#[test]
fn short_bare_key_rejected() {
    let text = "https://api.openai.com/v1\nsk-12345678901234\n";
    assert_eq!(parse_url_key(text), None);
}

#[test]
fn only_url_returns_none() {
    let text = "https://api.openai.com/v1\n";
    assert_eq!(parse_url_key(text), None);
}

#[test]
fn garbage_returns_none() {
    assert_eq!(parse_url_key("hello world"), None);
    assert_eq!(parse_url_key(""), None);
    assert_eq!(parse_url_key("\n\n\n"), None);
}

#[test]
fn rk_prefix_supported() {
    let text = "https://x.com\nrk-test1234567890abcdefghij";
    assert_eq!(
        parse_url_key(text),
        ok("https://x.com", "rk-test1234567890abcdefghij")
    );
}

#[test]
fn case_insensitive_key_name() {
    let text = "URL=https://x.com\nKEY=sk-test1234567890abcdefghij";
    assert_eq!(
        parse_url_key(text),
        ok("https://x.com", "sk-test1234567890abcdefghij")
    );
}
