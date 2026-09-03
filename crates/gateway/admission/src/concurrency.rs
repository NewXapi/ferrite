//! 闸③ 并发控制 — 本地 Semaphore 池。
//!
//! 替代 sub2api 的 Redis 全局槽位 (concurrency_cache.go):
//! **多副本下不追求全局精确并发** — sticky-ingress 把同一用户固定到同一
//! edge Pod, Pod 内 Semaphore 已足够 (设计文档原则 10: 没有证据不做全局 lease)。
//!
//! 两级槽位:
//! - channel 级: ChannelRecord.max_concurrency (快照下发);
//! - 单 key 级: 上游供应商对单 key 的并发限制 (比渠道级更细, new-api multikey 场景)。

/// 并发槽池 — per-channel + per-key 的本地 Semaphore 集合。
///
/// 实现提示: DashMap<ChannelKey, Arc<Semaphore>>; 快照替换时旧 Semaphore
/// 保留至全部 permit 归还 (Arc 计数) — 新请求走新池。
/// TODO(#305): DashMap 依赖引入 + 快照替换时的池迁移语义。
#[derive(Default)]
pub struct ConcurrencyPool {
    _priv: (),
}

/// 闸③: 获取渠道并发槽。
///
/// 返回 hold_id (释放凭据); 满 = Rejection::Busy (429, 客户端退避)。
/// dispatch 失败换候选时: 先 release 旧槽, 再对新渠道 acquire。
pub trait Gate: Send + Sync {
    fn acquire(&self, channel_key: &str, key_index: u32) -> Result<u64, crate::error::Rejection>;
    /// 幂等释放。
    fn release(&self, hold_id: u64);
}

impl Gate for ConcurrencyPool {
    fn acquire(&self, _channel_key: &str, _key_index: u32) -> Result<u64, crate::error::Rejection> {
        todo!("TODO(#305): Semaphore acquire + hold_id 登记")
    }
    fn release(&self, _hold_id: u64) {
        todo!("TODO(#305): 幂等归还")
    }
}
