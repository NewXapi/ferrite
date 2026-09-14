//! 模型卡片网格 — 数据来自真实 `GET /api/models`(见 [`crate::api::list_models_api`])。
//! 卡牌视觉恢复 #154 前的「模型展示卡」分区样式: 渐变玻璃容器 + 头部名称区 + 分区线
//! 加大数字统计块与 MiniStat 网格; 后端没有的字段(价格、趋势、分组报价)不展示,不造数据;
//! loading / error / empty 三态诚实,写法与 health.rs / overview.rs 一致。

use dioxus::prelude::*;

use client::ApiClient;

use crate::api::{self, ModelCardView};

/// One model = one card. 数据只来自真实 `ModelCardView` 字段(来源与 #154 接线版一致),
/// 卡面结构恢复 #154 前的「模型展示卡」: 头部(名称/归属/状态) + 分区线 + 大数字统计块
/// + 三列 MiniStat。Width and flow come from the parent layout; the card is self-contained.
#[component]
pub fn ModelCard(model: ModelCardView) -> Element {
    let card_cls = "flex flex-col gap-3 rounded-2xl border border-white/10 \
                    bg-gradient-to-b from-zinc-800/60 to-zinc-900/40 p-4 \
                    shadow-xl shadow-black/40 ring-1 ring-white/5 backdrop-blur-xl";

    // 状态:1 = 启用(与后端 status = 1 判定同口径),其余一律视为停用
    let (status_text, status_cls) = if model.status == 1 {
        ("启用", "text-emerald-400")
    } else {
        ("停用", "text-zinc-500")
    };

    rsx! {
        section { class: "{card_cls}",
            // Top bar: name + 归属/类型 + status(旧版头部槽位: 名称 + 厂商)
            header { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    h3 { class: "truncate text-base font-semibold tracking-tight text-zinc-50", "{model.name}" }
                    p { class: "mt-0.5 text-xs text-zinc-500", "{model.model_type} · {model.owner}" }
                }
                span { class: "shrink-0 {status_cls} text-xs font-medium", "{status_text}" }
            }

            div { class: "border-t border-white/5" }

            // 大数字统计块(旧版「24H TOKENS」槽位): 累计调用为主数 + 三列关键指标。
            // 能力布尔为 false 时含后端缺省(serde default)的情形,用「—」表示未声明,不武断展示「不支持」。
            div {
                p { class: "text-[11px] uppercase tracking-wider text-zinc-600", "累计调用" }
                p { class: "mt-1 text-2xl font-semibold tabular-nums tracking-tight text-zinc-50", "{model.usage_count}" }
                div { class: "mt-3 grid grid-cols-3 gap-2",
                    MiniStat { label: "最大上下文", value: format!("{} tokens", model.max_tokens) }
                    MiniStat { label: "视觉", value: if model.is_vision { "支持".to_string() } else { "—".to_string() } }
                    MiniStat { label: "工具调用", value: if model.is_tool { "支持".to_string() } else { "—".to_string() } }
                }
            }
        }
    }
}

#[component]
fn MiniStat(label: &'static str, value: String) -> Element {
    rsx! {
        div {
            p { class: "text-[11px] text-zinc-600", "{label}" }
            p { class: "mt-0.5 text-sm font-medium tabular-nums text-zinc-200", "{value}" }
        }
    }
}

/// 模型卡片网格: 遵循页面响应式约定 (手机 1 栏 / 平板 md 3 栏 / Web xl 5 栏)。
/// 数据来自真实 /api/models;后端单页上限 100 条,total 单独标注。
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
                Ok((items, tot)) => {
                    models.set(items);
                    total.set(tot);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let data = models();
    let total_n = total();
    let is_loading = loading();
    let error = err();

    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                div { class: "flex items-center gap-3",
                    // total 是库内总数;显示数受后端单页 100 条上限约束
                    span { class: "text-xs text-zinc-600", "共 {total_n} 个 · 显示 {data.len()} 个" }
                    button {
                        class: "shrink-0 rounded-xl border border-zinc-700 px-3 py-2 text-xs text-zinc-300 transition-colors hover:bg-zinc-800",
                        "data-testid": "refresh-models",
                        onclick: move |_| reload.set(reload() + 1),
                        "刷新"
                    }
                }
            }
            section { "data-testid": "models-panel", role: "region", "aria-label": "模型列表",
                if let Some(e) = error {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                        p { class: "text-sm text-red-300", "加载模型列表失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "retry-models",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if is_loading {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "正在加载模型…" }
                    }
                } else if data.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center",
                        p { class: "text-zinc-400", "后端无模型" }
                        p { class: "mt-1 text-xs text-zinc-600", "在管理端创建模型后这里会展示真实列表" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                        for m in data {
                            ModelCard { model: m }
                        }
                    }
                }
            }
        }
    }
}
