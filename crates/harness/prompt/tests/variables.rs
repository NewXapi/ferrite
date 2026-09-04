//! `expand_variables` / `VariableContext` 集成测试。
//!
//! ponytail: 单元测试全部下沉到 tests/ —— Cargo 对 tests/ 目录默认只在
//! `cargo test` 编译，写 `#[cfg(test)]` 在这里反而是 boilerplate 错误。

use harness_prompt::{VariableContext, expand_variables};

#[test]
fn expands_char_and_user() {
    let ctx = VariableContext {
        character_name: "Alice".into(),
        user_name: "Bob".into(),
    };
    assert_eq!(expand_variables("hi {{char}}", &ctx), "hi Alice");
    assert_eq!(expand_variables("hi {{user}}", &ctx), "hi Bob");
    assert_eq!(
        expand_variables("{{char}} meets {{user}}", &ctx),
        "Alice meets Bob"
    );
}

#[test]
fn preserves_unknown_macros() {
    let ctx = VariableContext::default();
    assert_eq!(expand_variables("{{time}} stays", &ctx), "{{time}} stays");
    assert_eq!(expand_variables("/roll 1d6", &ctx), "/roll 1d6");
}

#[test]
fn preserves_unclosed_braces() {
    let ctx = VariableContext::default();
    assert_eq!(expand_variables("hello {{ world", &ctx), "hello {{ world");
}

#[test]
fn empty_inputs() {
    let ctx = VariableContext::default();
    assert_eq!(expand_variables("", &ctx), "");
    assert_eq!(expand_variables("plain text", &ctx), "plain text");
}
