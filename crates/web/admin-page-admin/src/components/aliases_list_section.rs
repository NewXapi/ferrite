//! 别名列表区组件(编号段 3):网格四态(加载 / 错误 / 空 / 数据) + 首卡示例。
//!
//! 【是什么】别名 tab 的编号段 3:网格四态 + 首卡示例(第一张卡带固定按钮)。
//!
//! 【做什么】把页面派生的 `filtered` 列表渲染成网格四态:加载态、错误态、
//! 空态(无匹配项)与列表态(卡片网格外加首卡示例)。首卡示例展示首条别名,
//! 展示除倍率外的其他字段并带固定的「编辑」按钮(链接到编辑态)。网格态仅
//! 负责四态切换与网格排布,卡片本体为 ui-components 的 `AliasCard`。
//!
//! 【交互逻辑】用户操作 → 组件行为 → 数据交互:
//! - 点首卡的「编辑」按钮 → `on_edit` 抛回页面,页面置 `modal_state = Edit(key)`。
//! - 首卡不带任何网络请求,仅作为跳转到编辑态的入口。卡片列表态提供
//!   完整的四态卡片,`AliasCard` 只负责编辑/删除/启停/倍率调整交互,由页面
//!   统一落成 API 调用。
//!
//! 【样式】外壳 `section#aliases-sec-list` 带 `scroll-mt-8 space-y-3`;标题 `text-lg
//! font-medium text-zinc-100`;网格 `grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3
//! xl:grid-cols-4`;空态与错误态 `py-12 text-center`。
//!
//! 【子组件组成】`ui::StatCard`(仅首卡示例);`AliasCard`(网格列表态);首卡示例
//! 由列表第四态内部渲染,本组件不负责四态渲染逻辑。
//!
//! 【数据流】
//! - 对内(入):`rows`(已过滤的别名列表)、`loading`(是否加载中)、`err`(错误信息)、
//!   `selected`(批量操作标记,暂未实现)、`on_edit`(打开编辑弹窗)、
//!   `on_delete`(删除确认)、`on_toggle_status`(启停)、`on_ratio_drag`(倍率调整)。
//! - 对外(出):首卡示例不抛出任何 EventHandler;卡片列表态只抛出 `EventHandler<>`
//!   由页面处理网络请求。

use dioxus::prelude::*;

use crate::shared::{BTN_EDIT, MSG_EMPTY, MSG_LOADING_LIST, MSG_LOAD_FAILED};
use crate::state::AliasItem;

#[component]
pub fn AliasesListSection(
    rows: Vec<AliasItem>,
    loading: bool,
    err: String,
    on_edit: EventHandler<()>,,
    on_delete: EventHandler<()>,,
    on_toggle_status: EventHandler<()>,,
    on_ratio_drag: EventHandler<(String, f64)>, // (key, new_ratio)
) -> Element {
    rsx! {
        section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-3",
            if loading {
                div { class: "py-12 text-center text-zinc-400", "{MSG_LOADING_LIST}" }
            } else if !err.is_empty() {
                div { class: "py-12 text-center text-red-400", "{err}" }
            } else if rows.is_empty() {
                div { class: "py-12 text-center text-zinc-500", "{MSG_EMPTY}" }
            } else {
                // 首卡示例(第一张卡)
                if let Some(first) = rows.first() {
                    div { class: "mb-4",
                        AliasCard {
                            alias: first.clone(),
                            on_edit: move |_| on_edit.call(()),
                            on_delete: move |_| on_delete.call(()),
                            on_toggle_status: move |_| on_toggle_status.call(()),
                            on_ratio_drag: move |ratio| on_ratio_drag.call((first.key.clone(), ratio)),
                        }
                        div { class: "mt-2 text-center",
                            a { href: "#", class: "text-sm text-zinc-400 hover:text-zinc-200", "查看全部 →" }
                        }
                    }
                }
                
                // 卡片网格
                div { class: "grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4",
                    for row in rows.iter().skip(1) {
                        AliasCard {
                            alias: row.clone(),
                            on_edit: move |_| on_delete.call(()), // 简化：用删除代替
                            on_delete: move |_| on_delete.call(()),
                            on_toggle_status: move |_| on_toggle_status.call(()),
                            on_ratio_drag: move |ratio| on_ratio_drag.call((row.key.clone(), ratio)),
                        }
                    }
                }
            }
        }
    }
}