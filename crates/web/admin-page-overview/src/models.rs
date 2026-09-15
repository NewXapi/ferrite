//! 模型页 — 三内部 tab 演示卡网格(概览 / 分组价格 / 待定)。
//!
//! 卡面渲染交由 `ui::components::showcase::StatTabsCard`(自本页旧实现抽象出的通用
//! 展示组件), 本文件只做 [`ModelInfo`] → props 的数据映射; 数据源为 admin-mock
//! crate 的 `mock::models::MODELS`(纯演示数值,非后端数据), 页头「mock · N 个」
//! 如实标注演示属性。

use dioxus::prelude::*;

use mock::models::{MODELS, ModelInfo};
use ui::components::showcase::StatTabsCard;
use ui::components::showcase::stat_tabs_card::{
    GroupPriceRow, HeadlineStat, MiniStatItem, PriceTriple,
};

/// 模型展示卡: 把 [`ModelInfo`] 字段映射为 [`StatTabsCard`] props(三内部 tab:
/// 概览 / 分组价格 / 待定), 卡面结构与数据口径和映射前一致。
#[component]
pub fn ModelCard(model: ModelInfo) -> Element {
    let group_rows: Vec<GroupPriceRow> = model
        .groups
        .iter()
        .map(|g| GroupPriceRow {
            name: g.name.to_string(),
            price: PriceTriple {
                input: g.input.to_string(),
                output: g.output.to_string(),
                cache: g.cache.to_string(),
            },
        })
        .collect();

    rsx! {
        StatTabsCard {
            title: model.name.to_string(),
            subtitle: model.vendor.to_string(),
            description: model.description.to_string(),
            price: PriceTriple {
                input: model.price_input.to_string(),
                output: model.price_output.to_string(),
                cache: model.price_cache.to_string(),
            },
            headline: HeadlineStat {
                label: "24h Tokens".to_string(),
                value: model.tokens_24h.to_string(),
                sub: Some(format!("${}", model.cost_24h)),
            },
            mini_stats: vec![
                MiniStatItem {
                    label: "请求".to_string(),
                    value: model.requests_24h.to_string(),
                },
                MiniStatItem {
                    label: "成功率".to_string(),
                    value: model.success_rate.to_string(),
                },
                MiniStatItem {
                    label: "P50 延迟".to_string(),
                    value: model.latency_p50.to_string(),
                },
            ],
            trend: model.trend.to_vec(),
            heat: model.heat.to_vec(),
            groups: group_rows,
        }
    }
}

/// 模型页入口: 演示卡网格(页头 + ModelCard), 数据源为 admin-mock 演示数值。
/// 遵循页面响应式约定 (手机 1 栏 / 平板 md 3 栏 / Web lg 5 栏)。
#[component]
pub fn ModelsPanel() -> Element {
    let models = MODELS;

    rsx! {
        div { class: "space-y-4",
            "data-testid": "models-demo",
            role: "region",
            "aria-label": "模型（演示）",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                span { class: "text-xs text-zinc-600", "mock · {models.len()} 个" }
            }
            div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                for m in models {
                    ModelCard { model: m.clone() }
                }
            }
        }
    }
}
