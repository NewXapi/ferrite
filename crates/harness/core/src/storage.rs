//! 运行存储分类。
//!
//! 整文件搬运自 `tt-domain/src/models/agent/storage.rs`。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRunStorageClass {
    RunJournal,
    RunSummaryProjection,
    RunContext,
    RunWorkspaceProjection,
    RunToolIo,
    WorkspaceOutputs,
    WorkspaceScratch,
    Tasks,
    ModelResponses,
    Checkpoints,
    OtherRunArtifact,
}

impl AgentRunStorageClass {
    pub fn from_run_relative_path(relative_path: &str) -> Self {
        match relative_path {
            "run.json" | "events.jsonl" => return Self::RunJournal,
            "manifest.json" => return Self::RunContext,
            _ => {}
        }

        let component = relative_path
            .split_once('/')
            .map_or(relative_path, |(component, _)| component);

        match component {
            "input" | "invocations" => Self::RunContext,
            "persist" | "summaries" | "plan" => Self::RunWorkspaceProjection,
            "tool-args" | "tool-results" | "agent-results" => Self::RunToolIo,
            "output" => Self::WorkspaceOutputs,
            "scratch" => Self::WorkspaceScratch,
            "tasks" => Self::Tasks,
            "model-responses" => Self::ModelResponses,
            "checkpoints" => Self::Checkpoints,
            _ => Self::OtherRunArtifact,
        }
    }

    pub fn run_index() -> Self {
        Self::RunJournal
    }

    pub fn run_summary_projection() -> Self {
        Self::RunSummaryProjection
    }

    pub fn is_core_history(self) -> bool {
        matches!(self, Self::RunJournal | Self::RunSummaryProjection)
    }

    pub fn is_slim_artifact(self) -> bool {
        !self.is_core_history()
    }
}