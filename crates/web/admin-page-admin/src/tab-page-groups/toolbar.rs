//! 分组筛选与批量操作区:搜索框 + 状态分级胶囊 + 批量点选 chips + 批量动作条。
//! 纯交互组件:筛选/多选 signal 由页面注入就地读写,刷新/新建/批量动作
//! 以 `EventHandler` 抛回页面。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use contract::api::admin::GroupDto;

use super::shared::SEC_FILTER;

/// 筛选与操作区。
///
/// - `groups`:全量分组(chips 点选用,不经过关键词/分级筛选)。
/// - `filter_options`:分级胶囊文案(含计数,由页面算好传入)。
/// - `search` / `filter_tier` / `selected`:筛选与多选 signal,直接传入,
///   组件内就地读写(对齐 tab-page-currency 的 Signal-prop 约定)。
/// - `on_refresh` / `on_new` / `on_bulk_*`:写回页面的回调。
#[component]
pub fn GroupsToolbar(
    groups: Vec<GroupDto>,
    filter_options: Vec<String>,
    search: Signal<String>,
    filter_tier: Signal<usize>,
    selected: Signal<Vec<String>>,
    on_refresh: EventHandler<()>,
    on_new: EventHandler<()>,
    on_bulk_enable: EventHandler<()>,
    on_bulk_disable: EventHandler<()>,
    on_bulk_clear: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            id: "groups-sec-filter",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                    span { class: "text-xs text-zinc-500", "按倍率分级或关键词筛选" }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                        "data-testid": "refresh-groups",
                        onclick: move |_| on_refresh.call(()),
                        "刷新"
                    }
                    button {
                        class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                        "data-testid": "new-group",
                        onclick: move |_| on_new.call(()),
                        "✚ 新建分组"
                    }
                }
            }

            input {
                class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                r#type: "text",
                "data-testid": "group-search",
                placeholder: "搜索分组标识或备注...",
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }

            // 分类胶囊
            div { class: "flex flex-wrap gap-2",
                SegmentedCapsule {
                    items: filter_options,
                    active: filter_tier(),
                    on_select: move |i: usize| filter_tier.set(i),
                }
            }

            // 批量多选区 (卡牌外): 列全部分组 chips, 点选加入/移出选中集合;
            // 选中数>0 时下方出现批量动作条 (批量启停 / 清除)。
            div { class: "space-y-2",
                p { class: "mb-1.5 text-[11px] text-zinc-500", "批量操作: 点选分组" }
                div { class: "flex flex-wrap gap-1.5",
                    "data-testid": "bulk-select",
                    for g in groups {
                        {
                            let key = g.key.clone();
                            let gn = g.name.clone();
                            let picked = selected.peek().contains(&key);
                            let cls = if picked {
                                "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                            } else {
                                "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                            };
                            rsx! {
                                button {
                                    class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {cls}",
                                    "data-testid": "bulk-select-chip",
                                    onclick: move |_| {
                                        let mut s = selected.peek().to_vec();
                                        if let Some(pos) = s.iter().position(|k| *k == key) {
                                            s.remove(pos);
                                        } else {
                                            s.push(key.clone());
                                        }
                                        selected.set(s);
                                    },
                                    "{gn}"
                                }
                            }
                        }
                    }
                }
                // 批量动作条: 勾选后出现
                if !selected().is_empty() {
                    div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/80 bg-zinc-950 px-3 py-2.5",
                        "data-testid": "bulk-bar",
                        span { class: "text-xs text-zinc-400", "已选 {selected().len()} 项" }
                        button {
                            class: "rounded-lg border border-emerald-700/50 bg-emerald-900/30 px-2.5 py-1 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-800/50",
                            "data-testid": "bulk-enable",
                            onclick: move |_| on_bulk_enable.call(()),
                            "批量启用"
                        }
                        button {
                            class: "rounded-lg border border-zinc-700/80 bg-zinc-800/60 px-2.5 py-1 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700",
                            "data-testid": "bulk-disable",
                            onclick: move |_| on_bulk_disable.call(()),
                            "批量停用"
                        }
                        button {
                            class: "rounded-lg border border-zinc-700/80 px-2.5 py-1 text-xs text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                            "data-testid": "bulk-clear",
                            onclick: move |_| on_bulk_clear.call(()),
                            "清除"
                        }
                    }
                }
            }
        }
    }
}
