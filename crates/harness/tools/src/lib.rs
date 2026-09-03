//! harness-tools — 工具契约。只声明不执行。
//!
//! 模块：
//! - [`spec`] — ToolSpec / ToolId / ToolCatalog / ToolChoice / ToolTurnContract
//! - [`result`] — AgentModelTool / AgentToolResult
//! - [`gate`] — ToolRequestGate 授权 + 预算计数
//! - [`format`] — OpenAI `tools[]` 渲染 + Gemini schema 规范化
//! - [`adapter`] — Provider 适配器解析

pub mod adapter;
pub mod format;
pub mod gate;
pub mod result;
pub mod spec;

pub use adapter::{resolve_request_adapter, AgentProviderAdapter};
pub use format::{render_openai_tools, sanitize_schema_for_provider};
pub use gate::{ToolRequestGate, ToolRequestGateError};
pub use result::{AgentModelTool, AgentToolResult};
pub use spec::{
    InvocationToolSnapshot, ToolBinding, ToolCatalog, ToolChoice, ToolDescriptor, ToolError,
    ToolId, ToolInvocation, ToolProviderId, ToolSnapshotId, ToolTurnContract,
};