//! # sync — center ↔ edge 同步引擎
//!
//! 方向与触发 (02-b 路线图 D4):
//! ```text
//! edge pull (配置下行):  轮询是正确性路径, LISTEN/NOTIFY 只是加速
//! edge push (用量上行):  pending >= batch_size OR oldest >= interval
//! ```
//! 故障语义 (每条可测): center 不可达→last-known-good; 重复 batch→幂等;
//! gap→snapshot 兜底; 双端不可用→fail-closed。
//!
//! ## 模块地图
//!
//! | 模块 | 职责 |
//! |------|------|
//! | [`edge`]    | edge 引擎: pull (报摘要→应用增量/snapshot) + push (WAL 批→ACK) |
//! | [`center`]  | center 引擎: serve_delta (对账增量) + serve_push (幂等落库) |
//! | [`snapshot`] | 快照组装/解码/原子替换 |

pub mod center;
pub mod edge;
pub mod snapshot;

use contract::mutations::{AckResponse, DeltaResponse, Mutation, VersionSummary};

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// 网络层失败 — 调用方按退避重试, 不区分错误细节。
    #[error("transport: {0}")]
    Transport(String),
    /// 契约不匹配 (schema 版本落差) → 触发 snapshot 兜底而非报错。
    #[error("schema mismatch: local={local}, remote={remote}")]
    SchemaMismatch { local: u32, remote: u32 },
}

/// edge 侧引擎 — apps/gateway 持有, 后台任务驱动。
pub trait EdgeSync: Send + Sync {
    /// 一次 pull: 报本地摘要 → 应用增量/snapshot → 返回是否发生了快照替换。
    /// 应用顺序: 先落 store (幂等) → 原子替换内存快照 (ArcSwap) → 推进 cursor。
    /// 任何一步失败都不推进 cursor (下轮重放, 依赖幂等)。
    fn pull(&self, summary: &VersionSummary)
    -> impl Future<Output = Result<PullOutcome, SyncError>> + Send;

    /// 一次 push: 取 store 的 pending 批 → POST → 按 Ack 推进 cursor。
    fn push(&self) -> impl Future<Output = Result<AckResponse, SyncError>> + Send;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PullOutcome {
    /// 应用了增量 mutations。
    Delta,
    /// 触发了整包快照替换 (gap / 落后过多 / 首次启动)。
    SnapshotReplaced,
    /// 无变化。
    UpToDate,
}

/// center 侧引擎 — apps/console 持有。
pub trait CenterSync: Send + Sync {
    fn serve_delta(&self, summary: &VersionSummary)
    -> impl Future<Output = Result<DeltaResponse, SyncError>> + Send;
    fn serve_push(&self, batch: &[Mutation])
    -> impl Future<Output = Result<AckResponse, SyncError>> + Send;
}
