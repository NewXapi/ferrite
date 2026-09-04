//! `ToolId` 解析 + `ToolBinding` alias 校验的 wire-contract 回归测试。
//!
//! 防止 `a95d674` 的两个回归：
//! 1. `ToolId::parse("builtin:read:file")` 必须失败——native name 不能再含 `:`。
//! 2. `ToolBinding::new` 必须把 model alias 当作 OpenAI function name 校验
//!    （`^[A-Za-z0-9_-]{1,64}$`），防止带空格、冒号或过长字符串进入 wire payload。

use harness_tools::{ToolBinding, ToolDescriptor, ToolError, ToolId, ToolProviderId};
use serde_json::json;

fn descriptor(id: ToolId) -> ToolDescriptor {
    ToolDescriptor {
        id,
        title: None,
        description: None,
        input_schema: json!({ "type": "object" }),
        output_schema: None,
        annotations: json!({}),
    }
}

#[test]
fn tool_id_parse_rejects_native_name_containing_separator() {
    // The native name segment is delimited by `:`; another `:` in the input
    // means a malformed id and must error out before reaching `parse`.
    let err = ToolId::parse("builtin:read:file").expect_err("multi-colon id must be rejected");
    let message = format!("{err}");
    assert!(
        message.contains("native_name_invalid") || message.contains("must not contain"),
        "expected native_name_invalid error, got: {message}"
    );

    // Also via the serde deserializer path so the same shape is rejected when
    // it arrives from JSON.
    let err = serde_json::from_str::<ToolId>("\"builtin:read:file\"")
        .expect_err("serde path must reject embedded `:`");
    assert!(
        format!("{err}").contains("native_name_invalid"),
        "serde error should mention native_name_invalid; got: {err}"
    );

    // The constructor path itself rejects the same shape.
    let provider = ToolProviderId::builtin();
    let err = ToolId::new(&provider, "read:file").expect_err("ToolId::new must reject `:`");
    assert!(matches!(err, ToolError::InvalidData(_)));
}

#[test]
fn tool_id_parse_accepts_well_formed_id() {
    let parsed = ToolId::parse("builtin:read_file").expect("simple id parses");
    assert_eq!(parsed.native_name(), "read_file");
    assert!(parsed.is_builtin());
}

#[test]
fn tool_binding_alias_rejects_invalid_openai_function_names() {
    let read_id = ToolId::builtin("read_file").expect("tool id");

    // Empty
    let err = ToolBinding::new(descriptor(read_id.clone()), "", Some(2))
        .expect_err("empty alias must be rejected");
    assert!(matches!(err, ToolError::InvalidData(_)));

    // Whitespace
    let err = ToolBinding::new(descriptor(read_id.clone()), "read file", Some(2))
        .expect_err("alias with space must be rejected");
    assert!(matches!(err, ToolError::InvalidData(_)));

    // Colon — would collide with the `provider:native` separator convention.
    let err = ToolBinding::new(descriptor(read_id.clone()), "read:file", Some(2))
        .expect_err("alias with colon must be rejected");
    assert!(matches!(err, ToolError::InvalidData(_)));

    // Unicode / non-ASCII — function names on the wire are ASCII-only.
    let err = ToolBinding::new(descriptor(read_id.clone()), "café", Some(2))
        .expect_err("unicode alias must be rejected");
    assert!(matches!(err, ToolError::InvalidData(_)));

    // 65 chars — one over the OpenAI function-name ceiling.
    let oversize = "a".repeat(65);
    let err = ToolBinding::new(descriptor(read_id.clone()), oversize.as_str(), Some(2))
        .expect_err("65-char alias must be rejected");
    assert!(matches!(err, ToolError::InvalidData(_)));
}

#[test]
fn tool_binding_alias_accepts_valid_openai_function_names() {
    let read_id = ToolId::builtin("read_file").expect("tool id");

    for alias in [
        "read",
        "read_file",
        "ReadFile",
        "read-file",
        "read_file_v2",
        "a",
        "a".repeat(64).as_str(),
    ] {
        let binding = ToolBinding::new(descriptor(read_id.clone()), alias, Some(2))
            .unwrap_or_else(|err| panic!("alias `{alias}` must be accepted, got {err:?}"));
        assert_eq!(binding.model_alias(), alias);
    }
}
