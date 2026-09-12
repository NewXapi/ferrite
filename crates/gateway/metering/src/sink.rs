//! `sink` — 结算产物落地通道。
//!
//! 库层（metering + forward）只负责**产出** [`UsageEventRecord`]；
//! 事件往哪里去（写 usage_logs 平表 / WAL / 扣内存 quota）是 apps 侧的
//! 装配决策，通过实现本 trait 注入。库不认识存储。

use contract::records::UsageEventRecord;

/// 结算产物落地通道：apps 侧实现（写 usage_logs / 扣内存 quota）。
///
/// 库层只产出事件、不认识存储。实现必须**非阻塞或快速返回**：
/// `submit` 在热路径（流结束 / 响应提交点）上被同步调用，慢实现会拖住
/// 转发管道——重活（落盘、推送）请实现方自己 spawn 到后台。
pub trait SettleSink: Send + Sync {
    /// 提交一条结算事件。幂等性由实现的消费端负责
    /// (`event.meta.key` 全局唯一, 重复提交应被去重而非重复入账)。
    fn submit(&self, event: UsageEventRecord);
}
