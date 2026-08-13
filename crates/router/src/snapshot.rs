//! `ArcSwap<Snapshot>`：读无锁；SIGHUP 时整体替换。见 docs/08-mvp.md §4.3
//!
//! 快照是不可变的：重载时构造全新 `Snapshot` 再一次原子替换，
//! 因此重载失败时旧快照仍然完整可用（§2.3 的不变量）。

use rustc_hash::FxHashMap;

/// 渠道在快照中的下标。
pub type ChannelIdx = u32;

/// 一次请求要读的全部路由状态。构建期完成索引与排序，请求期只查表。
#[derive(Debug, Default)]
pub struct Snapshot {
    /// 模型名 -> 候选渠道，**已按 priority 降序排好**。见 §4.2
    pub by_model: FxHashMap<Box<str>, Vec<ChannelIdx>>,
    /// 别名 -> 真实模型名。
    pub alias: FxHashMap<Box<str>, Box<str>>,
}

impl Snapshot {
    /// 解析别名后返回候选渠道。未命中返回空切片。
    pub fn candidates(&self, model: &str) -> &[ChannelIdx] {
        let resolved = self.alias.get(model).map(|s| &**s).unwrap_or(model);
        self.by_model.get(resolved).map(|v| &**v).unwrap_or(&[])
    }
}
