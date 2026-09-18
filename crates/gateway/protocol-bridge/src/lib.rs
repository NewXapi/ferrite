//! `gateway-protocol-bridge` —— 数据面协议适配层
//!
//! 厂商协议兼容的集中地。对标 new-api `relay/channel/*/adaptor.go`。
//!
//! ## 架构：单格式双向 codec + 恒两跳
//!
//! 每个协议实现**一个** [`FormatCodec`]（本格式 ↔ IR 双向），格式互转恒定两跳
//! `src.decode → IR → dst.encode`。N 种格式只需 N 个 codec，而非 N×(N-1) 个
//! 有向转换器；加一个格式只写一个文件。
//!
//! 流式响应由每条流一个的 [`StreamEncoder`] 编码（`&mut self` 持有 block 边界与
//! tool 碎片累积状态），解码侧 [`FormatCodec::decode_event`] 保持无状态——
//! Claude 的 `input_json_delta` 到 OpenAI 的 `tool_calls[].arguments` 是逐事件
//! 1:1 映射，真正需要跨事件状态的是编码侧。
//!
//! ## 职责
//!
//! - [`ir`] — provider-neutral 中间表示（请求 / 响应 / 流事件），带未知字段逃逸口
//! - [`format_codec`] — [`FormatCodec`] / [`StreamEncoder`] trait + [`FormatRegistry`] 两跳调度
//! - [`openai`] / [`claude`] / [`gemini`] / [`responses`] — 四个格式的单格式 codec
//! - [`adaptor`] — 共享词汇：[`Protocol`] 格式标识与 [`AdaptorError`]
//! - [`sse`] — SSE 帧扫描（逐字保真透传 + 事件边界 + 供转换消费的完整帧）
//! - [`error_mapping`] — `contract::error::NormalizedError` → 各协议错误形状
//! - [`stage`] — pipeline Stage 4：错误映射 + 把 forward 备好的响应打包成 HTTP 响应
//!
//! ## 与其他 crate 的边界
//!
//! | crate | 角色 |
//! |-------|------|
//! | `gateway-forward` | IO 编排 + 双向转换闭环（请求与响应方向必须协作同一份 codec） |
//! | `gateway-protocol-bridge` | 转换规则本身（纯函数，无 IO） |
//! | `contract::error::NormalizedError` | 跨 crate 单一错误协议 |

pub mod adaptor;
pub mod claude;
pub mod error_mapping;
pub mod format_codec;
pub mod gemini;
pub mod ir;
pub mod openai;
pub mod responses;
pub mod sse;
pub mod stage;

pub use adaptor::{AdaptorError, Protocol};
pub use error_mapping::map_error;
pub use format_codec::{FormatCodec, FormatRegistry, StreamEncoder};
pub use ir::{
    ContentBlock, LlmRequest, LlmResponse, Message, Role, SamplingParams, StopReason, StreamEvent,
    TextBlock, ToolChoice, ToolDef, Usage,
};
pub use stage::ProtocolBridgeStage;
