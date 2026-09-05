//! EmbeddedStore (edge) — Fjall 实现。
//!
//! 职责 (edge 视角):
//! 1. **snapshot 区**: sync 拉下的 Catalog/Identity 快照 (只读热路径源);
//! 2. **WAL 区**: UsageEvent/HealthObservation 追加日志, 待推送 center;
//! 3. **cursor 区**: 已 ACK 的推送位点。
//!
//! key encoding 规则 (铁律: 只在本模块出现):
//! ```text
//! snapshot/catalog/{key}    → serde_json(ChannelRecord)
//! snapshot/identity/{key}   → serde_json(UserRecord|TokenRecord)
//! wal/usage/{v7_uuid}       → serde_json(UsageEventRecord)   [v7 字节序 = 时间序]
//! wal/health/{v7_uuid}      → serde_json(HealthObservationRecord)
//! cursor/usage-push         → u64 (已 ACK 的 wal 序列位点)
//! meta/schema-version       → u32
//! ```
//!
//! TODO(#413): fjall 依赖引入 + Keyspace/分区打开 + 崩溃恢复验证
//! (02-b D1 spike: SIGKILL 后 snapshot/WAL/cursor 三者一致)。

use crate::error::StoreError;
use crate::traits::*;

/// edge 存储 — 持有 fjall Keyspace + 各分区句柄。
/// TODO(#413): pub struct EmbeddedStore { keyspace: fjall::Keyspace, ... }
pub struct EmbeddedStore;

impl UsageStore for EmbeddedStore {
    async fn append_usage(
        &self,
        _event: &contract::records::UsageEventRecord,
    ) -> Result<contract::mutations::MutationId, StoreError> {
        todo!("TODO(#413): wal/usage/{{uuid}} append (单 key 写, 崩溃安全)")
    }
    async fn pending_usage(
        &self,
        _limit: usize,
    ) -> Result<Vec<contract::mutations::Mutation>, StoreError> {
        todo!("TODO(#413): wal 区 range scan (cursor 之后, 上限条数)")
    }
}

// ChannelStore/UserStore 等的 edge 实现 = snapshot 区只读 (写操作应到达 center,
// edge 收到配置写 = 编程错误 → StoreError::Conflict)。TODO(#414): 只读实现块。
