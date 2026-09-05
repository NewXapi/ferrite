//! `pool` —— `ProxyPool`：按 channel 索引的代理节点池
//!
//! 数据源：`service::sync` 推送的 `ProxySnapshot`，全量替换（ArcSwap）。
//! 进程内：`DashMap<i64 channel_id, Vec<Arc<ProxyNode>>>` 索引。

use super::node::ProxyNode;
use arc_swap::ArcSwap;
use dashmap::DashMap;
use std::sync::Arc;

/// 全量代理节点快照（来自 service::sync）
#[derive(Default)]
pub struct ProxySnapshot {
    pub nodes: Vec<ProxyNode>,
}

/// 代理节点池
pub struct ProxyPool {
    by_channel: DashMap<i64, Arc<Vec<Arc<ProxyNode>>>>,
    snapshot: Arc<ArcSwap<ProxySnapshot>>,
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            by_channel: DashMap::new(),
            snapshot: Arc::new(ArcSwap::from_pointee(ProxySnapshot::default())),
        }
    }

    /// 按 channel_id 选代理节点（priority + 随机）
    pub fn pick(&self, channel_id: i64) -> Option<Arc<ProxyNode>> {
        let list = self.by_channel.get(&channel_id)?;
        // TODO: 按 priority 分层 + 加权随机
        unimplemented!("ProxyPool::pick")
    }

    /// 全量替换快照
    pub fn install(&self, snap: ProxySnapshot) {
        // TODO: 重建 by_channel 索引
        unimplemented!("ProxyPool::install")
    }
}

impl Default for ProxyPool {
    fn default() -> Self {
        Self::new()
    }
}
