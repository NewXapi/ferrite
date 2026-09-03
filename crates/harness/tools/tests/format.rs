//! Provider schema 规范化与 OpenAI `tools[]` 渲染的 wire-shape 回归测试。
//!
//! 防止 `a95d674` 的两个回归：
//! 1. Gemini sanitize 错误把 `properties` map 里的用户字段名（如 `title`、`$id`）
//!    当作 JSON Schema keyword 删除，破坏合法 schema。
//! 2. 适配器解析与枚举的 wire shape 漂移（串大小写、`chat_completion_source` 解析）。

use harness_tools::{
    AgentModelTool, AgentProviderAdapter, render_openai_tools, resolve_request_adapter,
    sanitize_schema_for_provider,
};
use serde_json::{Value, json};

fn descriptor() -> AgentModelTool {
    AgentModelTool {
        tool_id: harness_tools::ToolId::builtin("get_weather").expect("tool id"),
        model_alias: "get_weather".to_string(),
        description: Some("Read current weather for a city.".to_string()),
        input_schema: json!({
            "type": "object",
            "title": "WeatherArgs",
            "$id": "https://example.com/weather.json",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "human-readable label, not the schema keyword"
                },
                "$id": {
                    "type": "string",
                    "description": "trace identifier, not the schema keyword"
                },
                "city": { "type": "string" }
            },
            "required": ["title", "$id", "city"]
        }),
    }
}

#[test]
fn gemini_sanitize_preserves_property_names_matching_keywords() {
    // The user's properties map uses `title` and `$id` as field names. The
    // sanitizer must NOT strip them — they are domain data, not JSON Schema
    // keywords applied to the property *schema*.
    let tool = descriptor();
    let sanitized = sanitize_schema_for_provider(&tool.input_schema, AgentProviderAdapter::Gemini);

    let properties = sanitized
        .get("properties")
        .and_then(Value::as_object)
        .expect("properties preserved");

    assert!(
        properties.contains_key("title"),
        "Gemini sanitize dropped the user field `title`; sanitized = {sanitized}"
    );
    assert!(
        properties.contains_key("$id"),
        "Gemini sanitize dropped the user field `$id`; sanitized = {sanitized}"
    );

    let title_schema = &properties["title"];
    assert_eq!(
        title_schema.get("type").and_then(Value::as_str),
        Some("string")
    );
    assert_eq!(
        title_schema.get("description").and_then(Value::as_str),
        Some("human-readable label, not the schema keyword")
    );

    // Top-level schema-level `title`/`$id` ARE keyword noise and must be stripped.
    assert!(
        sanitized.get("title").is_none(),
        "schema-level `title` should be removed; got {sanitized}"
    );
    assert!(
        sanitized.get("$id").is_none(),
        "schema-level `$id` should be removed; got {sanitized}"
    );
}

#[test]
fn gemini_sanitize_strips_combinators_and_required_nested_arrays() {
    // Gemini rejects `allOf`/`anyOf`/etc. and nested required arrays inside
    // function declarations; confirm both surface through the sanitizer.
    let schema = json!({
        "type": "object",
        "properties": {
            "either": {
                "allOf": [
                    { "type": "string", "title": "should-be-removed" }
                ],
                "description": "either branch"
            }
        },
        "required": ["either"]
    });
    let sanitized = sanitize_schema_for_provider(&schema, AgentProviderAdapter::Gemini);

    assert!(sanitized.get("allOf").is_none());
    let either = &sanitized["properties"]["either"];
    assert!(
        either.get("allOf").is_none(),
        "allOf must be stripped inside properties"
    );
    assert_eq!(
        either.get("description").and_then(Value::as_str),
        Some("either branch")
    );
    // `required` lives on the root schema, so it must be preserved and trimmed.
    assert_eq!(
        sanitized
            .get("required")
            .and_then(Value::as_array)
            .map(|a| a.len()),
        Some(1)
    );
}

#[test]
fn claude_sanitize_strips_schema_metadata_but_preserves_user_property_keys() {
    // Claude strips `$schema` / `$id` from the top-level only — user field
    // names inside `properties` stay intact.
    let schema = json!({
        "type": "object",
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": "https://example.com/x.json",
        "properties": {
            "$schema": { "type": "string" },
            "city": { "type": "string" }
        }
    });
    let sanitized = sanitize_schema_for_provider(&schema, AgentProviderAdapter::ClaudeMessages);

    assert!(sanitized.get("$schema").is_none());
    assert!(sanitized.get("$id").is_none());
    assert!(sanitized["properties"].get("$schema").is_some());
}

#[test]
fn openai_sanitize_is_a_noop() {
    // OpenAI / OpenAI Responses declare an empty remove list. Sanitize must
    // return an equivalent structure.
    let schema = json!({
        "type": "object",
        "title": "Weather",
        "properties": { "city": { "type": "string" } }
    });
    for adapter in [
        AgentProviderAdapter::OpenAi,
        AgentProviderAdapter::OpenAiResponses,
    ] {
        let sanitized = sanitize_schema_for_provider(&schema, adapter);
        assert_eq!(sanitized, schema, "OpenAI sanitize must not mutate schema");
    }
}

#[test]
fn render_openai_tools_emits_expected_wire_shape() {
    // Wire shape: outer `{type:"function", function:{name, description, parameters}}`
    // and `parameters` carries the sanitized input schema.
    let tools = vec![descriptor()];
    let rendered = render_openai_tools(&tools, AgentProviderAdapter::OpenAi);
    assert_eq!(rendered.len(), 1);

    let entry = &rendered[0];
    assert_eq!(entry.get("type").and_then(Value::as_str), Some("function"));

    let function = entry
        .get("function")
        .and_then(Value::as_object)
        .expect("function");
    assert_eq!(
        function.get("name").and_then(Value::as_str),
        Some("get_weather")
    );
    assert_eq!(
        function.get("description").and_then(Value::as_str),
        Some("Read current weather for a city.")
    );

    let parameters = function
        .get("parameters")
        .and_then(Value::as_object)
        .expect("parameters");
    assert_eq!(
        parameters.get("type").and_then(Value::as_str),
        Some("object")
    );
    assert!(parameters.get("properties").is_some());
}

#[test]
fn resolve_request_adapter_accepts_canonical_and_alias_strings() {
    // `chat_completion_source` is the wire-facing source identifier; both the
    // canonical snake_case and the user-friendly alias must resolve to the same
    // adapter. Unknown sources must error.
    assert_eq!(
        resolve_request_adapter("openai").expect("openai"),
        AgentProviderAdapter::OpenAi
    );
    assert_eq!(
        resolve_request_adapter("openai_responses").expect("openai_responses"),
        AgentProviderAdapter::OpenAiResponses
    );
    assert_eq!(
        resolve_request_adapter("responses").expect("responses alias"),
        AgentProviderAdapter::OpenAiResponses
    );
    assert_eq!(
        resolve_request_adapter("claude").expect("claude"),
        AgentProviderAdapter::ClaudeMessages
    );
    assert_eq!(
        resolve_request_adapter("gemini").expect("gemini"),
        AgentProviderAdapter::Gemini
    );
    assert_eq!(
        resolve_request_adapter("gemini_interactions").expect("gemini_interactions"),
        AgentProviderAdapter::GeminiInteractions
    );
    assert_eq!(
        resolve_request_adapter("").expect("default"),
        AgentProviderAdapter::OpenAi
    );
    assert_eq!(
        resolve_request_adapter("  openai_responses  ").expect("trimmed"),
        AgentProviderAdapter::OpenAiResponses
    );

    let err = resolve_request_adapter("nope").expect_err("unknown source must error");
    assert!(
        format!("{err}").contains("nope"),
        "error message should echo the unknown source; got {err}"
    );
}

#[test]
fn claude_sanitize_recurses_into_all_schema_subschema_positions() {
    let schema = json!({
        "$id": "top",
        "$defs": { "Def": { "$id": "def", "type": "object" } },
        "patternProperties": { "^x": { "$schema": "pattern", "type": "string" } },
        "if": { "$id": "if", "type": "object" },
        "then": { "$schema": "then", "type": "object" },
        "else": { "$id": "else", "type": "object" },
        "prefixItems": [
            { "$id": "prefix", "type": "string" }
        ],
        "properties": {
            "title": { "type": "string" },
            "$id": { "type": "number" },
            "if": { "type": "boolean" }
        }
    });

    let sanitized = sanitize_schema_for_provider(&schema, AgentProviderAdapter::ClaudeMessages);
    assert!(sanitized.get("$id").is_none());
    assert!(sanitized["$defs"]["Def"].get("$id").is_none());
    assert!(
        sanitized["patternProperties"]["^x"]
            .get("$schema")
            .is_none()
    );
    assert!(sanitized["if"].get("$id").is_none());
    assert!(sanitized["then"].get("$schema").is_none());
    assert!(sanitized["else"].get("$id").is_none());
    assert!(sanitized["prefixItems"][0].get("$id").is_none());

    let properties = sanitized["properties"].as_object().expect("properties");
    assert!(properties.contains_key("title"));
    assert!(properties.contains_key("$id"));
    assert!(properties.contains_key("if"));
}
