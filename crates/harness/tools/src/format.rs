//! OpenAI `tools[]` 渲染 + Provider schema 规范化。
//!
//! 整文件照抄自 TauriTavern
//! `tt-application/src/services/agent_model_gateway/schema.rs`（115 行）。

use serde_json::{Value, json};

use crate::adapter::AgentProviderAdapter;
use crate::result::AgentModelTool;

pub fn render_openai_tools(tools: &[AgentModelTool], adapter: AgentProviderAdapter) -> Vec<Value> {
    tools
        .iter()
        .map(|tool| {
            let mut function = json!({
                "name": tool.model_alias.as_str(),
                "parameters": sanitize_schema_for_provider(&tool.input_schema, adapter),
            });
            if let Some(description) = &tool.description {
                function["description"] = Value::String(description.clone());
            }
            json!({
                "type": "function",
                "function": function,
            })
        })
        .collect()
}

pub fn sanitize_schema_for_provider(schema: &Value, adapter: AgentProviderAdapter) -> Value {
    let mut schema = schema.clone();
    remove_schema_keys(&mut schema, adapter.schema_keys_to_remove());
    if matches!(
        adapter,
        AgentProviderAdapter::Gemini | AgentProviderAdapter::GeminiInteractions
    ) {
        normalize_gemini_schema(&mut schema, 0);
    }
    schema
}

fn normalize_gemini_schema(value: &mut Value, depth: usize) {
    let Value::Object(object) = value else {
        return;
    };

    if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
        for nested in properties.values_mut() {
            normalize_gemini_schema(nested, depth + 1);
        }
    }
    if let Some(items) = object.get_mut("items") {
        normalize_gemini_schema(items, depth + 1);
    }

    if depth > 0 {
        // Gemini rejects nested required arrays in function declarations. Runtime
        // tool validators still return recoverable tool errors for missing fields.
        object.remove("required");
    } else {
        prune_required_to_declared_properties(object);
    }

    if depth > 0
        && object.get("type").and_then(Value::as_str) == Some("object")
        && object
            .get("properties")
            .and_then(Value::as_object)
            .is_none_or(|properties| properties.is_empty())
    {
        object.insert("type".to_string(), Value::String("string".to_string()));
        object.remove("properties");
        object.remove("items");
    }
}

fn prune_required_to_declared_properties(object: &mut serde_json::Map<String, Value>) {
    let Some(required) = object.get("required").and_then(Value::as_array) else {
        return;
    };
    let Some(properties) = object.get("properties").and_then(Value::as_object) else {
        object.remove("required");
        return;
    };

    let retained = required
        .iter()
        .filter_map(Value::as_str)
        .filter(|name| properties.contains_key(*name))
        .map(|name| Value::String(name.to_string()))
        .collect::<Vec<_>>();

    if retained.is_empty() {
        object.remove("required");
    } else {
        object.insert("required".to_string(), Value::Array(retained));
    }
}

fn remove_schema_keys(value: &mut Value, keys: &[&str]) {
    match value {
        Value::Object(object) => {
            for key in keys {
                object.remove(*key);
            }
            // Descend only into values that look like JSON Schema sub-schemas.
            // The `properties` map's keys are user-defined field names (e.g.
            // `title`, `$id`), not schema keywords — naively walking every
            // nested object would silently delete those user fields.
            if let Some(properties) = object.get_mut("properties").and_then(Value::as_object_mut) {
                for nested in properties.values_mut() {
                    remove_schema_keys(nested, keys);
                }
            }
            for schema_key in SINGLE_SCHEMA_KEYS {
                if let Some(child) = object.get_mut(*schema_key) {
                    remove_schema_keys(child, keys);
                }
            }
            for schema_key in ARRAY_SCHEMA_KEYS {
                if let Some(array) = object.get_mut(*schema_key).and_then(Value::as_array_mut) {
                    for item in array {
                        remove_schema_keys(item, keys);
                    }
                }
            }
            for schema_key in MAP_SCHEMA_KEYS {
                if let Some(map) = object.get_mut(*schema_key).and_then(Value::as_object_mut) {
                    for item in map.values_mut() {
                        remove_schema_keys(item, keys);
                    }
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                remove_schema_keys(item, keys);
            }
        }
        _ => {}
    }
}

const SINGLE_SCHEMA_KEYS: &[&str] = &[
    "items",
    "additionalItems",
    "unevaluatedItems",
    "additionalProperties",
    "unevaluatedProperties",
    "contains",
    "propertyNames",
    "contentSchema",
    "not",
    "if",
    "then",
    "else",
];
const ARRAY_SCHEMA_KEYS: &[&str] = &["allOf", "anyOf", "oneOf", "prefixItems"];
const MAP_SCHEMA_KEYS: &[&str] = &[
    "dependencies",
    "dependentSchemas",
    "patternProperties",
    "definitions",
    "$defs",
];
