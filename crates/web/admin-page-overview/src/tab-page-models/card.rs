//! 模型展示卡:把 [`ModelCardView`] 字段映射为通用 `StatTabsCard` 的 props。

use dioxus::prelude::*;

use super::shared::{
    CARD_CONTEXT, CARD_DISABLED, CARD_ENABLED, CARD_HEADLINE, CARD_STATUS, CARD_TYPE, DASH,
};
use crate::api::ModelCardView;
use ui::components::showcase::StatTabsCard;
use ui::components::showcase::stat_tabs_card::{HeadlineStat, MiniStatItem, PriceTriple};

/// 模型展示卡: 把 [`ModelCardView`] 字段映射为 [`StatTabsCard`] props。
///
/// 后端缺的字段(描述、输入/输出/缓存价、24h 统计、趋势、热力、分组报价)以
/// 「—」或空序列占位; 有数据的字段映射为: 累计调用(大数字)、类型 / 最大上下文 /
/// 启用状态(三列迷你统计)。卡面结构与取数口径和映射前一致。
#[component]
pub fn ModelCard(model: ModelCardView) -> Element {
    rsx! {
        StatTabsCard {
            title: model.name,
            subtitle: model.owner,
            description: DASH.to_string(),
            price: PriceTriple {
                input: DASH.to_string(),
                output: DASH.to_string(),
                cache: DASH.to_string(),
            },
            headline: HeadlineStat {
                label: CARD_HEADLINE.to_string(),
                value: model.usage_count.to_string(),
                sub: None,
            },
            mini_stats: vec![
                MiniStatItem {
                    label: CARD_TYPE.to_string(),
                    value: model.model_type,
                },
                MiniStatItem {
                    label: CARD_CONTEXT.to_string(),
                    value: model.max_tokens.to_string(),
                },
                MiniStatItem {
                    label: CARD_STATUS.to_string(),
                    value: if model.status == 1 {
                        CARD_ENABLED.to_string()
                    } else {
                        CARD_DISABLED.to_string()
                    },
                },
            ],
            trend: vec![],
            heat: vec![],
            groups: vec![],
        }
    }
}
