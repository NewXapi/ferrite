//! # records — 领域实体 (逻辑 schema)
//!
//! 这里是 center/edge/web 三方共用的**存储无关**记录定义。
//! 物理编码 (PG 行、Fjall key/value) 一律由 store crate 的 codec 完成。
//!
//! ## 子模块地图
//!
//! | 模块 | 内容 |
//! |------|------|
//! | [`channel`]  | 上游渠道 + 凭据 |
//! | [`routing`]  | 分组 + 路由调度单元 |
//! | [`identity`] | 用户 + API 令牌 |
//! | [`usage`]    | 用量事件 + 健康观测 |
//! | [`billing`]  | 订阅/订单/兑换 (v2 域, 字段对应 07-database-schema.md) |
//!
//! ## 同构记录信封
//!
//! 每条需要同步的记录都包一层 [`SyncMeta`]: key / schema 版本 / 逻辑版本 /
//! 来源节点 / 时间。这是 center-edge 增量同步的最小元数据 (02-b 路线图 D0)。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub mod billing;
pub mod channel;
pub mod identity;
pub mod routing;
pub mod usage;

pub use billing::{
    PaymentOrderRecord, RedeemCodeRecord, SubscriptionPlanRecord, UserSubscriptionRecord,
};
pub use channel::{ChannelKey, ChannelRecord};
pub use identity::{TokenRecord, UserRecord};
pub use routing::{GroupRecord, RouteUnitRecord};
pub use usage::{HealthObservationRecord, UsageEventRecord};

/// 所有可同步记录共用的元数据信封。
///
/// - `key`: 节点内全局唯一的逻辑主键 (字符串形式, 便于 Fjall 与 PG 统一编码);
/// - `logical_version`: 该记录自身的单调版本, 修改即 +1;
/// - `origin`: 最初创建该记录的节点 (center 下发配置时为 "center")。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncMeta {
    pub key: String,
    pub schema_version: u32,
    pub logical_version: u64,
    pub origin: String,
    pub updated_at: DateTime<Utc>,
}
