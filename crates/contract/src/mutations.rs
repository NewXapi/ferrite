//! # mutations — center ↔ edge 增量同步契约
//!
//! 替代 Redis 的同步机制, 数据流:
//!
//! ```text
//! edge pull:  POST /internal/sync/delta { my_summary } → DeltaResponse
//! edge push:  POST /internal/sync/push  { mutations }  → AckResponse
//! ```
//!
//! 正确性规则 (来自 02-b 路线图 D3/D4):
//! 1. edge 报版本摘要 → center 只回缺失区间; gap/不兼容 → 整包 snapshot 兜底;
//! 2. MutationId 重复投递幂等 (center 按 id 去重);
//! 3. edge 收到 Ack 才推进本地 cursor;
//! 4. 通知 (LISTEN/NOTIFY) 只是加速, 轮询是正确性路径。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 全局唯一变更 id, UUIDv7 (时间有序 → 天然可按时间范围扫 WAL)。
pub type MutationId = Uuid;

/// 同步域划分: 每个域独立版本号, 互不阻塞。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncDomain {
    /// 配置: channel / route-unit / group (center 权威, edge 只读)。
    Catalog,
    /// 身份: user / token (center 权威, edge 只读快照)。
    Identity,
    /// 用量: usage-event / health (edge 权威产生, center 汇聚)。
    Usage,
}

/// 变更载荷: upsert 一条记录, 或删除一个 key。
/// 记录类型按域匹配 — Catalog 只会出现 Channel/Group/RouteUnit, 以此类推。
/// TODO(#230): center 收到不匹配域的 payload 时应拒绝并告警 (校验在 sync 层)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MutationPayload {
    UpsertChannel {
        record: crate::records::ChannelRecord,
    },
    UpsertGroup {
        record: crate::records::GroupRecord,
    },
    UpsertRouteUnit {
        record: crate::records::RouteUnitRecord,
    },
    UpsertUser {
        record: crate::records::UserRecord,
    },
    UpsertToken {
        record: crate::records::TokenRecord,
    },
    /// 用量/健康不做 upsert 语义 — 它们是 append-only 事件, 直接携带记录。
    UsageEvent {
        record: crate::records::UsageEventRecord,
    },
    HealthObservation {
        record: crate::records::HealthObservationRecord,
    },
    /// 逻辑删除: 只带 key, sync 层负责各存储的 tombstone 编码。
    Delete {
        key: String,
    },
}

/// 一条变更 = 幂等投递的最小单元。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Mutation {
    pub id: MutationId,
    pub domain: SyncDomain,
    /// 发起节点标识 ("center" / edge node id)。
    pub origin: String,
    /// 产生时间 (v7 uuid 已含时间, 但独立字段便于日志检索)。
    pub created_at: i64,
    pub payload: MutationPayload,
}

/// 版本摘要: edge 每次握手携带的"我已应用到哪个版本"。
/// key 是域名, 值是该域已应用的最后 revision。
pub type VersionSummary = BTreeMap<SyncDomain, u64>;

/// center 对 pull 的应答: 各域的增量区间。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaRange {
    /// 该域 center 当前最新 revision。
    pub latest: u64,
    /// edge 应应用的变更 (empty = 已是最新)。
    /// None = gap/落后过多, 走 snapshot 兜底 (见 below)。
    pub mutations: Option<Vec<Mutation>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeltaResponse {
    pub catalog: DeltaRange,
    pub identity: DeltaRange,
    /// Usage 域是 edge→center 方向, pull 应答里恒为 latest=0。
    pub usage: DeltaRange,
    /// 有任一域 gap=true 时, snapshot 非空: 该域全量压缩快照 (serde_json 字节)。
    /// TODO(#231): snapshot 载荷编码 — 直接 Vec<Record> JSON 还是 zstd? 先 JSON, 压缩以后加。
    pub snapshot: Option<SnapshotEnvelope>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotEnvelope {
    pub domain: SyncDomain,
    pub revision: u64,
    /// JSON 序列化的记录数组; 解码后由 store 层原子替换本地快照。
    pub records: Vec<serde_json::Value>,
}

/// edge push 批次的应答: center 逐条 ACK。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AckResponse {
    /// 已成功落库的 MutationId; edge 收到后推进 cursor + 清理 WAL。
    pub acked: Vec<MutationId>,
    /// 拒绝的条目及原因 (schema 不兼容等); edge 侧进死信队列而非无限重试。
    /// TODO(#232): 死信处理策略 — 重试 N 次后丢弃? 上报告警? 待定。
    pub rejected: Vec<RejectedMutation>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RejectedMutation {
    pub id: MutationId,
    pub reason: String,
}
