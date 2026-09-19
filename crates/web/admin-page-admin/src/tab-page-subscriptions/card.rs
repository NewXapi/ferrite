//! 订阅套餐卡片:单张套餐的概览(ID 徽标 + 标题 + 状态/分组徽标 + 开关 +
//! 编辑/删除 + 副标题 + 五格指标条 + Stripe/Creem 徽标)。
//!
//! 纯展示组件:数据以值传入(`PlanRow`),编辑 / 启停 / 删除事件通过
//! `on_edit` / `on_toggle` / `on_delete` 抛回页面(传入 plans 列表中的下标),
//! 写回逻辑在 `page` 的 `SubscriptionsPage` 里。

use dioxus::prelude::*;

use super::shared::{
    BTN_EDIT, LBL_DISABLED, LBL_ENABLED, LBL_GROUP_PREFIX, LBL_PAY_CHANNEL, LBL_PERIOD, LBL_PRICE,
    LBL_QUOTA, LBL_RESET_CYCLE, LBL_UNLIMITED, OPT_NO_UPGRADE, ToggleSwitch,
};
use crate::state::PlanRow;

/// 订阅套餐卡片
///
/// 【是什么】单张订阅套餐的概览卡:ID 徽标 + 标题 + 状态/分组徽标 + 开关 +
/// 编辑/删除 + 副标题 + 五格指标条 + 第三方配置徽标。
///
/// 【做什么】按传入的 `PlanRow` 值渲染一张卡;不负责写回(启停/编辑/删除
/// 均由回调抛回页面)、不负责列表容器与弹窗。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点开关 `ToggleSwitch` → `on_toggle.call(index)`;点「编辑」→
///   `on_edit.call(index)`;点「✕」→ `on_delete.call(index)`。
///   三个回调都只带本卡下标,由页面决定改 `plans` 哪一行。
/// - 价格/额度/有效期等派生串(如 `quota <= 0` 时显示「无限制」)在渲染前
///   算好,纯展示。
/// 数据交互:本组件**不发任何网络请求**。
///
/// 【样式】卡片 `group flex flex-col rounded-xl border border-zinc-800
/// bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700
/// hover:bg-zinc-900/90 shadow-md`;头部 `flex flex-wrap items-start
/// justify-between gap-2.5`;ID 徽标 `rounded-md border border-zinc-700/80
/// bg-zinc-800 font-mono`;状态徽标按 `enabled` 切绿/灰两套圆角 pill;
/// 指标条 `grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3 pt-3
/// border-t border-zinc-800/70`(手机 2 / sm 3 / md 5 列)。
///
/// 【子组件组成】`ToggleSwitch`(启停开关,来自 `shared.rs`);其余为原生元素。
///
/// 【数据流】
/// - 对内(入):`plan`(该行 `PlanRow` 全量数据,页面 `plans` signal 中一行
///   的克隆)、`index`(在页面 `plans` 列表中的下标,回调原样回传)。
/// - 对外(出):`on_edit(index)` → 页面 `open_edit`(开弹窗回填);
///   `on_toggle(index)` → 页面就地取反 `plans[i].enabled`;
///   `on_delete(index)` → 页面 `plans.remove(i)`。
#[component]
pub fn PlanCard(
    plan: PlanRow,
    /// 在页面 plans 列表中的下标,回调原样回传
    index: usize,
    on_edit: EventHandler<usize>,
    on_toggle: EventHandler<usize>,
    on_delete: EventHandler<usize>,
) -> Element {
    let title_txt = plan.title.clone();
    let sub_txt = plan.subtitle.clone();
    let price_str = format!("${:.2}", plan.price);
    let quota_str = if plan.quota <= 0.0 {
        LBL_UNLIMITED.to_string()
    } else {
        format!("{}", plan.quota)
    };
    let period_str = format!("{} {}", plan.period_val, plan.period_unit);

    rsx! {
        div {
            key: "{plan.id}",
            class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/90 shadow-md",

            // 卡片头部行: ID + 标题 + 状态/分组徽标 + 操作按钮
            div { class: "flex flex-wrap items-start justify-between gap-2.5",
                div { class: "flex items-center gap-2.5 min-w-0 flex-1",
                    span { class: "shrink-0 rounded-md border border-zinc-700/80 bg-zinc-800 px-2 py-0.5 text-xs font-mono font-bold text-zinc-300",
                        "#{plan.id}"
                    }
                    h3 { class: "truncate text-base font-bold text-zinc-100", "{title_txt}" }
                    span {
                        class: if plan.enabled { "rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400" } else { "rounded-full border border-zinc-700 bg-zinc-800/80 px-2.5 py-0.5 text-[11px] font-medium text-zinc-500" },
                        if plan.enabled { {LBL_ENABLED} } else { {LBL_DISABLED} }
                    }
                    if !plan.group.is_empty() && plan.group != OPT_NO_UPGRADE {
                        span { class: "rounded-full border border-sky-500/30 bg-sky-500/10 px-2.5 py-0.5 text-[11px] font-medium text-sky-400 uppercase",
                            {LBL_GROUP_PREFIX} "{plan.group}"
                        }
                    }
                }
                div { class: "flex items-center gap-2 shrink-0",
                    ToggleSwitch {
                        on: plan.enabled,
                        on_toggle: move |_| on_toggle.call(index),
                    }
                    button {
                        class: "rounded-lg border border-zinc-700 bg-zinc-800 px-2.5 py-1 text-xs text-zinc-200 transition-colors hover:bg-zinc-700 hover:text-white",
                        onclick: move |_| on_edit.call(index),
                        {BTN_EDIT}
                    }
                    button {
                        class: "rounded-lg border border-red-900/50 bg-red-950/20 px-2 py-1 text-xs text-red-400 transition-colors hover:bg-red-900/30 hover:text-red-300",
                        onclick: move |_| on_delete.call(index),
                        "✕"
                    }
                }
            }

            // 副标题
            if !sub_txt.is_empty() {
                p { class: "mt-1.5 text-xs text-zinc-400 leading-relaxed", "{sub_txt}" }
            }

            // 关键指标条 (对标 Image #5 字段)
            div { class: "mt-3.5 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3 pt-3 border-t border-zinc-800/70 text-xs",
                div {
                    span { class: "text-[11px] text-zinc-500 block", {LBL_PRICE} }
                    span { class: "font-mono font-bold text-sm text-emerald-400", "{price_str}" }
                }
                div {
                    span { class: "text-[11px] text-zinc-500 block", {LBL_PERIOD} }
                    span { class: "font-medium text-zinc-200", "{period_str}" }
                }
                div {
                    span { class: "text-[11px] text-zinc-500 block", {LBL_QUOTA} }
                    span { class: "font-mono font-semibold text-amber-300", "{quota_str}" }
                }
                div {
                    span { class: "text-[11px] text-zinc-500 block", {LBL_PAY_CHANNEL} }
                    span { class: "text-zinc-300 font-medium", "{plan.payment_method}" }
                }
                div {
                    span { class: "text-[11px] text-zinc-500 block", {LBL_RESET_CYCLE} }
                    span { class: "text-zinc-400", "{plan.reset_cycle}" }
                }
            }

            // 第三方配置徽标展示
            if !plan.stripe_price_id.is_empty() || !plan.creem_product_id.is_empty() {
                div { class: "mt-2.5 flex flex-wrap gap-2 text-[10px] text-zinc-500 font-mono",
                    if !plan.stripe_price_id.is_empty() {
                        span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Stripe: {plan.stripe_price_id}" }
                    }
                    if !plan.creem_product_id.is_empty() {
                        span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Creem: {plan.creem_product_id}" }
                    }
                }
            }
        }
    }
}
