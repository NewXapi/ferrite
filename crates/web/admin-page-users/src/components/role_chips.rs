//! 角色权限选择器:单选 chips 面板(替代原生 `<select>`,与分组 chips 同款外观)。
//!
//! 角色是单值枚举(1 | 10 | 100),选中即替换;候选取 `fetch_roles()`
//! 的非「全部」项,标签与筛选胶囊/卡片徽标同源。

use dioxus::prelude::*;

use crate::api;
use ui::FieldPanel;

use super::Chip;

/// 角色权限单选 chips。
///
/// 【是什么】单选胶囊面板,替代原生 `<select>`,外观与分组 chips 同款。
///
/// 【做什么】渲染候选角色(取 `fetch_roles()` 非「全部」项)与选中态,单值替换上报。
///
/// 【交互逻辑】点 chip → `on_change(value)`(角色单选替换);无其他交互。
///
/// 【样式】`rounded-xl border-zinc-700 bg-zinc-950` 面板 + 反色选中胶囊;
/// `role="group"` + `aria-label` + 每颗 chip `aria-pressed`。
///
/// 【子组件组成】共享 `Chip`(每角色一颗)。
///
/// 【数据流】
/// - 对内(入):`role`(当前角色 Signal);候选项来自 `crate::api::fetch_roles()`。
/// - 对外(出):`on_change`(新角色值)。
#[component]
pub fn RoleChips(role: Signal<u16>, on_change: EventHandler<u16>) -> Element {
    let items: Vec<(&'static str, u16)> = api::fetch_roles()
        .iter()
        .filter(|(_, v)| *v != 0)
        .copied()
        .collect();

    rsx! {
        // 角色面板:单选 chips,点选即替换(面板壳收敛到 ui::FieldPanel)
        // DONE: focus-within 面板壳收敛到 ui::FieldPanel(样式 token 全仓单点,主题化改 FIELD_PANEL 常量) in=demo by=agent
        FieldPanel {
            testid: "user-role-chips".to_string(),
            role: "group",
            aria_label: "角色权限选择".to_string(),
            div { class: "flex flex-wrap gap-1.5",
                for (label, value) in items {
                    Chip {
                        label: label.to_string(),
                        selected: role() == value,
                        testid: format!("user-role-chip-{value}"),
                        on_press: move |_| on_change.call(value),
                    }
                }
            }
        }
    }
}
