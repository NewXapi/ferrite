//! estimate 测试 — 验证 token 估算。

use metering::estimate::{estimate_prompt_tokens, estimate_tokens};

#[test]
fn estimate_tokens_cjk_text() {
    let text = "你好世界"; // 4 CJK chars
    let tokens = estimate_tokens(text);
    // CJK ≈ 0.6 tok/char → 4 * 0.6 = 2.4 → ceil = 3
    assert!(tokens >= 2 && tokens <= 4, "got {tokens}");
}

#[test]
fn estimate_tokens_latin_text() {
    let text = "hello"; // 5 latin chars
    let tokens = estimate_tokens(text);
    // Latin ≈ 0.25 tok/char → 5 * 0.25 = 1.25 → ceil = 2
    assert!(tokens >= 1 && tokens <= 3, "got {tokens}");
}

#[test]
fn estimate_tokens_mixed_text() {
    let text = "hello 你好 123";
    let tokens = estimate_tokens(text);
    assert!(tokens > 0);
}

#[test]
fn estimate_tokens_empty_text() {
    assert_eq!(estimate_tokens(""), 0);
}

#[test]
fn estimate_prompt_tokens_extracts_content() {
    let body = bytes::Bytes::from(
        r#"{"messages":[{"role":"user","content":"hello world"},{"role":"assistant","content":"hi there"}]}"#,
    );
    let tokens = estimate_prompt_tokens(&body);
    assert!(tokens > 0);
}

#[test]
fn estimate_prompt_tokens_invalid_json() {
    let body = bytes::Bytes::from("not json");
    assert_eq!(estimate_prompt_tokens(&body), 0);
}

#[test]
fn estimate_prompt_tokens_empty_body() {
    let body = bytes::Bytes::from("");
    assert_eq!(estimate_prompt_tokens(&body), 0);
}