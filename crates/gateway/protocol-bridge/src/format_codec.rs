//! 单格式双向 codec —— 两跳转换抽象（WP1）。
//!
//! 旧 [`adaptor::Codec`] 是 `(source, target)` 的**有向直转**：N 种格式互转需要
//! N×(N-1) 个 codec（ferrite 现状是每对方向各写一个 `ClaudeCodec`/`GeminiCodec`）。
//! 本模块改为**单格式** codec：每个格式只与本模块的 provider-neutral IR（[`ir`]）
//! 互转，注册表把 `src → dst` 拆成恒定两跳 `src.decode → IR → dst.encode`。
//! 加一个格式 = 实现一个 trait，复杂度从 N×M 降到 N+M。
//!
//! ## decode 无状态 / encode 有状态
//!
//! 逐事件解码（如 Claude SSE `input_json_delta` → OpenAI `tool_calls[].arguments`）
//! 是 1:1 映射，不需要跨事件状态；真正需要 per-stream 状态的是**编码侧**——
//! Claude/Responses 的输出要按 block 索引补 `content_block_start` / `output_item.added`
//! 边界帧、把 IR 的 `ToolCallDelta` 碎片累积成完整 JSON。故只有
//! [`FormatCodec::stream_encoder`] 返回的 [`StreamEncoder`] 持有 `&mut self` 状态，
//! [`FormatCodec::decode_event`] 保持无状态。
//!
//! 帧切分不在 codec 内：`decode_event` 只吃一个完整 SSE 帧的 data 负载，
//! 跨 chunk 行重组由 pipeline 层的共享扫描器（见 [`sse`]）负责。
//!
//! WP1 只落地抽象与注册表；内置 codec 由 WP2-WP5 实现，旧 [`adaptor::Codec`]
//! 路径保持原样接线（迁移策略 A′）。
//!
//! [`adaptor::Codec`]: crate::adaptor::Codec
//! [`ir`]: crate::ir
//! [`sse`]: crate::sse

use bytes::Bytes;
use std::collections::HashMap;
use std::sync::Arc;

use crate::adaptor::{AdaptorError, Protocol};
use crate::ir::{LlmRequest, LlmResponse, StreamEvent};

/// 单格式 codec：本格式 ↔ IR 双向。加一个格式 = 实现这一个 trait。
///
/// 所有方法都以 IR 为另一端：`decode_*` 把本格式字节解析成 IR，`encode_*` 把 IR
/// 打成本格式字节。同一格式只实现这一个 trait，请求/响应双向都覆盖，不再有
/// 「请求 codec / 响应 codec」两套（旧架构把方向拆进 `(source, target)` pair，
/// 拿请求方向的 codec 转响应是 G1 记载的 502 根源）。
pub trait FormatCodec: Send + Sync {
    /// 本 codec 处理的协议格式。
    fn format(&self) -> Protocol;

    /// 本格式请求体 → IR。
    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError>;
    /// IR → 本格式请求体。
    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError>;

    /// 本格式非流式响应体 → IR。
    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError>;
    /// IR → 本格式非流式响应体。
    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError>;

    /// 一个完整 SSE 帧的 data 负载 → IR 事件（无状态：逐事件 1:1 映射）。
    ///
    /// 输入是**单帧**负载：调用方负责跨 chunk 的行重组（见模块级文档）。
    fn decode_event(&self, data: &str) -> Result<Vec<StreamEvent>, AdaptorError>;

    /// 创建本格式的流编码器（有状态：block 边界帧、tool 碎片累积）。
    ///
    /// 每条流调一次；返回的 [`StreamEncoder`] 在流的整个生命周期内 `&mut self`
    /// 持有状态，跨流之间不共享。
    fn stream_encoder(&self) -> Box<dyn StreamEncoder>;
}

/// 每流一个的流编码器：`&mut self` 持有流状态（block 索引 / tool 碎片累积）。
///
/// 生命周期：`encode_event` 被逐事件调用，`finish` 在流结束时调用一次（补
/// block 结束/收尾帧）。同一流的两次调用共享状态；两条流必须各拿一个
/// `stream_encoder()`，状态不串。
pub trait StreamEncoder: Send {
    /// 一个 IR 事件 → 本格式的若干 SSE 帧字节（可能零帧：状态机暂存时）。
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError>;
    /// 流结束：吐出暂存的收尾帧。
    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError>;
}

/// 单格式注册表：`Protocol → Arc<dyn FormatCodec>`（不再有 pair 表）。
///
/// [`Self::translate_request`] / [`Self::translate_response`] 把 `src → dst`
/// 封装成恒定两跳 `src.decode → IR → dst.encode`，调用方不见 decode/encode 拼装。
pub struct FormatRegistry {
    codecs: HashMap<Protocol, Arc<dyn FormatCodec>>,
}

impl FormatRegistry {
    /// 空注册表。内置 codec 由 WP2-WP5 的 `with_defaults()` 装载。
    pub fn new() -> Self {
        Self {
            codecs: HashMap::new(),
        }
    }

    /// 注册一个单格式 codec；同 `format()` 的旧注册被覆盖。
    pub fn register(&mut self, codec: Arc<dyn FormatCodec>) {
        self.codecs.insert(codec.format(), codec);
    }

    /// 查某格式的 codec；None = 该格式未注册。
    pub fn resolve(&self, fmt: Protocol) -> Option<Arc<dyn FormatCodec>> {
        self.codecs.get(&fmt).cloned()
    }

    /// 两跳：src 格式请求体 → IR → dst 格式请求体。`src == dst` 或任一侧是
    /// [`Protocol::Passthrough`] 时**零转换直通**（不解析字节，原样返回）。
    pub fn translate_request(
        &self,
        src: Protocol,
        dst: Protocol,
        body: Bytes,
    ) -> Result<Bytes, AdaptorError> {
        if self.is_passthrough(src, dst) {
            return Ok(body);
        }
        let src_codec = self
            .resolve(src)
            .ok_or(AdaptorError::NotRegistered { from: src, to: dst })?;
        let dst_codec = self
            .resolve(dst)
            .ok_or(AdaptorError::NotRegistered { from: src, to: dst })?;
        let ir = src_codec.decode_request(body)?;
        dst_codec.encode_request(&ir)
    }

    /// 两跳：src 格式响应体 → IR → dst 格式响应体。直通规则同
    /// [`Self::translate_request`]。
    pub fn translate_response(
        &self,
        src: Protocol,
        dst: Protocol,
        body: Bytes,
    ) -> Result<Bytes, AdaptorError> {
        if self.is_passthrough(src, dst) {
            return Ok(body);
        }
        let src_codec = self
            .resolve(src)
            .ok_or(AdaptorError::NotRegistered { from: src, to: dst })?;
        let dst_codec = self
            .resolve(dst)
            .ok_or(AdaptorError::NotRegistered { from: src, to: dst })?;
        let ir = src_codec.decode_response(body)?;
        dst_codec.encode_response(&ir)
    }

    /// 零转换直通条件：同格式，或任一侧显式声明透传。
    fn is_passthrough(&self, src: Protocol, dst: Protocol) -> bool {
        src == dst || src == Protocol::Passthrough || dst == Protocol::Passthrough
    }
}

impl Default for FormatRegistry {
    fn default() -> Self {
        Self::new()
    }
}
