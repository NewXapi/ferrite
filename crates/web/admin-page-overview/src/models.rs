//! 模型页 — 卡网格,数据来自真实 `/api/models`(管理端模型列表)。
//!
//! 卡面渲染交由 `ui::components::showcase::StatTabsCard`(自本页旧实现抽象出的通用
//! 展示组件), 本文件只做 [`ModelCardView`] → props 的数据映射。后端没有的字段
//! (描述、价格、24h 统计、趋势、分组报价)渲染「—」占位或空序列,不造数据。

use dioxus::prelude::*;

use crate::api::{self, ModelCardView};
use client::ApiClient;
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
            description: "—".to_string(),
            price: PriceTriple {
                input: "—".to_string(),
                output: "—".to_string(),
                cache: "—".to_string(),
            },
            headline: HeadlineStat {
                label: "累计调用".to_string(),
                value: model.usage_count.to_string(),
                sub: None,
            },
            mini_stats: vec![
                MiniStatItem {
                    label: "类型".to_string(),
                    value: model.model_type,
                },
                MiniStatItem {
                    label: "最大上下文".to_string(),
                    value: model.max_tokens.to_string(),
                },
                MiniStatItem {
                    label: "状态".to_string(),
                    value: if model.status == 1 {
                        "启用".to_string()
                    } else {
                        "停用".to_string()
                    },
                },
            ],
            trend: vec![],
            heat: vec![],
            groups: vec![],
        }
    }
}

/// 模型页入口: 卡网格(页头 + ModelCard), 数据经 [`api::list_models_api`] 取自
/// `/api/models`(loading / 错误 / 空态走与 `health.rs` 同一的 Signal 取数模式)。
/// 遵循页面响应式约定 (手机 1 栏 / 平板 md 3 栏 / Web lg 5 栏)。
#[component]
pub fn ModelsPanel() -> Element {
    let mut models = use_signal(Vec::<ModelCardView>::new);
    let mut total = use_signal(|| 0i64);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match api::list_models_api(&client).await {
                Ok((items, n)) => {
                    models.set(items);
                    total.set(n);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = models();
    let total = total();
    let loading = loading();
    let err = err();

    rsx! {
        div { class: "space-y-4",
            "data-testid": "models-panel",
            role: "region",
            "aria-label": "模型",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                span { class: "text-xs text-zinc-600", "共 {total} 个" }
            }
            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "text-sm text-red-300", "加载模型列表失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-border px-3 py-1.5 text-xs text-muted-foreground hover:bg-accent",
                        onclick: move |_| reload.set(reload() + 1),
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "正在加载模型列表…" }
                }
            } else if list.is_empty() {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "暂无模型" }
                    p { class: "mt-1 text-xs text-muted-foreground/70", "/api/models 返回空列表 —— 配置模型后这里会展示真实卡片" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                    for m in &list {
                        ModelCard { model: m.clone() }
                    }
                }
            }
        }
    }
}
