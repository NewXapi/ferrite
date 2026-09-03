//! harness-core — Agent 回合状态机。零 runtime 依赖。
//!
//! 模块：
//! - [`status`] — 运行状态枚举 + 终态判定
//! - [`run`] — Agent Run 主体结构 + 调用 / 任务 / 工件 / Summary
//! - [`event`] — 运行时事件级别 + 事件结构
//! - [`workspace_path`] — Workspace 安全路径校验
//! - [`profile`] — Agent Profile + 策略 + 默认值
//! - [`plan`] — Plan 策略（none / free / strict / hybrid）
//! - [`storage`] — 运行存储分类
//! - [`profile_diagnostic`] — Profile 诊断结构
//!
//! 见 README.md。

pub mod event;
pub mod plan;
pub mod profile;
pub mod profile_diagnostic;
pub mod run;
pub mod status;
pub mod storage;
pub mod workspace_path;

// 便捷 re-export — 常用类型可在根路径直接拿到。

pub use event::{AgentRunEvent, AgentRunEventLevel};
pub use plan::{AgentPlanMode, AgentPlanNodePolicy, AgentPlanPolicy, DEFAULT_AGENT_PLAN_BETA};
pub use profile::{
    AGENT_PROFILE_KIND, AGENT_PROFILE_SCHEMA_VERSION, AgentContextPolicy, AgentDelegationPolicy,
    AgentModelBinding, AgentModelBindingMode, AgentModelRetryPolicy, AgentOutputArtifact,
    AgentOutputArtifactTarget, AgentOutputPolicy, AgentPresetBinding, AgentPresetBindingMode,
    AgentPresetRef, AgentProfileDefinition, AgentProfileId, AgentProfileInstructions,
    AgentProfileSourceTrace, AgentProfileSummary, AgentRunPolicy, AgentSkillPolicy,
    AgentToolDescriptionOverride, AgentToolPolicy, AgentWorkspacePolicy,
    DEFAULT_AGENT_DELEGATION_MAX_CONCURRENT_INVOCATIONS,
    DEFAULT_AGENT_DELEGATION_MAX_INVOCATIONS_PER_RUN,
    DEFAULT_AGENT_DELEGATION_RESULT_BUDGET_TOKENS, DEFAULT_AGENT_HANDOFF_MAX_DEPTH,
    DEFAULT_AGENT_INITIAL_CHAT_HISTORY_MESSAGES, DEFAULT_AGENT_MODEL_MAX_RETRIES,
    DEFAULT_AGENT_MODEL_RETRY_INTERVAL_MS, DEFAULT_AGENT_PROFILE_ID,
    DEFAULT_AGENT_SKILL_MAX_READ_CHARS_PER_CALL, DEFAULT_AGENT_SKILL_MAX_READ_CHARS_PER_RUN,
    DEFAULT_AGENT_TOOL_MAX_CALLS_PER_RUN, DEFAULT_AGENT_TOOL_MAX_ROUNDS, ResolvedAgentOutputPolicy,
    ResolvedAgentProfile,
};
pub use profile_diagnostic::{
    AgentProfileDiagnostic, AgentProfileDiagnosticBlock, AgentProfileDiagnosticRepairAction,
    AgentProfileDiagnosticResource, AgentProfileDiagnosticResourceKind,
    AgentProfileDiagnosticSeverity, AgentProfileHealth,
};
pub use run::{
    AgentChatCommitMode, AgentChatRef, AgentDelegationContinuation, AgentInvocation,
    AgentInvocationExitPolicy, AgentInvocationKind, AgentInvocationStatus, AgentRun,
    AgentRunCommittedMessageProjection, AgentRunPresentation, AgentRunSkillScopeRefs,
    AgentRunSummaryProjection, AgentTaskRecord, AgentTaskStatus, ArtifactSpec, ArtifactTarget,
    CommitPolicy, WorkspaceFileWriteMode,
};
pub use status::{
    AGENT_RUN_SUMMARY_PROJECTION_SCHEMA_VERSION, AgentRunStatus, ROOT_AGENT_INVOCATION_ID,
};
pub use storage::AgentRunStorageClass;
pub use workspace_path::{WorkspacePath, WorkspacePathError};
