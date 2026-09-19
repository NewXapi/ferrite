//! 角色权限选择器:单选 chips 面板(替代原生 `<select>`,与分组 chips 同款外观)。
//!
//! 角色是单值枚举(1 | 10 | 100),选中即替换;候选取 `fetch_roles()`
//! 的非「全部」项,标签与筛选胶囊/卡片徽标同源。

use dioxus::prelude::*;

use crate::api;

#[component]
pub fn RoleChips(role: Signal<u16>, on_change: EventHandler<u16>) -> Element {
    let items: Vec<(&'static str, u16)> = api::fetch_roles()
        .iter()
        .filter(|(_, v)| *v != 0)
        .copied()
        .collect();

    rsx! {
        div {
            class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-2.5 focus-within:border-zinc-500",
            "data-testid": "user-role-chips",
            role: "group",
            "aria-label": "角色权限选择",

            div { class: "flex flex-wrap gap-1.5",
                for (label, value) in items {
                    {
                        let on = role() == value;
                        let tone = if on {
                            "border-zinc-100 bg-zinc-100 text-zinc-900"
                        } else {
                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                        };
                        rsx! {
                            button {
                                class: "rounded-full border px-2.5 py-0.5 text-xs font-medium transition-colors {tone}",
                                "data-testid": "user-role-chip-{value}",
                                "aria-pressed": "{on}",
                                onclick: move |_| on_change.call(value),
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
    }
}
