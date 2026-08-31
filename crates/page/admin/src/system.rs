//! 系统 tab:系统信息 + 公告 + 运维操作。
//! 数据全部来自 EntityStore mock，写回 announcement 立即同步。

use dioxus::prelude::*;
use crate::state::EntityStore;

#[component]
pub fn SystemPanel() -> Element {
    let store = use_context::<EntityStore>();

    let mut announcement = store.announcement;
    let facts = store.system_facts;

    // Local edit buffer for announcement
    let mut edit_ann = use_signal(|| announcement.read().clone());
    let mut saved = use_signal(|| false);

    // Action states (mock execution feedback)
    let mut cache_cleared = use_signal(|| false);
    let mut topology_rebuilt = use_signal(|| false);
    let mut config_exported = use_signal(|| false);

    let fact_list = use_memo(move || facts.read().clone());

    rsx! {
        div { class: "flex flex-col gap-10 p-1",

            // 1. 系统信息
            div {
                div { class: "flex items-center justify-between mb-4",
                    h2 { class: "text-lg font-semibold text-zinc-100", "系统信息" }
                    div { class: "text-xs text-zinc-500", "实时 mock 数据" }
                }
                div { class: "grid grid-cols-1 md:grid-cols-3 xl:grid-cols-5 gap-3",
                    for (label, value) in fact_list.read().iter() {
                        {
                            rsx! {
                            div { class: "rounded-md border border-zinc-800 bg-zinc-900 p-4",
                                div { class: "text-xs text-zinc-500 mb-1", "{label}" }
                                div { class: "text-lg font-medium text-white break-all", "{value}" }
                            }
                        }
                        }
                    }
                }
            }

            // 2. 系统公告
            div {
                div { class: "flex items-center justify-between mb-4",
                    h2 { class: "text-lg font-semibold text-zinc-100", "系统公告" }
                }

                div { class: "grid grid-cols-1 lg:grid-cols-5 gap-6",
                    // 编辑区
                    div { class: "lg:col-span-3 rounded-md border border-zinc-800 bg-zinc-900 p-5",
                        div { class: "text-xs text-zinc-500 mb-3", "编辑公告内容（保存后全站可见）" }
                        textarea {
                            class: "w-full h-28 rounded-md border border-zinc-700 bg-zinc-950 px-4 py-3 text-sm text-zinc-100 resize-y focus:border-zinc-400 outline-none font-light",
                            value: "{edit_ann.read()}",
                            oninput: move |e| {
                                edit_ann.set(e.value());
                                saved.set(false);
                            },
                        }
                        div { class: "flex gap-3 mt-4",
                            button {
                                class: "flex-1 py-2.5 rounded-md border border-zinc-700 text-zinc-400 hover:bg-zinc-800 text-sm",
                                onclick: move |_| {
                                    edit_ann.set(announcement.read().clone());
                                    saved.set(false);
                                },
                                "重置"
                            }
                            button {
                                class: "flex-1 py-2.5 bg-white text-zinc-900 rounded-md font-medium text-sm hover:bg-zinc-100",
                                onclick: move |_| {
                                    announcement.set(edit_ann.read().clone());
                                    saved.set(true);
                                },
                                if *saved.read() { "已保存 ✓" } else { "保存到 Store" }
                            }
                        }
                    }

                    // 预览卡 (模拟用户看到的公告条)
                    div { class: "lg:col-span-2",
                        div { class: "rounded-md border border-zinc-800 bg-zinc-900 p-5 h-full flex flex-col",
                            div { class: "text-xs text-zinc-500 mb-3 flex items-center gap-2",
                                span { "预览效果" }
                                span { class: "px-2 py-px text-[10px] bg-amber-500/10 text-amber-400 rounded", "用户可见" }
                            }
                            div { class: "flex-1 flex items-center justify-center p-6 bg-gradient-to-r from-zinc-950 to-zinc-900 border border-dashed border-zinc-700 rounded text-center",
                                div { class: "max-w-xs",
                                    div { class: "inline-flex items-center gap-2 px-4 py-1.5 bg-blue-500/10 border border-blue-500/30 rounded-full text-blue-400 text-xs mb-4",
                                        "📢"
                                        "系统公告"
                                    }
                                    p { class: "text-sm text-zinc-300 leading-relaxed",
                                        "{announcement.read()}"
                                    }
                                }
                            }
                            div { class: "text-[10px] text-zinc-500 mt-4 text-center", "公告将以横幅形式展示在前端" }
                        }
                    }
                }
            }

            // 3. 运维操作 (mock)
            div {
                div { class: "mb-4",
                    h2 { class: "text-lg font-semibold text-zinc-100", "运维操作" }
                    p { class: "text-xs text-zinc-500", "点击按钮模拟执行，仅切换文案反馈" }
                }
                div { class: "grid grid-cols-1 md:grid-cols-3 xl:grid-cols-5 gap-3",
                    // 清空缓存
                    div { class: "rounded-md border border-zinc-800 bg-zinc-900 p-5 flex flex-col",
                        div { class: "text-white font-medium mb-1", "清空缓存" }
                        div { class: "text-xs text-zinc-500 flex-1 mb-6", "清除所有 Redis / 本地缓存数据" }
                        button {
                            class: if *cache_cleared.read() {
                                "w-full py-2 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 text-sm"
                            } else {
                                "w-full py-2 rounded-md border border-zinc-700 hover:border-zinc-400 text-sm text-zinc-300 hover:text-white"
                            },
                            onclick: move |_| cache_cleared.set(true),
                            if *cache_cleared.read() { "已执行 ✓" } else { "立即执行" }
                        }
                    }

                    // 重建拓扑缓存
                    div { class: "rounded-md border border-zinc-800 bg-zinc-900 p-5 flex flex-col",
                        div { class: "text-white font-medium mb-1", "重建拓扑缓存" }
                        div { class: "text-xs text-zinc-500 flex-1 mb-6", "重新生成渠道-分组-模型路由表" }
                        button {
                            class: if *topology_rebuilt.read() {
                                "w-full py-2 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 text-sm"
                            } else {
                                "w-full py-2 rounded-md border border-zinc-700 hover:border-zinc-400 text-sm text-zinc-300 hover:text-white"
                            },
                            onclick: move |_| topology_rebuilt.set(true),
                            if *topology_rebuilt.read() { "已执行 ✓" } else { "立即执行" }
                        }
                    }

                    // 导出配置
                    div { class: "rounded-md border border-zinc-800 bg-zinc-900 p-5 flex flex-col",
                        div { class: "text-white font-medium mb-1", "导出配置" }
                        div { class: "text-xs text-zinc-500 flex-1 mb-6", "导出当前完整管理配置为 JSON" }
                        button {
                            class: if *config_exported.read() {
                                "w-full py-2 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/30 text-sm"
                            } else {
                                "w-full py-2 rounded-md border border-zinc-700 hover:border-zinc-400 text-sm text-zinc-300 hover:text-white"
                            },
                            onclick: move |_| config_exported.set(true),
                            if *config_exported.read() { "已执行 ✓" } else { "立即执行" }
                        }
                    }
                }
            }
        }
    }
}
