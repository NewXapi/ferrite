//! P2 流式韧性 — 缓冲窗口 / 心跳 / 断流分类（骨架，PR #158）。
//!
//! 对标 api-hub `_smart_stream`（:618-723）/`_drain_buffered`（:556-615），并采纳
//! Bifrost 的 TTFT 监控纪律：**已向客户端外发过字节（Live 相）就不做破坏性重启**，
//! 只能刷缓冲 + `event: error` 帧收尾；首字节之前（Window 相）断流则整条换候选
//! 重试，客户端无感（= api-hub buffer_stream，flo2 buffer-before-forward 同款）。
//!
//! 参数健康化（todo/gateway-resilience.md P2）：窗口 2s 而非 api-hub 的 60s、
//! 重试 max 3 而非 100、心跳 15s（行业 10–15s）。
//!
//! ## 挂点说明
//!
//! ferrite 的 [`stream::SseContext`](crate::stream::SseContext) 扫描链（token 计数
//! 已在管道内）比 api-hub 的字节级 hack 更干净；本模块只做**缓冲 / 心跳 / 断流
//! 分类**，token 计数仍走原管道（[`crate::stream::pipe_chunk`]），不重复建设。
//! 消费点在 `stage.rs::commit_forwarded` 的 unfold 流（P2-A 落地时接线）。

use bytes::Bytes;

use crate::stream::SseContext;

/// 韧性管道参数。apps 从 config `[stream]` 段注入（P2-A），默认即健康值。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StreamResilience {
    /// 首字节前的透明重试窗口时长（ms）。窗口内 chunk 只攒不发，断流可整条换候选。
    pub window_ms: u64,
    /// Live 相上游静默阈值（ms）：超过即向客户端发 `: keepalive\n\n` SSE 注释帧保活。
    pub heartbeat_ms: u64,
    /// 窗口相断流重试的换候选上限（防 api-hub max=100 式暴力默认）。
    pub max_retries: u32,
}

impl Default for StreamResilience {
    fn default() -> Self {
        // 窗口 2s（非 60s）、心跳 15s（非 10s 下限）、重试 max 3（非 100）。
        Self {
            window_ms: 2_000,
            heartbeat_ms: 15_000,
            max_retries: 3,
        }
    }
}

/// 流所处阶段，决定断流语义（Bifrost：外发过字节就不能破坏性重启）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamPhase {
    /// 首字节前透明重试窗口：零字节已发，断流可整条换候选重试。
    Window,
    /// 已向客户端外发字节：只能刷缓冲 + error 帧收尾，绝不重启流。
    Live,
}

/// 韧性层消化一个上游 chunk（或 EOF）后，上层（`commit_forwarded`）应执行的动作。
#[derive(Debug, Clone)]
pub enum ResilientChunk {
    /// 待外发的净 SSE 字节（Live 相过阈值刷出，或 Window 相转 Live 时排空）。
    Emit(Bytes),
    /// 窗口内继续攒进 buf，暂不外发。
    Buffered,
    /// 窗口内断流（EOF 且无完整判据、零字节已发）：上层整条换候选重试。
    WindowEofRetryable,
    /// Live 相断流：已缓冲字节刷给客户端后以 `event: error` 帧收尾。
    /// `flushed` = 已刷出的字节数（结算与日志用）。
    Broken { flushed: usize },
}

/// 韧性管道的单 chunk 决策步。
///
/// 语义（P2-A，对标 api-hub `_smart_stream`/`_drain_buffered`）：
///
/// - **Window 相**：chunk 累积进 `buf` 不外发；`chunk = None`（EOF）时校验完整性
///   —— 以 `[DONE]` 或 `finish_reason` 二选一收尾（判据照抄 `_drain_buffered`）则
///   排空缓冲整体外发，否则返回 [`ResilientChunk::WindowEofRetryable`]；
/// - **Live 相**：按 32KB / 150ms 阈值从 `buf` 刷出发 [`ResilientChunk::Emit`]；
///   上游静默超 `cfg.heartbeat_ms` 时补 `: keepalive\n\n` 注释帧；EOF 无完整判据
///   返回 [`ResilientChunk::Broken`]；
/// - `ctx` 始终同步喂给原扫描链（token 计数不受缓冲影响）。
///
/// 窗口到期（`window_ms`）由调用方在时间驱动侧切 `Window → Live`，本函数纯按
/// 传入 `phase` 决策，不持时钟。
pub fn pipe_resilient(
    _ctx: &mut SseContext,
    _buf: &mut Vec<u8>,
    _chunk: Option<&Bytes>,
    _phase: StreamPhase,
    _cfg: &StreamResilience,
) -> ResilientChunk {
    todo!("TODO(#158): 窗口相缓冲 / Live 相刷+心跳+EOF 判据；对标 _smart_stream")
}
