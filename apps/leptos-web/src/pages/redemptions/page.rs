use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, CardHeader, CardTitle, CardContent};

use super::data::{RedemptionCardData, RedemptionDemoData};


use super::card::*;
use super::data::*;

#[component]
fn RedemptionsStatsSection(data: RedemptionDemoData) -> impl IntoView {
    view! {
        div { class: "grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-4",
            // Total count card
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80",
                div { class: "flex items-center gap-3 mb-2",
                    div { class: "h-8 w-8 rounded-lg bg-blue-500/20 flex items-center justify-center",
                        svg { class: "h-4 w-4 text-blue-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M19 11H5m14 0a9 9 0 0 1-9 9 9 0 0 1-9-9 9 0 0 1 9 9 9 0 0 1 9-9z" }
                        }
                    }
                    span { class: "text-sm font-medium text-zinc-400", "兑换码总数" }
                }
                div { class: "text-2xl font-bold text-zinc-100", {data.total_count} }
                div { class: "text-xs text-zinc-500 mt-1", "所有兑换码的总量" }
            }

            // Unused count card
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80",
                div { class: "flex items-center gap-3 mb-2",
                    div { class: "h-8 w-8 rounded-lg bg-emerald-500/20 flex items-center justify-center",
                        svg { class: "h-4 w-4 text-emerald-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9 12l2 2 4-4m6 2a9 9 0 1 1-18 0 9 9 0 1 1 18 0z" }
                        }
                    }
                    span { class: "text-sm font-medium text-zinc-400", "未使用" }
                }
                div { class: "text-2xl font-bold text-emerald-400", {data.unused_count} }
                div { class: "text-xs text-zinc-500 mt-1", "可用于兑换的码数" }
            }

            // Used count card
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80",
                div { class: "flex items-center gap-3 mb-2",
                    div { class: "h-8 w-8 rounded-lg bg-zinc-400/20 flex items-center justify-center",
                        svg { class: "h-4 w-4 text-zinc-300", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9 5H7a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V7a2 2 0 0 0-2-2h-2M9 5a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2M9 5V3a2 2 0 0 1 2-2h2a2 2 0 0 1 2 2v2" }
                        }
                    }
                    span { class: "text-sm font-medium text-zinc-400", "已核销" }
                }
                div { class: "text-2xl font-bold text-zinc-300", {data.used_count} }
                div { class: "text-xs text-zinc-500 mt-1", "已被使用的兑换码数" }
            }

            // Disabled count card
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/80",
                div { class: "flex items-center gap-3 mb-2",
                    div { class: "h-8 w-8 rounded-lg bg-amber-500/20 flex items-center justify-center",
                        svg { class: "h-4 w-4 text-amber-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 1 1-18 0 9 9 0 1 1 18 0z" }
                        }
                    }
                    span { class: "text-sm font-medium text-zinc-400", "已停用" }
                }
                div { class: "text-2xl font-bold text-amber-400", {data.disabled_count} }
                div { class: "text-xs text-zinc-500 mt-1", "已被停用的兑换码数" }
            }
        }
    }
}

// List section - adapted from dioxus list.rs
#[component]
fn RedemptionsListSection(
    redemptions: Vec<RedemptionCardData>,
    on_disable: EventHandler<String>,
) -> impl IntoView {
    // Filter state
    let (search_term, set_search_term) = signal(String::new());
    let (status_filter, set_status_filter) = signal(0u8); // 0 = 全部, 1 = 未使用, 2 = 已核销, 3 = 已停用

    // Filtered redemptions
    let filtered_redemptions = Signal::derive(move || {
        redemptions.iter()
            .filter(|r| {
                // Search filter
                let search_match = search_term.get().is_empty() ||
                    r.code_preview.to_lowercase().contains(&search_term.get().to_lowercase()) ||
                    r.key.to_lowercase().contains(&search_term.get().to_lowercase());

                // Status filter
                let status_match = status_filter.get() == 0 || r.status == status_filter.get();

                search_match && status_match
            })
            .cloned()
            .collect::<Vec<_>>()
    });

    let filtered_count = Signal::derive(move || filtered_redemptions.get().len());

    view! {
        section { id: "reds-sec-list", class: "scroll-mt-8 space-y-4",
            "data-testid": "redemptions-list",
            role: "list",
            "aria-label": "兑换码列表",

            // Header
            div { class: "flex flex-col gap-3 md:flex-row md:items-center md:justify-between",
                div { class: "flex flex-col gap-1",
                    h2 { class: "text-lg font-medium text-zinc-100", "兑换码列表" }
                    p { class: "text-sm text-zinc-500", {move || format!("{} 张卡片", filtered_count.get())} }
                }
            }

            // Search and filter toolbar
            div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-4",
                "aria-label": "兑换码筛选与操作",

                div { class: "flex flex-col gap-3 md:flex-row md:items-center",
                    // Search input
                    div { class: "relative flex-1",
                        input {
                            class: "w-full rounded-lg border border-zinc-700 bg-zinc-800/50 px-3 py-2 pl-10 text-sm text-zinc-100 placeholder-zinc-500 focus:border-emerald-500/50 focus:outline-none focus:ring-1 focus:ring-emerald-500/50",
                            "type": "text",
                            placeholder: "搜索兑换码预览 (如 fx-086c****) ...",
                            value: search_term.get(),
                            on:input: move |e| set_search_term.set(event_target_value(&e)),
                        }
                        svg { class: "absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-zinc-400", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                            path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M21 21l-6-6m2-5a7 7 0 1 1-14 0 7 7 0 0 1 14 0z" }
                        }
                    }

                    // Status filter
                    div { class: "flex gap-2",
                        button {
                            class: format!("px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", if status_filter.get() == 0 { "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30" } else { "bg-zinc-800/80 text-zinc-300 border border-zinc-700 hover:bg-zinc-700/50" }),
                            on:click: move |_| set_status_filter.set(0u8),
                            "全部"
                        }
                        button {
                            class: format!("px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", if status_filter.get() == 1 { "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30" } else { "bg-zinc-800/80 text-zinc-300 border border-zinc-700 hover:bg-zinc-700/50" }),
                            on:click: move |_| set_status_filter.set(1u8),
                            "未使用"
                        }
                        button {
                            class: format!("px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", if status_filter.get() == 2 { "bg-zinc-400/20 text-zinc-300 border border-zinc-400/30" } else { "bg-zinc-800/80 text-zinc-300 border border-zinc-700 hover:bg-zinc-700/50" }),
                            on:click: move |_| set_status_filter.set(2u8),
                            "已核销"
                        }
                        button {
                            class: format!("px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", if status_filter.get() == 3 { "bg-amber-500/20 text-amber-300 border border-amber-500/30" } else { "bg-zinc-800/80 text-zinc-300 border border-zinc-700 hover:bg-zinc-700/50" }),
                            on:click: move |_| set_status_filter.set(3u8),
                            "已停用"
                        }
                    }
                }
            }

            // Redemptions grid
            {move || {
                let filtered = filtered_redemptions.get();
                if filtered.is_empty() {
                    view! {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16",
                            role: "alert",
                            "aria-label": "兑换码加载失败",

                            div { class: "flex flex-col items-center gap-4 text-center",
                                svg { class: "h-12 w-12 text-zinc-600", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9.172 16.172a4 4 0 0 1 5.656 0M9 10h.01M15 10h.01M21 12a9 9 0 1 1-18 0 9 9 0 1 1 18 0z" }
                                }
                                p { class: "text-lg font-medium text-zinc-400", "没有匹配的兑换码" }
                                p { class: "text-sm text-zinc-500", "尝试调整搜索词或状态筛选器" }
                            }
                        }
                    }.into_any()
                } else {
                    view! {
                        CardGrid { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            {filtered.into_iter().map(|redemption| {
                                view! {
                                    RedemptionCard {
                                        item: redemption,
                                        on_copy: move |key| { /* Handle copy action */ },
                                        on_disable: on_disable.clone(),
                                    }
                                }
                            }).collect::<Vec<_>>()}
                        }
                    }.into_any()
                }
            }}
        }
    }
}

// Main component
#[component]
pub fn RedemptionsPage() -> impl IntoView {
    // State management
    let (redemptions, set_redemptions) = signal(demo_data().redemptions);
    let loading = signal(false);
    let error = signal(None::<String>);
    let reload_trigger = signal(0u32);

    // Actions
    let handle_disable = move |key: String| {
        // In real app, this would call the disable API
        // For demo, just remove the redemption from the list
        set_redemptions.update(|reds| {
            reds.retain(|r| r.key != key);
        });
    };

    let _handle_reload = move || {
        reload_trigger.update(|v| *v += 1);
    };

    // Auto-load effect - using demo data
    use_effect(move || {
        let data = demo_data();
        set_redemptions.set(data.redemptions);
        loading.set(false);
    });

    // Computed values
    let stats_data = Signal::derive(move || {
        let data = demo_data();
        RedemptionDemoData {
            total_count: data.total_count,
            unused_count: data.unused_count,
            used_count: data.used_count,
            disabled_count: data.disabled_count,
            available_quota: data.available_quota,
            redemptions: redemptions.get().clone(),
        }
    });

    view! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            // Stats section
            RedemptionsStatsSection { data: stats_data() }

            // List section
            RedemptionsListSection {
                redemptions: redemptions.get().clone(),
                on_disable: handle_disable,
            }
        }
    }
}