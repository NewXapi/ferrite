//! 管理面板的共享实体 store：所有 tab 与拓扑图读写同一份 Signal，
//! 任一侧修改立即同步。
//!
//! 数据与 DTO 都归 `crate::api`；本文件只负责把 API 响应包成响应式 Signal。

use dioxus::prelude::*;

use crate::api::{
    self, AliasRow, ChannelRow, CrossRatioRow, GroupRatioRow, GroupRow, KeyRow, ModelPriceRow,
    PresentationRow, RateRuleRow,
};

#[derive(Clone, Copy)]
pub struct EntityStore {
    pub groups: Signal<Vec<GroupRow>>,
    pub aliases: Signal<Vec<AliasRow>>,
    pub channels: Signal<Vec<ChannelRow>>,
    pub presentations: Signal<Vec<PresentationRow>>,
    pub api_keys: Signal<Vec<KeyRow>>,
    pub group_ratios: Signal<Vec<GroupRatioRow>>,
    pub cross_ratios: Signal<Vec<CrossRatioRow>>,
    pub model_prices: Signal<Vec<ModelPriceRow>>,
    pub rate_rules: Signal<Vec<RateRuleRow>>,
    pub banned_words: Signal<Vec<String>>,
    pub announcement: Signal<String>,
    pub system_facts: Signal<Vec<(String, String)>>,
    /// 分组→模型别名关系；图层边由此和渠道 dispatch 派生。
    pub group_alias_links: Signal<Vec<(String, String)>>,
}

impl EntityStore {
    /// 拉一次后端数据，逐字段包成 Signal。
    pub fn load() -> Self {
        let data = api::fetch_admin_data();
        Self {
            groups: Signal::new(data.groups),
            aliases: Signal::new(data.aliases),
            channels: Signal::new(data.channels),
            presentations: Signal::new(data.presentations),
            api_keys: Signal::new(data.api_keys),
            group_ratios: Signal::new(data.group_ratios),
            cross_ratios: Signal::new(data.cross_ratios),
            model_prices: Signal::new(data.model_prices),
            rate_rules: Signal::new(data.rate_rules),
            banned_words: Signal::new(data.banned_words),
            announcement: Signal::new(data.announcement),
            system_facts: Signal::new(data.system_facts),
            group_alias_links: Signal::new(data.group_alias_links),
        }
    }
}
