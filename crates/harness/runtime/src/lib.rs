//! harness-runtime — Agent loop driver and tool execution. Backend only.

pub mod cancel;
pub mod delegation;
pub mod delta_agg;
pub mod event_sink;
pub mod loop_engine;
pub mod persistence;
pub mod provider;
pub mod tool_exec;
pub mod turn;

pub use cancel::{CancelReason, CancellationToken};
pub use delegation::{
    DelegationError, DelegationRequest, build_child_request, check_delegation, register_delegation,
    run_delegated_task, truncate_result,
};
pub use delta_agg::{AggregateError, DeltaAggregator, ToolAliasResolver, TurnAggregate};
pub use event_sink::{EventFactory, EventSink, MpscEventSink, VecEventSink};
pub use loop_engine::{
    AgentRunDeps, AgentRunRequest, LoopError, load_resumable_run, run_agent_run,
};
pub use persistence::{PersistenceError, RunPersistence};
pub use provider::{
    ChatProvider, ProviderDelta, ProviderError, ProviderFinishReason, ProviderRequest,
    ProviderStream, ProviderUsage, ToolCallFragment, empty_request,
};
pub use tool_exec::{ToolExecError, ToolExecutor, ToolHandler};
pub use turn::{TurnDriver, TurnError, TurnOutcome};
