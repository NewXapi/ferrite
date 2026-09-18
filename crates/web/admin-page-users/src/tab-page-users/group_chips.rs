//! 生效分组选择器:形似输入框的覆盖面板,chips 多选,点选切换选中态。
//!
//! 后端 `auth_users.groups` 是 TEXT[] 多值(`set_groups` 整体替换);
//! 选中集合在父级 signal 里维护,本组件只负责渲染与上报增删。
//! 首个选中项即生效分组(计费组倍率 / token 未设组时的回落值)。
//!
//! 分组列表由面板拉取后经 context 注入;本组件只读,不认识来源。

use dioxus::prelude::*;

#[component]
pub fn GroupChips(group: Signal<Vec<String>>, on_change: EventHandler<Vec<String>>) -> Element {
    // 分组列表由面板拉取后注入;本组件只读,不认识来源
    let groups = use_context::<Signal<Vec<(String, String)>>>();
    let list = groups();

    rsx! {
        div {
            class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2.5 focus-within:border-zinc-500",
            "data-testid": "user-group-chips",
            role: "group",
            "aria-label": "生效分组选择",

            if list.is_empty() {
                p { class: "text-xs text-zinc-500", "暂无分组(后端 /api/group 为空)" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for (label, value) in list.iter() {
                        {
                            let on = group().iter().any(|g| g == value);
                            let tone = if on {
                                "border-zinc-100 bg-zinc-100 text-zinc-900"
                            } else {
                                "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                            };
                            let v = value.clone();
                            rsx! {
                                button {
                                    class: "rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors {tone}",
                                    "data-testid": "user-group-chip-{value}",
                                    "aria-pressed": "{on}",
                                    onclick: move |_| {
                                        let mut next: Vec<String> = group()
                                            .iter()
                                            .filter(|g| *g != &v)
                                            .cloned()
                                            .collect();
                                        if next.len() == group().len() {
                                            // 原集合不含本项 = 新增
                                            next.push(v.clone());
                                        }
                                        on_change.call(next);
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
