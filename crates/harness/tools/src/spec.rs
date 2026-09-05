//! Tool contract types — declarative only, no runtime execution.
//!
//! 整文件照抄自 TauriTavern `tt-domain/src/models/tool.rs`，并把 `DomainError`
//! 替换为本 crate 内部 `ToolError`。Ferrite 端 `harness-core` 当前为空壳，
//! 我们在这里自给自足。

use std::{
    collections::{BTreeMap, HashSet, btree_map::Entry},
    fmt,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;
use thiserror::Error;

const TOOL_ID_SEPARATOR: char = ':';
const BUILTIN_TOOL_PROVIDER_ID: &str = "builtin";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ToolError {
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Conflict: {0}")]
    Conflict(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolProviderId(String);

impl ToolProviderId {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ToolError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(ToolError::InvalidData(
                "tool.provider_id_empty: tool provider id cannot be empty".to_string(),
            ));
        }
        if raw.contains(TOOL_ID_SEPARATOR) {
            return Err(ToolError::InvalidData(format!(
                "tool.provider_id_invalid: tool provider id `{raw}` cannot contain `{TOOL_ID_SEPARATOR}`"
            )));
        }

        Ok(Self(raw))
    }

    pub fn builtin() -> Self {
        Self(BUILTIN_TOOL_PROVIDER_ID.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolProviderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ToolProviderId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ToolProviderId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolId(String);

impl ToolId {
    pub const SEPARATOR: char = TOOL_ID_SEPARATOR;
    pub const BUILTIN_PROVIDER: &'static str = BUILTIN_TOOL_PROVIDER_ID;

    pub fn new(
        provider_id: &ToolProviderId,
        native_name: impl AsRef<str>,
    ) -> Result<Self, ToolError> {
        let native_name = native_name.as_ref();
        if native_name.is_empty() {
            return Err(ToolError::InvalidData(
                "tool.native_name_empty: tool native name cannot be empty".to_string(),
            ));
        }
        if native_name.contains(TOOL_ID_SEPARATOR) {
            return Err(ToolError::InvalidData(format!(
                "tool.native_name_invalid: tool native name `{native_name}` must not contain `{TOOL_ID_SEPARATOR}`"
            )));
        }

        Ok(Self(format!(
            "{}{TOOL_ID_SEPARATOR}{native_name}",
            provider_id.as_str()
        )))
    }

    pub fn parse(raw: impl Into<String>) -> Result<Self, ToolError> {
        let raw = raw.into();
        let (provider_id, native_name) = raw.split_once(TOOL_ID_SEPARATOR).ok_or_else(|| {
            ToolError::InvalidData(format!(
                "tool.id_invalid: tool id `{raw}` must contain a provider and native name"
            ))
        })?;
        let provider_id = ToolProviderId::parse(provider_id.to_string())?;
        Self::new(&provider_id, native_name)
    }

    pub fn builtin(native_name: impl AsRef<str>) -> Result<Self, ToolError> {
        Self::new(&ToolProviderId::builtin(), native_name)
    }

    pub fn provider_id(&self) -> &str {
        self.0
            .split_once(TOOL_ID_SEPARATOR)
            .expect("ToolId constructor guarantees a separator")
            .0
    }

    pub fn native_name(&self) -> &str {
        self.0
            .split_once(TOOL_ID_SEPARATOR)
            .expect("ToolId constructor guarantees a separator")
            .1
    }

    pub fn is_builtin(&self) -> bool {
        self.provider_id() == BUILTIN_TOOL_PROVIDER_ID
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for ToolId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for ToolId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::parse(String::deserialize(deserializer)?).map_err(de::Error::custom)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub id: ToolId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// 注解；开放 JSON 对象。约定键 `stealth: true` 表示该工具结果不回灌聊天流
    /// （对齐 SillyTavern ToolDefinition.stealth，严格布尔 true 才生效）。
    pub annotations: Value,
}

impl ToolDescriptor {
    /// 该工具是否为 stealth（`annotations.stealth` 严格等于布尔 `true`）。
    pub fn is_stealth(&self) -> bool {
        self.annotations.get("stealth") == Some(&Value::Bool(true))
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Hash)]
pub struct ToolSnapshotId(String);

impl ToolSnapshotId {
    pub fn parse(raw: impl Into<String>) -> Result<Self, ToolError> {
        let raw = raw.into();
        if raw.is_empty() {
            return Err(ToolError::InvalidData(
                "tool.snapshot_id_empty: tool snapshot id cannot be empty".to_string(),
            ));
        }
        Ok(Self(raw))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolSnapshotId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolBinding {
    descriptor: ToolDescriptor,
    model_alias: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_calls: Option<usize>,
}

impl ToolBinding {
    pub fn new(
        descriptor: ToolDescriptor,
        model_alias: impl Into<String>,
        max_calls: Option<usize>,
    ) -> Result<Self, ToolError> {
        let model_alias = model_alias.into();
        if model_alias.is_empty() {
            return Err(ToolError::InvalidData(format!(
                "tool.snapshot_alias_empty: tool `{}` has an empty model alias",
                descriptor.id
            )));
        }
        // OpenAI function names allow 1–64 chars from `[A-Za-z0-9_-]`. Anything
        // else (spaces, colons, unicode, …) must be rejected up front so the
        // wire payload never carries an invalid identifier.
        if model_alias.len() > 64
            || !model_alias
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
        {
            return Err(ToolError::InvalidData(format!(
                "tool.snapshot_alias_invalid: tool `{}` model alias `{model_alias}` must match ^[A-Za-z0-9_-]{{1,64}}$",
                descriptor.id
            )));
        }
        if max_calls == Some(0) {
            return Err(ToolError::InvalidData(format!(
                "tool.snapshot_tool_budget_invalid: tool `{}` max calls must be greater than zero",
                descriptor.id
            )));
        }
        Ok(Self {
            descriptor,
            model_alias,
            max_calls,
        })
    }

    pub fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    pub fn tool_id(&self) -> &ToolId {
        &self.descriptor.id
    }

    pub fn model_alias(&self) -> &str {
        &self.model_alias
    }

    pub fn max_calls(&self) -> Option<usize> {
        self.max_calls
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InvocationToolSnapshot {
    schema_version: u32,
    id: ToolSnapshotId,
    bindings: Vec<ToolBinding>,
    max_calls_per_invocation: usize,
}

impl InvocationToolSnapshot {
    pub const SCHEMA_VERSION: u32 = 1;

    pub fn try_new(
        id: ToolSnapshotId,
        bindings: Vec<ToolBinding>,
        max_calls_per_invocation: usize,
    ) -> Result<Self, ToolError> {
        if max_calls_per_invocation == 0 {
            return Err(ToolError::InvalidData(
                "tool.snapshot_budget_invalid: max calls per invocation must be greater than zero"
                    .to_string(),
            ));
        }

        let mut tool_ids = HashSet::with_capacity(bindings.len());
        let mut aliases = HashSet::with_capacity(bindings.len());
        for binding in &bindings {
            if !tool_ids.insert(binding.tool_id().clone()) {
                return Err(ToolError::Conflict(format!(
                    "tool.snapshot_duplicate_id: duplicate tool id `{}`",
                    binding.tool_id()
                )));
            }
            if !aliases.insert(binding.model_alias().to_string()) {
                return Err(ToolError::Conflict(format!(
                    "tool.snapshot_duplicate_alias: duplicate model alias `{}`",
                    binding.model_alias()
                )));
            }
        }

        Ok(Self {
            schema_version: Self::SCHEMA_VERSION,
            id,
            bindings,
            max_calls_per_invocation,
        })
    }

    pub fn id(&self) -> &ToolSnapshotId {
        &self.id
    }

    pub fn bindings(&self) -> &[ToolBinding] {
        &self.bindings
    }

    pub fn max_calls_per_invocation(&self) -> usize {
        self.max_calls_per_invocation
    }

    pub fn binding(&self, tool_id: &ToolId) -> Option<&ToolBinding> {
        // ponytail: invocation tool surfaces are small; add a derived index only if profiling
        // shows linear lookup matters.
        self.bindings
            .iter()
            .find(|binding| binding.tool_id() == tool_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCatalog {
    descriptors: BTreeMap<ToolId, ToolDescriptor>,
}

impl ToolCatalog {
    pub fn try_from_descriptors(
        descriptors: impl IntoIterator<Item = ToolDescriptor>,
    ) -> Result<Self, ToolError> {
        let mut catalog = BTreeMap::new();
        for descriptor in descriptors {
            match catalog.entry(descriptor.id.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(descriptor);
                }
                Entry::Occupied(entry) => {
                    return Err(ToolError::Conflict(format!(
                        "tool.catalog_duplicate_id: duplicate tool id `{}`",
                        entry.key()
                    )));
                }
            }
        }

        Ok(Self {
            descriptors: catalog,
        })
    }

    pub fn get(&self, id: &ToolId) -> Option<&ToolDescriptor> {
        self.descriptors.get(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ToolDescriptor> {
        self.descriptors.values()
    }

    pub fn len(&self) -> usize {
        self.descriptors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.descriptors.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Specific(ToolId),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolInvocation {
    pub call_id: String,
    pub tool_id: ToolId,
    pub arguments: Value,
    #[serde(default)]
    pub provider_metadata: Value,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ToolTurnContract {
    snapshot_id: ToolSnapshotId,
    tools: Vec<ToolId>,
    choice: ToolChoice,
}

impl ToolTurnContract {
    pub fn all(snapshot: &InvocationToolSnapshot, choice: ToolChoice) -> Result<Self, ToolError> {
        let tools = snapshot
            .bindings()
            .iter()
            .map(|binding| binding.tool_id().clone())
            .collect::<Vec<_>>();

        if matches!(choice, ToolChoice::Required) && tools.is_empty() {
            return Err(ToolError::InvalidData(
                "tool.turn_required_empty: required tool choice needs at least one tool"
                    .to_string(),
            ));
        }
        if let ToolChoice::Specific(tool_id) = &choice
            && snapshot.binding(tool_id).is_none()
        {
            return Err(ToolError::InvalidData(format!(
                "tool.turn_specific_not_available: specific tool `{tool_id}` is not available in snapshot `{}`",
                snapshot.id()
            )));
        }

        Ok(Self {
            snapshot_id: snapshot.id().clone(),
            tools,
            choice,
        })
    }

    pub fn snapshot_id(&self) -> &ToolSnapshotId {
        &self.snapshot_id
    }

    pub fn tools(&self) -> &[ToolId] {
        &self.tools
    }

    pub fn choice(&self) -> &ToolChoice {
        &self.choice
    }
}
