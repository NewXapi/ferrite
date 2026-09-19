//! 别名筛选与操作区(编号段 2):标题 + 刷新/新建按钮 + 搜索框 + 分级胶囊。
//!
//! search / filter_tier / reload 由页面持有(页面要用它们算 filtered、
//! 触发重拉),组件通过 Signal prop 就地读写;on_new(开弹窗)是跨组件
//! 交互,由页面闭包注入。组件内部零 use_signal。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use super::shared::SEC_FILTER;

/// 别名筛选与操作区。
#[component]
pub fn AliasesToolbarSection(
    /// 搜索词(页面据此算 filtered,Signal 双向绑定)
    search: Signal<String>,
    /// 状态分级档位(0 全部 / 1 标准 / 2 自定 / 3 免费)
    filter_tier: Signal<usize>,
    /// 重拉计数:刷新按钮 +1 触发页面 use_effect 重拉
    reload: Signal<u32>,
    /// 分级胶囊文案(含各档计数,页面派生)
    filter_options: Vec<String>,
    /// 「新建别名」:开弹窗,跨组件交互,由页面闭包处理
    on_new: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section {
            id: "aliases-sec-filter",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                    span { class: "text-xs text-zinc-500", "按倍率与资费规则快速筛选" }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                        "data-testid": "refresh-aliases",
                        onclick: move |_| reload.set(reload() + 1),
                        "刷新"
                    }
                    button {
                        class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                        onclick: on_new,
                        "✚ 新建别名"
                    }
                }
            }

            input {
                class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                r#type: "text",
                placeholder: "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...",
                value: "{search}",
                oninput: move |e| search.set(e.value()),
            }

            div { class: "flex flex-wrap gap-2",
                SegmentedCapsule {
                    items: filter_options,
                    active: filter_tier(),
                    on_select: move |i: usize| filter_tier.set(i),
                }
            }
        }
    }
}
