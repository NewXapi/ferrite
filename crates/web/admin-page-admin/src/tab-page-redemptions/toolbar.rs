//! 兑换码筛选与操作区(编号段 2):标题 + 生成按钮 + 搜索框 + 状态胶囊。
//!
//! search / filter_tier / reload 由页面持有,组件通过 Signal prop 就地读写;
//! on_generate(开生成弹窗并预填表单)是跨组件交互,由页面闭包注入。
//! 组件内部零 use_signal。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use super::shared::SEC_FILTER;

/// 兑换码筛选与操作区。
#[component]
pub fn RedemptionsToolbarSection(
    /// 搜索词(页面据此算 filtered,Signal 双向绑定)
    search: Signal<String>,
    /// 状态分级档位(0 全部 / 1 未使用 / 2 已核销 / 3 已停用)
    filter_tier: Signal<usize>,
    /// 重拉计数:预留(本区暂无刷新按钮,与 siblings 保持同形签名)
    reload: Signal<u32>,
    /// 状态胶囊文案(含各档计数,页面派生)
    filter_options: Vec<String>,
    /// 「生成兑换码」:开生成弹窗,跨组件交互,由页面闭包处理
    on_generate: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        section {
            id: "reds-sec-filter",
            "data-testid": "redemptions-filter",
            role: "search",
            "aria-label": "兑换码筛选与操作",
            class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "flex items-center justify-between gap-3",
                div { class: "flex items-center gap-2",
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                    span { class: "text-xs text-zinc-500", "按状态分级筛选;停用后不可重新启用" }
                }
                button {
                    "data-testid": "generate-redemptions",
                    class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                    onclick: on_generate,
                    "✚ 生成兑换码"
                }
            }

            input {
                "data-testid": "redemptions-search",
                class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                r#type: "text",
                placeholder: "搜索兑换码预览 (如 fx-086c****) ...",
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
