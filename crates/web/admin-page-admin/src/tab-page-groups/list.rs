//! 分组卡片网格区:四态分支(loading / error / empty / data) + 首卡示例
//! + `GroupCard` 网格。纯渲染组件:数据与写回回调由 page 注入。
//!
//! 删 / 启停 / 倍率三类卡片操作统一走 `on_write` 回传 `(key, WriteOp)`,
//! 由页面分派到对应写工厂;编辑走 `on_edit` 回传 key。

use dioxus::prelude::*;
use ui::GroupCard as PrototypeGroupCard;

use contract::api::admin::GroupDto;

use super::modal::GroupCard;
use super::shared::{SEC_LIST, WriteOp};

/// 分组列表(四态 + 卡片网格)。
///
/// - `filtered`:经页面筛选后的分组列表。
/// - `loading` / `err`:加载与错误态(二者优先于列表内容)。
/// - `on_edit`:点击卡片「编辑」时回传分组 key。
/// - `on_write`:删除 / 启停 / 倍率写回时回传 `(key, WriteOp)`。
/// - `on_retry`:错误态点「重试」时触发整页重拉。
#[component]
pub fn GroupsList(
    filtered: Vec<GroupDto>,
    loading: bool,
    err: Option<String>,
    on_edit: EventHandler<String>,
    on_write: EventHandler<(String, WriteOp)>,
    on_retry: EventHandler<()>,
) -> Element {
    rsx! {
        section { id: "groups-sec-list", class: "scroll-mt-8 space-y-4",
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                    if loading { "加载中…" } else { "{filtered.len()} 组" }
                }
            }

            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                    p { class: "text-sm text-red-300", "加载分组失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                        "data-testid": "retry-groups",
                        onclick: move |_| on_retry.call(()),
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "正在加载分组…" }
                }
            } else if filtered.is_empty() {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "没有匹配的分组" }
                }
            } else {
                // 首卡示例 (ui crate 的原型 GroupCard, 只读展示)
                if let Some(group) = filtered.first().cloned() {
                    {
                        let is_default = group.name == "default";
                        rsx! {
                            div {
                                class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                role: "region",
                                "aria-label": "新卡示例",
                                "data-testid": "group-card-prototype",
                                PrototypeGroupCard {
                                    group,
                                    is_default,
                                }
                            }
                        }
                    }
                }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    "data-testid": "groups-list",
                    for g in filtered {
                        {
                            let edit_key = g.key.clone();
                            let delete_key = g.key.clone();
                            let toggle_key = g.key.clone();
                            let ratio_key = g.key.clone();
                            let current_status = g.status;
                            let is_default = g.name == "default";
                            // 启用/停用:按当前 status 取目标值 (1↔2);
                            // target 在 group 值移入 rsx 前算好, 避免 move 后再借用
                            let toggle_target = if current_status == 1 { 2 } else { 1 };
                            let on_toggle_status = move |_| {
                                on_write.call((
                                    toggle_key.clone(),
                                    WriteOp::ToggleStatus(toggle_target),
                                ));
                            };
                            // 倍率滑条松手写回: 复用同一写工厂, 就地更新本地 ratio
                            let on_ratio_drag = move |v: f64| {
                                on_write.call((ratio_key.clone(), WriteOp::SetRatio(v)));
                            };
                            rsx! {
                                GroupCard {
                                    key: "{g.key}",
                                    group: g,
                                    is_default,
                                    on_edit: move |_| on_edit.call(edit_key.clone()),
                                    on_delete: move |_| {
                                        on_write.call((delete_key.clone(), WriteOp::Delete))
                                    },
                                    on_toggle_status,
                                    on_ratio_drag,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
