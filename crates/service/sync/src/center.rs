//! center 引擎 — 对账与幂等落库。
//!
//! serve_delta:
//! ```sql
//! -- 对每域: edge 报 v_local, center 当前 v_latest
//! -- v_local == v_latest        → unchanged
//! -- v_local <  v_latest, 有记录 → mutations (v_local+1 .. v_latest] (outbox 序)
//! -- 记录被 compact / gap       → snapshot 兜底 (DeltaRange.mutations = None)
//! ```
//!
//! serve_push:
//! ```sql
//! -- 每条 mutation: 按 id 幂等
//! INSERT INTO usage_logs ... ON CONFLICT (mutation_id) DO NOTHING
//! -- acknowledged (无论首次还是重放) → acked.push(id)
//! ```
//!
//! TODO(#431): mutation 载荷 → 表的逐类型分发 (match MutationPayload);
//! 域不匹配载荷 → rejected (契约校验在 sync 层)。

use super::SyncError;
use contract::mutations::{AckResponse, DeltaResponse, Mutation, VersionSummary};

pub struct CenterEngine;

impl super::CenterSync for CenterEngine {
    async fn serve_delta(&self, _summary: &VersionSummary) -> Result<DeltaResponse, SyncError> {
        todo!("TODO(#431): 三域对账 + outbox 读取 + snapshot 兜底判定")
    }
    async fn serve_push(&self, _batch: &[Mutation]) -> Result<AckResponse, SyncError> {
        todo!("TODO(#431): 逐条幂等落库 + acked/rejected 分类")
    }
}
