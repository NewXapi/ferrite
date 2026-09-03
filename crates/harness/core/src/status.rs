//! 运行状态枚举。

use serde::{Deserialize, Serialize};

pub const ROOT_AGENT_INVOCATION_ID: &str = "inv_root";
pub const AGENT_RUN_SUMMARY_PROJECTION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Created,
    InitializingWorkspace,
    AssemblingContext,
    CallingModel,
    DispatchingTool,
    ApplyingWorkspacePatch,
    CreatingCheckpoint,
    AwaitingHostCommit,
    Finishing,
    Completed,
    PartialSuccess,
    Cancelling,
    Cancelled,
    Failed,
}

impl AgentRunStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::PartialSuccess | Self::Cancelled | Self::Failed
        )
    }
}