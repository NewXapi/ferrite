//! 分组 + 倍率页:三个面板(总览 / 新增编辑 / 倍率速览)。
//! 数据同 `state::EntityStore`,与拓扑、别名页实时同步。

use dioxus::prelude::*;

use crate::entities::InputCell;
use crate::pages::{GhostBtn, GridShell, Panel, PushBtn};
use crate::state::{EntityStore, GroupRow};

#[component]
pub fn GroupsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut groups = store.groups;
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut mult = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        let m = mult.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        match *editing.peek() {
            Some(i) => {
                groups.write()[i] = GroupRow {
                    name: n,
                    display: d,
                    multiplier: m,
                };
            }
            None => groups.write().push(GroupRow {
                name: n,
                display: d,
                multiplier: m,
            }),
        }
        name.set(String::new());
        display.set(String::new());
        mult.set(String::new());
        editing.set(None);
    };

    rsx! {
        GridShell {
            // 面板 1:总览(点行载入编辑;✕ 删除)
            Panel {
                title: "分组总览",
                hint: "点行载入编辑;倍率就是 new-api 的 group_ratio",
                if groups.read().is_empty() {
                    p { class: "text-[11px] text-zinc-600", "还没有分组" }
                } else {
                    div { class: "space-y-1.5",
                        for (i, g) in groups.read().iter().enumerate() {
                            {
                                let disp = if g.display.is_empty() {
                                    String::new()
                                } else {
                                    g.display.clone()
                                };
                                let tone = if editing() == Some(i) {
                                    "border-zinc-600 bg-zinc-900"
                                } else {
                                    "border-zinc-800 bg-zinc-950/60 hover:border-zinc-700"
                                };
                                rsx! {
                                    div { class: "rounded-lg border p-2 {tone}",
                                        button {
                                            class: "flex w-full items-baseline justify-between gap-2 text-left",
                                            onclick: move |_| {
                                                let g = groups.read()[i].clone();
                                                name.set(g.name);
                                                display.set(g.display);
                                                mult.set(format!("{}", g.multiplier));
                                                editing.set(Some(i));
                                            },
                                            span { class: "truncate text-xs text-zinc-200", "{g.name}" }
                                            span { class: "shrink-0 text-[11px] text-zinc-500",
                                                "{disp}"
                                            }
                                        }
                                        div { class: "mt-1 flex items-center justify-between",
                                            span { class: "text-[11px] text-zinc-500", "倍率 ×{g.multiplier}" }
                                            button {
                                                class: "text-[11px] text-red-500 hover:text-red-400",
                                                onclick: move |_| {
                                                    groups.write().remove(i);
                                                    if editing() == Some(i) { editing.set(None); }
                                                },
                                                "✕"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 面板 2:新增 / 编辑
            Panel {
                title: "新增 / 编辑分组",
                hint: "名称为空不提交;倍率 ≥ 0",
                div { class: "space-y-2",
                    InputCell { label: "分组名", value: name, placeholder: "vip", grow: true }
                    InputCell { label: "展示名", value: display, placeholder: "VIP(可选)", grow: true }
                    InputCell { label: "倍率", value: mult, placeholder: "1.0" }
                    div { class: "flex gap-2 pt-1",
                        PushBtn { label: "保存", on_click: commit }
                        if editing().is_some() {
                            GhostBtn {
                                label: "取消",
                                on_click: move |_| {
                                    editing.set(None);
                                    name.set(String::new());
                                    display.set(String::new());
                                    mult.set(String::new());
                                },
                            }
                        }
                    }
                }
            }

            // 面板 3:倍率速览
            Panel {
                title: "倍率速览",
                hint: "供订阅 / 渠道页对照",
                div { class: "space-y-1.5",
                    for g in groups.read().iter() {
                        div { class: "flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-950/60 px-2.5 py-1.5",
                            span { class: "truncate text-xs text-zinc-300", "{g.name}" }
                            span { class: "font-mono text-xs text-zinc-400", "×{g.multiplier}" }
                        }
                    }
                }
            }
        }
    }
}
