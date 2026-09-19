//! 订阅套餐编辑/新建弹窗(三 Tab:基本信息 / 规则与周期 / 第三方支付配置)。
//!
//! 纯展示组件:表单状态以 `Signal` 注入(Signal 是可拷贝的全局句柄),输入
//! 直接写回页面级 signal;提交 / 关闭事件通过 `on_submit` / `on_cancel`
//! 抛给页面,校验与写回在 `page` 的 `commit` 里。
//! 三个 Tab 体各自独立成组件(`SubscriptionBasicTab` / `SubscriptionRulesTab`
//! / `SubscriptionPaymentTab`),弹窗只保留外壳 + Tab 切换条 + 底部按钮。

use dioxus::prelude::*;

use super::shared::{
    BTN_CLOSE, BTN_SAVE, FIELD_ALLOW_REDEEM, FIELD_ALLOW_WALLET, FIELD_CREEM_ID,
    FIELD_CURRENCY_PRICE, FIELD_DOWNGRADE_GROUP, FIELD_ENABLED, FIELD_GROUP, FIELD_LIMIT,
    FIELD_PAYMENT_METHOD, FIELD_PERIOD_SECTION, FIELD_PERIOD_UNIT, FIELD_PERIOD_VAL, FIELD_PLAN_SUBTITLE,
    FIELD_PLAN_TITLE, FIELD_PRICE, FIELD_QUOTA, FIELD_RESET_CYCLE, FIELD_RESET_SECS,
    FIELD_RESET_SECTION, FIELD_SORT, FIELD_STRIPE_ID, FIELD_WAFFO_ID, MSG_CURRENCY_PRICE_HINT,
    MSG_DOWNGRADE_HINT, MSG_LIMIT_HINT, MSG_PAYMENT_METHOD_HINT, MSG_PAYMENT_NOTE, MSG_PH_CREEM_ID,
    MSG_PH_PLAN_SUBTITLE, MSG_PH_PLAN_TITLE, MSG_PH_STRIPE_ID, MSG_PH_WAFFO_ID, MSG_PRICE_HINT,
    MSG_QUOTA_HINT, OPT_DEFAULT_GROUP, OPT_DOWNGRADE_PREV, OPT_NO_UPGRADE, OPT_PAY_ONLY_SPECIES,
    OPT_PAY_UNLIMITED, OPT_PAY_WALLET_EXCHANGE, OPT_RESET_CUSTOM, OPT_RESET_DAILY,
    OPT_RESET_MONTHLY, OPT_RESET_NEVER, OPT_RESET_WEEKLY, OPT_UNIT_DAY, OPT_UNIT_HOUR,
    OPT_UNIT_MONTH, OPT_UNIT_SECOND, OPT_UNIT_YEAR, TAB_BASIC, TAB_PAYMENT, TAB_RULES, TTL_EDIT,
    TTL_NEW, TTL_SUBTITLE, ToggleSwitch,
};
use crate::state::GroupRow;

/// 订阅套餐编辑/新建弹窗
///
/// 【是什么】编辑/新建订阅套餐的三 Tab 弹窗外壳:遮罩 + 头部 + Tab 切换条
/// + 三选一的 Tab 体 + 底部按钮。
///
/// 【做什么】按 `modal_tab` 切换 `SubscriptionBasicTab` / `SubscriptionRulesTab`
/// / `SubscriptionPaymentTab`;把页面注入的 21 个 `f_*` signal 原样透传给
/// 对应 Tab 体。不负责校验与写回(在页面 `commit`)、不负责开合(页面持有
/// `show_modal`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点遮罩或「✕」或「关闭」→ `on_cancel.call(())` 请页面关弹窗;弹窗体
///   上 `e.stop_propagation()` 防冒泡到遮罩被误关。
/// - 点三个 Tab 按钮 → 写本组件持有的 `modal_tab` signal(0/1/2)。
/// - 点「保存更改」→ `on_submit.call(())` 请页面 `commit` 写 `plans`。
/// 数据交互:本组件**不发网络请求**;输入框直接写页面注入的 `f_*` signal。
///
/// 【样式】遮罩 `fixed inset-0 z-50 flex items-center justify-center bg-black/70
/// p-4 backdrop-blur-sm`;弹窗体 `w-full max-w-2xl rounded-2xl border
/// border-zinc-800 bg-zinc-900 p-6 shadow-2xl space-y-5 max-h-[90vh]
/// overflow-y-auto scroll-subtle`;Tab 切换条 `border-b border-zinc-800 pb-2`,
/// 激活页签 `bg-zinc-800 font-semibold`;底部保存按钮 `bg-amber-400` 实底。
///
/// 【子组件组成】`SubscriptionBasicTab` / `SubscriptionRulesTab` /
/// `SubscriptionPaymentTab`(三个 Tab 体,同文件内定义)。
///
/// 【数据流】
/// - 对内(入):`editing_idx`(`Some` = 编辑态,决定标题文案)、`modal_tab`
///   (当前页签)、`groups`(可选升级分组候选)与 21 个 `f_*` 表单 signal,
///   全部由页面 `SubscriptionsPage` 持有并以 Signal prop 注入。
/// - 对外(出):`on_cancel(())` → 页面 `show_modal.set(false)`;
///   `on_submit(())` → 页面 `commit`。Signal 由 Tab 体内的输入框直接写回。
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
                            if editing_idx().is_some() { {TTL_EDIT} } else { {TTL_NEW} }
                        }
                        p { class: "mt-0.5 text-xs text-zinc-400", {TTL_SUBTITLE} }
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
                        {TAB_BASIC}
                    }
                    button {
                        class: if modal_tab() == 1 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                        onclick: move |_| modal_tab.set(1),
                        {TAB_RULES}
                    }
                    button {
                        class: if modal_tab() == 2 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                        onclick: move |_| modal_tab.set(2),
                        {TAB_PAYMENT}
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
                        {BTN_CLOSE}
                    }
                    button {
                        class: "rounded-xl bg-amber-400 px-5 py-2 text-xs font-bold text-zinc-950 hover:bg-amber-300 transition-colors shadow-lg shadow-amber-500/10",
                        onclick: move |_| on_submit.call(()),
                        {BTN_SAVE}
                    }
                }
            }
        }
    }
}

/// Tab 0:基本信息(标题 / 副标题 / 价格 / 额度 / 站内支付 / 分组 / 限购 / 排序)
///
/// 【是什么】弹窗第一个页签的表单体:套餐标题、副标题、两个价格、额度、
/// 站内支付方式、升级/降级分组、限购、排序。
///
/// 【做什么】渲染各字段标签 + 受控输入框 + 字段旁说明;不持状态、不校验、
/// 不提交(提交由弹窗底部按钮走页面 `commit`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 在任一 input/select 输入 → 直接写回对应的页面注入 signal(如
///   `f_title.set(e.value())`),无本地副本。
/// - 「升级分组」下拉的候选由 `groups` signal 读出,首项固定「不升级」。
/// 数据交互:纯本地 signal 写入,不发网络。
///
/// 【样式】Tab 体 `space-y-4 pt-1`;字段用 `label.block.space-y-1` 包裹,
/// 标签 `text-xs font-medium text-zinc-300`,说明 `text-[11px] text-zinc-500`;
/// 输入框统一 `w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5
/// py-2 text-sm focus:border-zinc-500 outline-none`;成对字段走
/// `grid grid-cols-1 sm:grid-cols-2 gap-4`。
///
/// 【子组件组成】无子组件:原生 `label` / `span` / `input` / `select` /
/// `option` / `p`。
///
/// 【数据流】
/// - 对内(入):`groups`(升级分组候选,页面 `store.groups`)与 `f_title` /
///   `f_subtitle` / `f_price` / `f_quota` / `f_currency_price` /
///   `f_payment_method` / `f_group` / `f_downgrade_group` / `f_limit` /
///   `f_sort` 十个表单 signal,均由页面持有。
/// - 对外(出):无 EventHandler;所有输入直接写回上述 Signal(页面 `commit`
///   后续读取它们组装 `PlanRow`)。
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
                span { class: "text-xs font-medium text-zinc-300", {FIELD_PLAN_TITLE} }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                    value: "{f_title()}",
                    placeholder: MSG_PH_PLAN_TITLE,
                    oninput: move |e| f_title.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", {FIELD_PLAN_SUBTITLE} }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                    value: "{f_subtitle()}",
                    placeholder: MSG_PH_PLAN_SUBTITLE,
                    oninput: move |e| f_subtitle.set(e.value()),
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_PRICE} }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_price()}",
                        oninput: move |e| f_price.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_PRICE_HINT} }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_QUOTA} }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_quota()}",
                        oninput: move |e| f_quota.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_QUOTA_HINT} }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_CURRENCY_PRICE} }
                    input {
                        r#type: "number",
                        step: "0.1",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_currency_price()}",
                        oninput: move |e| f_currency_price.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_CURRENCY_PRICE_HINT} }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_PAYMENT_METHOD} }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_payment_method()}",
                        oninput: move |e| f_payment_method.set(e.value()),
                        option { value: OPT_PAY_ONLY_SPECIES, {OPT_PAY_ONLY_SPECIES} }
                        option { value: OPT_PAY_WALLET_EXCHANGE, {OPT_PAY_WALLET_EXCHANGE} }
                        option { value: OPT_PAY_UNLIMITED, {OPT_PAY_UNLIMITED} }
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_PAYMENT_METHOD_HINT} }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_GROUP} }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_group()}",
                        oninput: move |e| f_group.set(e.value()),
                        option { value: OPT_NO_UPGRADE, {OPT_NO_UPGRADE} }
                        for g in groups.read().iter() {
                            option { value: "{g.name}", "{g.name}" }
                        }
                    }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_DOWNGRADE_GROUP} }
                    select {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_downgrade_group()}",
                        oninput: move |e| f_downgrade_group.set(e.value()),
                        option { value: OPT_DOWNGRADE_PREV, {OPT_DOWNGRADE_PREV} }
                        option { value: OPT_DEFAULT_GROUP, {OPT_DEFAULT_GROUP} }
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_DOWNGRADE_HINT} }
                }
            }
            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_LIMIT} }
                    input {
                        r#type: "number",
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                        value: "{f_limit()}",
                        oninput: move |e| f_limit.set(e.value()),
                    }
                    p { class: "text-[11px] text-zinc-500", {MSG_LIMIT_HINT} }
                }
                label { class: "block space-y-1",
                    span { class: "text-xs font-medium text-zinc-300", {FIELD_SORT} }
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
///
/// 【是什么】弹窗第二个页签:三个开关行(启用/允许余额兑换/额度用尽后允许
/// 用钱包余额)+ 有效期设置 + 额度重置两组字段。
///
/// 【做什么】渲染开关与两个分组的下拉/输入;不持状态、不校验、不提交。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 拨任一 `ToggleSwitch` → 写回对应 `f_enabled` / `f_allow_redeem` /
///   `f_allow_wallet` signal(取反)。
/// - 改有效期数值/单位、重置周期、自定义秒数 → 写回对应 signal。
/// 数据交互:纯本地 signal 写入,不发网络。注:「自定义秒数」输入框的 value
/// 固定为 "0" 且无 `oninput`,是否可编辑由重置周期是否「自定义」控制。
///
/// 【样式】Tab 体 `space-y-4 pt-1`;开关行 `flex items-center justify-between
/// py-2 border-b border-zinc-800/80`(左 `text-sm text-zinc-200 font-medium`,
/// 右开关);分组标题「有效期设置」「额度重置」分别用 `text-amber-400` /
/// `text-emerald-400` 的 `text-xs font-semibold`;两列字段走
/// `grid grid-cols-1 sm:grid-cols-2 gap-4`。
///
/// 【子组件组成】`ToggleSwitch`(三处启停开关)。
///
/// 【数据流】
/// - 对内(入):`f_enabled` / `f_allow_redeem` / `f_allow_wallet` /
///   `f_period_val` / `f_period_unit` / `f_reset_cycle` 六个页面持有的 signal。
/// - 对外(出):无 EventHandler;输入与开关直接写回上述 Signal。
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
                span { class: "text-sm text-zinc-200 font-medium", {FIELD_ENABLED} }
                ToggleSwitch { on: f_enabled(), on_toggle: move |_| f_enabled.set(!f_enabled()) }
            }
            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                span { class: "text-sm text-zinc-200 font-medium", {FIELD_ALLOW_REDEEM} }
                ToggleSwitch { on: f_allow_redeem(), on_toggle: move |_| f_allow_redeem.set(!f_allow_redeem()) }
            }
            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                span { class: "text-sm text-zinc-200 font-medium", {FIELD_ALLOW_WALLET} }
                ToggleSwitch { on: f_allow_wallet(), on_toggle: move |_| f_allow_wallet.set(!f_allow_wallet()) }
            }

            // 有效期设置
            div { class: "pt-2 space-y-2",
                h4 { class: "text-xs font-semibold text-amber-400 flex items-center gap-1.5", {FIELD_PERIOD_SECTION} }
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", {FIELD_PERIOD_VAL} }
                        input {
                            r#type: "number",
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_period_val()}",
                            oninput: move |e| f_period_val.set(e.value()),
                        }
                    }
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", {FIELD_PERIOD_UNIT} }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_period_unit()}",
                            oninput: move |e| f_period_unit.set(e.value()),
                            option { value: OPT_UNIT_HOUR, {OPT_UNIT_HOUR} }
                            option { value: OPT_UNIT_DAY, {OPT_UNIT_DAY} }
                            option { value: OPT_UNIT_MONTH, {OPT_UNIT_MONTH} }
                            option { value: OPT_UNIT_YEAR, {OPT_UNIT_YEAR} }
                            option { value: OPT_UNIT_SECOND, {OPT_UNIT_SECOND} }
                        }
                    }
                }
            }

            // 额度重置
            div { class: "pt-2 space-y-2",
                h4 { class: "text-xs font-semibold text-emerald-400 flex items-center gap-1.5", {FIELD_RESET_SECTION} }
                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", {FIELD_RESET_CYCLE} }
                        select {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                            value: "{f_reset_cycle()}",
                            oninput: move |e| f_reset_cycle.set(e.value()),
                            option { value: OPT_RESET_NEVER, {OPT_RESET_NEVER} }
                            option { value: OPT_RESET_DAILY, {OPT_RESET_DAILY} }
                            option { value: OPT_RESET_WEEKLY, {OPT_RESET_WEEKLY} }
                            option { value: OPT_RESET_MONTHLY, {OPT_RESET_MONTHLY} }
                            option { value: OPT_RESET_CUSTOM, {OPT_RESET_CUSTOM} }
                        }
                    }
                    label { class: "block space-y-1",
                        span { class: "text-xs text-zinc-400", {FIELD_RESET_SECS} }
                        input {
                            r#type: "number",
                            disabled: f_reset_cycle() != OPT_RESET_CUSTOM,
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
///
/// 【是什么】弹窗第三个页签:顶部一段说明条 + 三个第三方平台商品 ID 输入。
///
/// 【做什么】渲染说明条与三个受控输入;不持状态、不校验、不提交。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:在任一输入框输入 → 直接写回
/// 对应 `f_stripe_id` / `f_creem_id` / `f_waffo_id` signal。纯本地 signal
/// 写入,不发网络。
///
/// 【样式】Tab 体 `space-y-4 pt-1`;顶部说明条 `rounded-xl border
/// border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-300
/// leading-relaxed`;字段 `label.block.space-y-1` + 标签
/// `text-xs font-medium text-zinc-300`;输入框 `font-mono`,统一
/// `rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2`。
///
/// 【子组件组成】无子组件:原生 `div` / `label` / `span` / `input`。
///
/// 【数据流】
/// - 对内(入):`f_stripe_id` / `f_creem_id` / `f_waffo_id` 三个页面持有的
///   signal(初值均为空串)。
/// - 对外(出):无 EventHandler;输入直接写回上述 Signal。
#[component]
fn SubscriptionPaymentTab(
    f_stripe_id: Signal<String>,
    f_creem_id: Signal<String>,
    f_waffo_id: Signal<String>,
) -> Element {
    rsx! {
        div { class: "space-y-4 pt-1",
            div { class: "rounded-xl border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-300 leading-relaxed",
                {MSG_PAYMENT_NOTE}
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", {FIELD_STRIPE_ID} }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_stripe_id()}",
                    placeholder: MSG_PH_STRIPE_ID,
                    oninput: move |e| f_stripe_id.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", {FIELD_CREEM_ID} }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_creem_id()}",
                    placeholder: MSG_PH_CREEM_ID,
                    oninput: move |e| f_creem_id.set(e.value()),
                }
            }
            label { class: "block space-y-1",
                span { class: "text-xs font-medium text-zinc-300", {FIELD_WAFFO_ID} }
                input {
                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                    value: "{f_waffo_id()}",
                    placeholder: MSG_PH_WAFFO_ID,
                    oninput: move |e| f_waffo_id.set(e.value()),
                }
            }
        }
    }
}
