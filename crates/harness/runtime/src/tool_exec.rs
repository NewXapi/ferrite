//! Tool execution: gate, schema subset, handler dispatch.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use serde_json::Value;
use thiserror::Error;

use harness_tools::{
    AgentToolResult, InvocationToolSnapshot, ToolId, ToolInvocation, ToolRequestGate,
    ToolRequestGateError, ToolTurnContract,
};

/// Async tool handler.
pub type ToolHandler = Arc<
    dyn Fn(ToolInvocation) -> Pin<Box<dyn Future<Output = AgentToolResult> + Send>> + Send + Sync,
>;

/// Tool executor errors that abort the loop rather than becoming a tool result.
#[derive(Debug, Error)]
pub enum ToolExecError {
    #[error(transparent)]
    Gate(#[from] ToolRequestGateError),
}

/// Dispatches authorized tool calls to registered handlers.
#[derive(Default)]
pub struct ToolExecutor {
    handlers: HashMap<ToolId, ToolHandler>,
    gate: ToolRequestGate,
    completed: HashMap<(String, ToolId), AgentToolResult>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool_id: ToolId, handler: ToolHandler) {
        self.handlers.insert(tool_id, handler);
    }

    /// Start a fresh run: budgets and replay results never cross run boundaries.
    pub fn begin_run(&mut self) {
        self.gate = ToolRequestGate::default();
        self.completed.clear();
    }

    pub async fn execute(
        &mut self,
        snapshot: &InvocationToolSnapshot,
        turn: &ToolTurnContract,
        invocation: ToolInvocation,
    ) -> Result<AgentToolResult, ToolExecError> {
        self.gate
            .authorize_and_reserve(snapshot, turn, &invocation)?;
        let replay_key = (invocation.call_id.clone(), invocation.tool_id.clone());
        if let Some(result) = self.completed.get(&replay_key) {
            return Ok(result.clone());
        }

        if let Some(schema) = snapshot
            .binding(&invocation.tool_id)
            .map(|binding| binding.descriptor().input_schema.clone())
            && let Err(message) = validate_object_schema(&schema, &invocation.arguments)
        {
            let result = error_result(invocation, "schema_invalid", message);
            self.completed.insert(replay_key, result.clone());
            return Ok(result);
        }

        let Some(handler) = self.handlers.get(&invocation.tool_id) else {
            let result = error_result(
                invocation,
                "unregistered_tool",
                "no handler registered for tool",
            );
            self.completed.insert(replay_key, result.clone());
            return Ok(result);
        };

        let result = handler(invocation).await;
        self.completed.insert(replay_key, result.clone());
        Ok(result)
    }
}

fn error_result(
    invocation: ToolInvocation,
    code: &str,
    message: impl Into<String>,
) -> AgentToolResult {
    AgentToolResult {
        call_id: invocation.call_id,
        tool_id: invocation.tool_id,
        content: message.into(),
        structured: Value::Null,
        is_error: true,
        error_code: Some(code.to_string()),
        resource_refs: Vec::new(),
    }
}

/// Minimal JSON Schema subset: object + required + properties.type.
fn validate_object_schema(schema: &Value, arguments: &Value) -> Result<(), String> {
    let Some(object_schema) = schema.as_object() else {
        return Ok(());
    };
    if object_schema.get("type").and_then(Value::as_str) != Some("object") {
        return Ok(());
    }
    let Some(args) = arguments.as_object() else {
        return Err("arguments must be a JSON object".to_string());
    };
    if let Some(required) = object_schema.get("required").and_then(Value::as_array) {
        for key in required.iter().filter_map(Value::as_str) {
            if !args.contains_key(key) {
                return Err(format!("missing required property `{key}`"));
            }
        }
    }
    if let Some(properties) = object_schema.get("properties").and_then(Value::as_object) {
        for (key, property) in properties {
            let Some(value) = args.get(key) else {
                continue;
            };
            if let Some(expected) = property.get("type").and_then(Value::as_str)
                && !value_matches_type(value, expected)
            {
                return Err(format!("property `{key}` must be {expected}"));
            }
        }
    }
    Ok(())
}

fn value_matches_type(value: &Value, expected: &str) -> bool {
    match expected {
        "object" => value.is_object(),
        "array" => value.is_array(),
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "null" => value.is_null(),
        _ => true,
    }
}
