//! `pool` —— 代理节点池（ArcSwap 单源，channel_key 索引）
//!
//! 单一状态 `ArcSwap<HashMap<channel_key, Vec<Arc<ProxyNode>>>>`：install 一次性
//! 建好索引整体换掉，读走 `load()` 无锁。
//!
//! 一致性是**最终一致**：`install` 原子换指针，但已经拿到旧 `Arc` 的读者会继续
//! 看旧快照直到下次 `load()`。代理选择容忍这点滞后（选到刚被移除的节点，最坏结果
//! 是这一个请求走了旧出口），换来读路径零锁。需要强一致就得加读写锁，代价是
//! 每请求争锁——不值得。

use super::node::ProxyNode;
use arc_swap::ArcSwap;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;

/// 全量代理节点快照（来自 service::sync）
#[derive(Default)]
pub struct ProxySnapshot {
    pub nodes: Vec<ProxyNode>,
}

/// 代理节点池
pub struct ProxyPool {
    // 单一状态：channel_key -> 按 priority 降序的 ProxyNode 列表
    by_channel: ArcSwap<HashMap<String, Vec<Arc<ProxyNode>>>>,
}

impl ProxyPool {
    pub fn new() -> Self {
        Self {
            by_channel: ArcSwap::from(Arc::new(HashMap::new())),
        }
    }

    /// 全量替换快照：重建 channel 索引
    pub fn install(&self, snap: ProxySnapshot) {
        let mut channel_map: HashMap<String, Vec<Arc<ProxyNode>>> = HashMap::new();
        for node in snap.nodes {
            let node_arc = Arc::new(node);
            for channel_key in &node_arc.channel_keys {
                channel_map
                    .entry(channel_key.clone())
                    .or_default()
                    .push(node_arc.clone());
            }
        }
        // 每个 channel 列表按 priority 降序排序（高优先级在前）
        for nodes in channel_map.values_mut() {
            nodes.sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
        }
        self.by_channel.store(Arc::new(channel_map));
    }

    /// 按 channel 选代理节点（priority 分层 + 层内随机）
    ///
    /// `rng` 注入使随机选择的测试完全确定性，与 `dispatch::selector::Selector::pick`
    /// 同一约定（不依赖全局随机源）。
    ///
    /// 返回 `None`：该 channel 无代理节点（调用方按直连处理）。
    pub fn pick(&self, channel_key: &str, rng: &mut dyn rand::RngCore) -> Option<Arc<ProxyNode>> {
        let channel_map = self.by_channel.load();
        let nodes = channel_map.get(channel_key)?;
        // 列表已按 priority 降序：最高层是前缀，取其长度即层大小。
        let max_priority = nodes.first()?.priority;
        let tier_len = nodes
            .iter()
            .take_while(|n| n.priority == max_priority)
            .count();
        let idx = rng.gen_range(0..tier_len);
        Some(nodes[idx].clone())
    }

    /// 该 channel 下全部节点（已按 priority 降序）。无节点时返回空 Vec。
    pub fn candidates(&self, channel_key: &str) -> Vec<Arc<ProxyNode>> {
        self.by_channel
            .load()
            .get(channel_key)
            .cloned()
            .unwrap_or_default()
    }
}

impl Default for ProxyPool {
    fn default() -> Self {
        Self::new()
    }
}
