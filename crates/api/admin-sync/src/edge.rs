//! edge 引擎 — pull/push 的执行编排。
//!
//! pull 流程:
//! ```text
//! 1. 读本地 summary {catalog: 41, identity: 18, usage: 0}
//! 2. POST /internal/sync/delta
//! 3. 响应分发:
//!    - 增量 (Option::Some(muts)) → 逐条 store 落地 (幂等) → 内存快照重建 → cursor++
//!    - snapshot (Option::Some(env)) → snapshot::replace (原子, 见 snapshot.rs)
//!    - gap (None) → 视为 snapshot 路径
//! ```
//! push 流程:
//! ```text
//! 1. store.pending_usage(batch) → 未 ACK 批
//! 2. POST /internal/sync/push
//! 3. acked → cursor 推进 + WAL 清理; rejected → 死信 (TODO(#232))
//! ```
//!
//! TODO(#430): pull/push 周期驱动 (tokio interval) + 指数退避 + LISTEN/NOTIFY 加速通道。

use super::{PullOutcome, SyncError};
use contract::mutations::VersionSummary;

pub struct EdgeEngine;

impl super::EdgeSync for EdgeEngine {
    async fn pull(&self, _summary: &VersionSummary) -> Result<PullOutcome, SyncError> {
        todo!("TODO(#430): pull 编排 (delta 应用/snapshot 替换/cursor)")
    }
    async fn push(&self) -> Result<contract::mutations::AckResponse, SyncError> {
        todo!("TODO(#430): push 编排 (WAL 批/ACK/cursor 推进)")
    }
}
