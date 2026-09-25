//! 别名筛选与操作区组件(编号段 2):标题 + 刷新/新建按钮 + 搜索框 + 分级胶囊。
//!
//! search / filter_tier / reload 由页面持有(页面要用它们算 filtered、
//! 触发重拉),组件通过 Signal prop 就地读写;on_new(开弹窗)是跨组件
//! 交互,由页面闭包注入。组件内部零 use_signal。
//!
//! 【是什么】别名 tab 的编号段 2:区段标题 + 刷新/新建按钮 + 搜索框 + 按倍率分档的分级胶囊。
//!
//! 【做什么】承载四个筛选/操作入口(搜索词、分档、刷新、新建);不负责按条件过滤
//! 列表(页面算),不负责真正的拉取(由页面 effect 响应 `reload` 执行)。
//!
//! 【交互逻辑】用户操作 → 组件行为 → 数据交互:
//! - 在搜索框输入 → `oninput` 写 `search`(页面据此算 filtered),不发网络。
//! - 点分级胶囊某档 → `SegmentedCapsule::on_select` 写 `filter_tier`,不发网络。
//! - 点「刷新」→ `reload` 自增 1,页面 `use_effect` 监听到变化后重拉列表(发 GET)。
//! - 点「新建别名」→ 调 `on_new`(`MouseEvent`),由页面闭包打开弹窗,组件不自持弹窗状态。
//!
//! 【样式】外壳 `section#aliases-sec-filter`,class 为
//! `scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5`
//! (ScrollSpy 锚点 + 圆角描边面板,子项纵向 `gap-4`)。刷新按钮为描边幽灵样式
//! `border-zinc-700 bg-zinc-950` hover 转 `border-zinc-500`,新建按钮为白底
//! `bg-white text-zinc-900` hover `bg-zinc-200`;搜索框 `rounded-xl border-zinc-700/80
//! bg-zinc-950`,聚焦时 `focus:border-zinc-500`;胶囊行 `flex flex-wrap gap-2` 允许换行。
//!
//! 【子组件组成】`ui::SegmentedCapsule`(分档胶囊);按钮与输入框为原生元素,未抽组件。
//!
//! 【数据流】
//! - 对内(入):`search`(页面持有的搜索词)、`filter_tier`(0 全部 / 1 标准 / 2 自定 /
//!   3 免费)、`reload`(重拉计数)、`filter_options`(页面派生的含计数胶囊文案)、
//!   `on_new`(页面注入的开弹窗回调)。
//! - 对外(出):`search` / `filter_tier` / `reload` 三个 Signal 就地写回同一份页面状态;
//!   `on_new` 抛回页面,由页面 `open_new` 重置表单并置 `modal_state = New`。

use dioxus::prelude::*;

use crate::shared::{BTN_NEW_ALIAS, BTN_REFRESH, MSG_SEARCH_PLACEHOLDER, SEC_FILTER, SEC_FILTER_NOTE};
use ui::SegmentedCapsule;

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
        section { id: "aliases-sec-filter", class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            // 标题行 + 说明
            div { class: "flex items-center justify-between",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_FILTER}" }
                p { class: "text-xs text-zinc-500", "{SEC_FILTER_NOTE}" }
            }
            
            // 按钮行:刷新 + 新建
            div { class: "flex items-center gap-3",
                button {
                    class: "rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2 text-sm text-zinc-200 hover:border-zinc-500 transition-colors",
                    onclick: move |_| reload.set(reload() + 1),
                    "{BTN_REFRESH}"
                }
                button {
                    class: "rounded-xl bg-white text-zinc-900 px-4 py-2 text-sm font-medium hover:bg-zinc-200 transition-colors",
                    onclick: on_new,
                    "{BTN_NEW_ALIAS}"
                }
            }
            
            // 搜索框
            div { class: "relative",
                input {
                    class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 focus:border-zinc-500 focus:outline-none",
                    placeholder: MSG_SEARCH_PLACEHOLDER,
                    value: search(),
                    oninput: move |evt| search.set(evt.value()),
                }
            }
            
            // 分级胶囊
            SegmentedCapsule {
                options: filter_options,
                selected: filter_tier(),
                on_select: move |idx| filter_tier.set(idx),
            }
        }
    }
}