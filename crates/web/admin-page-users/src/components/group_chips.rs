//! 生效分组选择器:形似输入框的覆盖面板,chips 多选,点选切换选中态。
//!
//! 后端 `auth_users.groups` 是 TEXT[] 多值(`set_groups` 整体替换);
//! 选中集合在父级 signal 里维护,本组件只负责渲染与上报增删。
//! 首个选中项即生效分组(计费组倍率 / token 未设组时的回落值)。
//!
//! 分组列表由面板拉取后经 context 注入;本组件只读,不认识来源。
use dioxus::prelude::*;
use ui::FieldPanel;

use super::Chip;
use crate::shared::MSG_NO_GROUPS;

/// 单颗分组 chip(供 `GroupChips` 的列表循环复用)。
///
/// 【是什么】一颗可点选的分组胶囊:选中反色,未选灰边。
///
/// 【做什么】渲染 label 与选中态;点击时在本组件内推导 next 集合并上报。
/// 集合的增删推导收在这里(旧形状把 20 行推导内联在循环里,不可读)。
///
/// 【交互逻辑】点击 → 按「本 chip 的 value 是否已在集合」推导 next
/// (不在 = 新增,在 = 删除)→ `on_pick(next)`。
///
/// 【样式】选中反色由共享 `Chip` 渲染;`aria-pressed` 标注选中,testid 取分组名。
///
/// 【子组件组成】无。
///
/// 【数据流】
/// - 对内(入):`label` / `value` / `group`(选中集合 Signal)。
/// - 对外(出):`on_pick`(推导后的完整新集合)。
#[component]
pub fn GroupChip(
    label: String,
    value: String,
    group: Signal<Vec<String>>,
    on_pick: EventHandler<Vec<String>>,
) -> Element {
    let selected = group().iter().any(|g| g == &value);

    rsx! {
        Chip {
            label,
            selected,
            testid: format!("user-group-chip-{value}"),
            // 本 chip 不在集合 = 新增;在 = 删除。推导收在本组件内,循环体只剩组件调用。
            on_press: move |_| {
                let mut next: Vec<String> = group()
                    .iter()
                    .filter(|g| *g != &value)
                    .cloned()
                    .collect();
                if next.len() == group().len() {
                    next.push(value.clone());
                }
                on_pick.call(next);
            },
        }
    }
}

/// 生效分组多选 chips。
///
/// 【是什么】形似输入框的覆盖面板,圆角胶囊 chips 多选;首个选中项即生效分组
/// (计费组倍率 / token 未设组时的回落值)。
///
/// 【做什么】渲染候选分组与选中态、上报增删后的完整集合;不管理集合本身
/// (父级 signal)、不拉分组列表(context 注入,本组件只读,不认识来源)。
///
/// 【交互逻辑】点 chip → 由 `GroupChip` 推导 next → 页面 signal 更新;
/// 列表为空时渲染空态文案。
///
/// 【样式】`rounded-xl border-zinc-700 bg-zinc-950` 面板 + 选中反色胶囊;
/// `role="group"` + `aria-label` + 每颗 chip `aria-pressed` + testid 取分组名。
///
/// 【子组件组成】`GroupChip`(每分组一颗)。
///
/// 【数据流】
/// - 对内(入):`group`(选中集合 Signal);候选列表来自 context。
/// - 对外(出):`on_change`(完整新集合)。
#[component]
pub fn GroupChips(group: Signal<Vec<String>>, on_change: EventHandler<Vec<String>>) -> Element {
    // 分组列表由面板拉取后注入;本组件只读,不认识来源
    let groups = use_context::<Signal<Vec<(String, String)>>>();
    let list = groups();

    rsx! {
        // 面板壳收敛到 ui::FieldPanel(样式 token 全仓单点)
        // DONE: 同款 focus-within 面板壳一并收敛 ui::FieldPanel(与 role_chips 同一处常量) in=demo by=agent
        FieldPanel {
            testid: "user-group-chips".to_string(),
            role: "group",
            aria_label: "生效分组选择".to_string(),
            if list.is_empty() {
                p { class: "{ui::TYPE_DESC}", "{MSG_NO_GROUPS}" }
            } else {
                // chips 行:每分组一颗,点选切换
                div { class: "flex flex-wrap gap-1.5",
                    for (label, value) in list.iter() {
                        GroupChip {
                            label: label.clone(),
                            value: value.clone(),
                            group,
                            on_pick: on_change,
                        }
                    }
                }
            }
        }
    }
}
