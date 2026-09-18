//! 订阅套餐编辑/新建弹窗(三 Tab:基本信息 / 规则与周期 / 第三方支付配置)。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),输入
//! 直接写回页面级 signal;提交 / 关闭事件通过 `on_submit` / `on_cancel`
//! 抛给页面,校验与写回在 `page` 的 `commit` 里。
//! 三个 Tab 体各自独立成组件(`SubscriptionBasicTab` / `SubscriptionRulesTab`
//! / `SubscriptionPaymentTab`),弹窗只保留外壳 + Tab 切换条 + 底部按钮。

use dioxus::prelude::*;

use super::shared::ToggleSwitch;
use crate::state::GroupRow;

/// 订阅套餐编辑/新建弹窗
#[component]
pub fn SubscriptionFormModal(
    editing_idx: Signal<Option<usize>>,
    modal_tab: Signal<u8>,
    groups: Signal<Vec<GroupRow>>,
    f_title: Signal<String>,
    f_subtitle: Signal<String>,
    f_price: Signal<String>,
    f_quota: Signal<String>,
    f_currency_price: Signal<String>,
    f_payment_method: Signal<String>,
    f_group: Signal<String>,
    f_downgrade_group: Signal<String>,
    f_limit: Signal<String>,
    f_sort: Signal<String>,
    f_enabled: Signal<bool>,
    f_allow_redeem: Signal<bool>,
    f_allow_wallet: Signal<bool>,
    f_period_val: Signal<String>,
    f_period_unit: Signal<String>,
    f_reset_cycle: Signal<String>,
    f_stripe_id: Signal<String>,
    f_creem_id: Signal<String>,
    f_waffo_id: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-2xl rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto scroll-subtle",
                onclick: move |e| e.stop_propagation(),

                // 弹窗头部
                div { class: "flex items-start justify-between",
                    div {
                        h3 { class: "text-lg font-bold text-zinc-100",
                            if editing_idx().is_some() { "更新套餐信息" } else { "新建订阅套餐" }
                        }
                        p { class: "mt-0.5 text-xs text-zinc-400", "修改现有订阅套餐的配置" }
                    }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                        onclick: move |_| on_cancel.call(()),
                        "✕"
                    }
                }

                // 弹窗内部 Tab 切换条
                div { class: "flex items-center gap-2 border-b border-zinc-800 pb-2 text-xs",
                    button {
                        class: if modal_tab() == 0 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                        onclick: move |_| modal_tab.set(0),
                        "基本信息"
                    }
                    button {
                        class: if modal_tab() == 1 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                        onclick: move |_| modal_tab.set(1),
                        "规则与周期"
                    }
                    button {
                        class: if modal_tab() == 2 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                        onclick: move |_| modal_tab.set(2),
                        "第三方支付配置"
                    }
                }

                // ---- Tab 0: 基本信息 (Image #6) ----
                if modal_tab() == 0 {
                    SubscriptionBasicTab {
                        groups,
                        f_title, f_subtitle, f_price, f_quota, f_currency_price,
                        f_payment_method, f_group, f_downgrade_group, f_limit, f_sort,
                    }
                }

                // ---- Tab 1: 规则与周期 (Image #7) ----
                if modal_tab() == 1 {
                    SubscriptionRulesTab {
                        f_enabled, f_allow_redeem, f_allow_wallet,
                        f_period_val, f_period_unit, f_reset_cycle,
                    }
                }

                // ---- Tab 2: 第三方支付配置 (Image #8) ----
                if modal_tab() == 2 {
                    SubscriptionPaymentTab {
                        f_stripe_id, f_creem_id, f_waffo_id,
                    }
                }

                // 弹窗底部操作按钮
                div { class: "flex items-center justify-end gap-3 pt-3 border-t border-zinc-800",
                    button {
                        class: "rounded-xl border border-zinc-700 px-4 py-2 text-xs font-medium text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                        onclick: move |_| on_cancel.call(()),
                        "关闭"
                    }
                    button {
                        class: "rounded-xl bg-amber-400 px-5 py-2 text-xs font-bold text-zinc-950 hover:bg-amber-300 transition-colors shadow-lg shadow-amber-500/10",
                        onclick: move |_| on_submit.call(()),
                        "保存更改"
                    }
                }
            }
        }
    }
}

/// Tab 0:基本信息(标题 / 副标题 / 价格 / 额度 / 站内支付 / 分组 / 限购 / 排序)
#[component]
fn SubscriptionBasicTab(
    groups: Signal<Vec<GroupRow>>,
    f_title: Signal<String>,
    f_subtitle: Signal<String>,
    f_price: Signal<String>,
    f_quota: Signal<String>,
    f_currency_price: Signal<String>,
    f_payment_method: Signal<String>,
    f_group: Signal<String>,
    f_downgrade_group: Signal<String>,
    f_limit: Signal<String>,
    f_sort: Signal<String>,
) -> Element {
    rsx! {
        div { class: "space-y-4 pt-1",
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", "套餐标题" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                    value: "{f_title()}",
                    placeholder: "例如：开拓的封赏",
                    oninput: move |e| f_title.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", "套餐副标题" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                    value: "{f_subtitle()}",
                    placeholder: "向你们致敬，向外开拓的勇士们！",
                    oninput: move |e| f_subtitle.set(e.value()),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "套餐价格 ($)" }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_price()}",
                        oninput: move |e| f_price.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", "用户购买该套餐需支付的金额，具体币种由支付渠道决定" }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "额度 (点)" }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_quota()}",
                        oninput: move |e| f_quota.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", "套餐包含的总额度，每个计费周期可用；0 表示不限量" }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "套餐价格（菌种）" }
                    input {
                        r#type: "number",
                        step: "0.1",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_currency_price()}",
                        oninput: move |e| f_currency_price.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", "最小单位 0.1。仅当支付方式包含它时才生效。" }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "站内支付方式" }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_payment_method()}",
                        oninput: move |e| f_payment_method.set(e.value()),
                        option { value: "仅扣菌种", "仅扣菌种" }
                        option { value: "允许余额兑换", "允许余额兑换" }
                        option { value: "无限制", "无限制" }
                    }
                    p { class: "text-[11px] text-zinc-500", "只影响站内货币，不影响第三方支付渠道。" }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "升级分组" }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_group()}",
                        oninput: move |e| f_group.set(e.value()),
                        option { value: "不升级", "不升级" }
                        for g in groups.read().iter() {
                            option { value: "{g.name}", "{g.name}" }
                        }
                    }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "降级分组" }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_downgrade_group()}",
                        oninput: move |e| f_downgrade_group.set(e.value()),
                        option { value: "降级到购买前分组", "降级到购买前分组" }
                        option { value: "默认分组", "默认分组" }
                    }
                    p { class: "text-[11px] text-zinc-500", "订阅过期后降级到该分组" }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "限购" }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_limit()}",
                        oninput: move |e| f_limit.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", "0 表示不限" }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", "排序" }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_sort()}",
                        oninput: move |e| f_sort.set(e.value()),
                    }
                }
            }
        }
    }
}

/// Tab 1:规则与周期(三个开关 + 有效期 + 额度重置)
#[component]
fn SubscriptionRulesTab(
    f_enabled: Signal<bool>,
    f_allow_redeem: Signal<bool>,
    f_allow_wallet: Signal<bool>,
    f_period_val: Signal<String>,
    f_period_unit: Signal<String>,
    f_reset_cycle: Signal<String>,
) -> Element {
    rsx! {
        div { class: "space-y-4 pt-1",
            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                span { class: "text-sm text-zinc-200 font-medium", "启用状态" }
                ToggleSwitch { on: f_enabled(), on_toggle: move |_| f_enabled.set(!f_enabled()) }
            }
            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                span { class: "text-sm text-zinc-200 font-medium", "允许余额兑换" }
                ToggleSwitch { on: f_allow_redeem(), on_toggle: move |_| f_allow_redeem.set(!f_allow_redeem()) }
            }
            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                span { class: "text-sm text-zinc-200 font-medium", "额度用尽后允许使用钱包余额" }
                ToggleSwitch { on: f_allow_wallet(), on_toggle: move |_| f_allow_wallet.set(!f_allow_wallet()) }
            }

            // 有效期设置
            div { class: "pt-2 space-y-2",
                h4 { class: "text-xs font-semibold text-amber-400 flex items-center gap-1.5", "有效期设置" }
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", "有效期数值" }
                        input {
                            r#type: "number",
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_period_val()}",
                            oninput: move |e| f_period_val.set(e.value()),
                        }
                    }
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", "有效期单位" }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_period_unit()}",
                            oninput: move |e| f_period_unit.set(e.value()),
                            option { value: "小时", "小时" }
                            option { value: "天", "天" }
                            option { value: "个月", "个月" }
                            option { value: "年", "年" }
                            option { value: "秒", "秒" }
                        }
                    }
                }
            }

            // 额度重置
            div { class: "pt-2 space-y-2",
                h4 { class: "text-xs font-semibold text-emerald-400 flex items-center gap-1.5", "额度重置" }
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", "重置周期" }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_reset_cycle()}",
                            oninput: move |e| f_reset_cycle.set(e.value()),
                            option { value: "不重置", "不重置" }
                            option { value: "每天", "每天" }
                            option { value: "每周", "每周" }
                            option { value: "每月", "每月" }
                            option { value: "自定义", "自定义" }
                        }
                    }
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", "自定义秒数" }
                        input {
                            r#type: "number",
                            disabled: f_reset_cycle() != "自定义",
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 disabled:opacity-40 focus:border-zinc-500 outline-none",
                            value: "0",
                        }
                    }
                }
            }
        }
    }
}

/// Tab 2:第三方支付配置(Stripe / Creem / Waffo Pancake)
#[component]
fn SubscriptionPaymentTab(
    f_stripe_id: Signal<String>,
    f_creem_id: Signal<String>,
    f_waffo_id: Signal<String>,
) -> Element {
    rsx! {
        div { class: "space-y-4 pt-1",
            div { class: "rounded-xl border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-300 leading-relaxed",
                "使用此套餐的标题和价格，在已保存的店铺中创建 Pancake 产品。需要先在支付设置中完整配置 Waffo Pancake。"
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", "Stripe Price ID" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_stripe_id()}",
                    placeholder: "price_1M...",
                    oninput: move |e| f_stripe_id.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", "Creem Product ID" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_creem_id()}",
                    placeholder: "prod_...",
                    oninput: move |e| f_creem_id.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", "Waffo Pancake Product ID" }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_waffo_id()}",
                    placeholder: "选择产品或输入 ID",
                    oninput: move |e| f_waffo_id.set(e.value()),
                }
            }
        }
    }
}
