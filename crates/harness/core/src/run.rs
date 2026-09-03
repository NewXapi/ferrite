//! Agent Run 主体结构 + 调用 / 任务 / 工件 / Summary 投影。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::status::AGENT_RUN_SUMMARY_PROJECTION_SCHEMA_VERSION;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSkillScopeRefs {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<super::profile::AgentPresetRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub character_id: Option<String>,
}

impl AgentRunSkillScopeRefs {
    pub fn is_empty(&self) -> bool {
        self.preset.is_none() && self.character_id.is_none()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunPresentation {
    Foreground,
    Background,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentChatCommitMode {
    Replace,
    Append,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceFileWriteMode {
    Replace,
    Append,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRun {
    pub id: String,
    pub workspace_id: String,
    pub stable_chat_id: String,
    pub chat_ref: AgentChatRef,
    pub generation_type: String,
    pub profile_id: Option<String>,
    #[serde(default, skip_serializing_if = "AgentRunSkillScopeRefs::is_empty")]
    pub skill_scope_refs: AgentRunSkillScopeRefs,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persist_base_state_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_message_count: Option<usize>,
    pub presentation: AgentRunPresentation,
    pub status: crate::status::AgentRunStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind",
    deny_unknown_fields
)]
pub enum AgentChatRef {
    Character {
        character_id: String,
        file_name: String,
    },
    Group {
        chat_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunSummaryProjection {
    pub schema_version: u32,
    pub run_id: String,
    pub source_run_updated_at: DateTime<Utc>,
    pub commit_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_message: Option<AgentRunCommittedMessageProjection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_at: Option<DateTime<Utc>>,
}

impl Default for AgentRunSummaryProjection {
    fn default() -> Self {
        Self {
            schema_version: AGENT_RUN_SUMMARY_PROJECTION_SCHEMA_VERSION,
            run_id: String::new(),
            source_run_updated_at: DateTime::<Utc>::from_timestamp(0, 0).expect("unix epoch"),
            commit_count: 0,
            committed_message: None,
            terminal_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentRunCommittedMessageProjection {
    pub commit_id: String,
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_index: Option<usize>,
    pub committed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationKind {
    Root,
    Subagent,
    Handoff,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationStatus {
    Created,
    Running,
    Completed,
    Failed,
    Cancelled,
    Transferred,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentInvocationExitPolicy {
    RunFinishAllowed,
    TaskReturnRequired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentDelegationContinuation {
    ReturnToParent,
    TransferControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentInvocation {
    pub id: String,
    pub run_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_invocation_id: Option<String>,
    pub profile_id: String,
    pub kind: AgentInvocationKind,
    pub status: AgentInvocationStatus,
    pub exit_policy: AgentInvocationExitPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentTaskStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskRecord {
    pub id: String,
    pub run_id: String,
    pub parent_invocation_id: String,
    pub child_invocation_id: String,
    pub target_profile_id: String,
    pub workspace_key: String,
    pub continuation: AgentDelegationContinuation,
    pub status: AgentTaskStatus,
    #[serde(default)]
    pub task: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Work-Product 工件规格 — ResolvedAgentOutputPolicy 唯一用户（profile.rs）。
// 整段搬运自 tt-domain/src/models/agent/mod.rs:475-495。

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtifactSpec {
    pub id: String,
    pub path: String,
    pub kind: String,
    pub target: ArtifactTarget,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub assembly_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactTarget {
    MessageBody,
    MessageExtra { key: String },
    CombinedMarkdown,
    HiddenRunArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitPolicy {
    pub default_target: ArtifactTarget,
    #[serde(default)]
    pub combine_template: Option<String>,
    #[serde(default)]
    pub store_artifacts_in_extra: bool,
}
