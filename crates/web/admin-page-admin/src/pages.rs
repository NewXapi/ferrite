//! 管理区功能页:每个 tab 一个操作面板页,面板按 1/3/5 栏响应式铺开
//! (手机 1 栏 / 平板 3 栏 / 桌面 5 栏)。交互对齐 new-api 对应功能区:
//! 渠道的状态速览/编辑/调度/批量,别名的计费,订阅与兑换码的生成与审计。
//!
//! 数据全走 `state::EntityStore` 的初始 mock 值在 api.rs 中获取；接 API 时把初始值换成请求结果即可。
//!
//! 布局约定(与项目 gate-checklist 一致):
//! - 桌面端面板间用「分隔线 + 独占行」表达从属关系,不占标签页;
//! - 交互控件以原生为主(select / number input / checkbox),自定义件必须带状态语义;
//! - 反馈一致:确认用「已保存/已生成/已测速」文字,危险操作用红色。

use crate::state::{EntityStore, PlanRow};
use dioxus::prelude::*;

// ============ 页面骨架 ============

/// 1/3 栏响应式网格(手机 1 / 平板与Web 3 栏)。
#[component]
pub fn GridShell(children: Element) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3", {children} }
    }
}

/// 面板基础件:标题 + 说明 + 内容。
#[component]
pub fn Panel(title: &'static str, hint: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            p { class: "text-sm font-medium text-zinc-100", "{title}" }
            p { class: "text-[11px] text-zinc-600", "{hint}" }
            {children}
        }
    }
}

/// 主按钮(确认 / 保存 / 生成等)。
#[component]
pub(crate) fn PushBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 危险操作(删除 / 停用)。
#[component]
pub(crate) fn DangerBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-red-900/60 px-3 py-1.5 text-xs text-red-400 hover:border-red-700",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 幽灵操作(清空 / 取消)。
#[component]
pub(crate) fn GhostBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 状态开关(对齐 new-api 的启用/停用徽章;带 on/off 文字态)。
#[component]
pub(crate) fn ToggleSwitch(on: bool, on_toggle: EventHandler<()>) -> Element {
    let track = if on { "bg-zinc-100" } else { "bg-zinc-700" };
    let knob = if on { "translate-x-4" } else { "translate-x-0" };
    rsx! {
        button {
            class: "relative h-5 w-9 shrink-0 rounded-full transition-colors {track}",
            role: "switch",
            "aria-checked": "{on}",
            onclick: move |_| on_toggle.call(()),
            span { class: "absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-zinc-950 transition-transform {knob}" }
        }
    }
}

// ============ 订阅页 ============

/// 订阅管理: 单栏卡牌展示 (web/平板/手机均为 1 栏) + 多 Tab 编辑弹窗 (对齐 new-api 订阅配置)
#[component]
pub fn SubscriptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut plans = store.plans;
    let groups = store.groups;

    let mut show_modal = use_signal(|| false);
    let mut modal_tab = use_signal(|| 0u8);
    let mut editing_idx = use_signal(|| None::<usize>);

    // 基本信息表单字段
    let mut f_id = use_signal(|| 0i32);
    let mut f_title = use_signal(String::new);
    let mut f_subtitle = use_signal(String::new);
    let mut f_price = use_signal(|| "0".to_string());
    let mut f_quota = use_signal(|| "0".to_string());
    let mut f_currency_price = use_signal(|| "0".to_string());
    let mut f_payment_method = use_signal(|| "仅扣菌种".to_string());
    let mut f_group = use_signal(|| "不升级".to_string());
    let mut f_downgrade_group = use_signal(|| "降级到购买前分组".to_string());
    let mut f_limit = use_signal(|| "0".to_string());
    let mut f_sort = use_signal(|| "0".to_string());

    // 规则与周期字段
    let mut f_enabled = use_signal(|| true);
    let mut f_allow_redeem = use_signal(|| true);
    let mut f_allow_wallet = use_signal(|| true);
    let mut f_period_val = use_signal(|| "1".to_string());
    let mut f_period_unit = use_signal(|| "个月".to_string());
    let mut f_reset_cycle = use_signal(|| "不重置".to_string());

    // 第三方支付字段
    let mut f_stripe_id = use_signal(String::new);
    let mut f_creem_id = use_signal(String::new);
    let mut f_waffo_id = use_signal(String::new);

    let mut open_edit = move |i: usize| {
        let p = plans.read()[i].clone();
        f_id.set(p.id);
        f_title.set(p.title);
        f_subtitle.set(p.subtitle);
        f_price.set(format!("{}", p.price));
        f_quota.set(format!("{}", p.quota));
        f_currency_price.set(format!("{}", p.currency_price));
        f_payment_method.set(p.payment_method);
        f_group.set(p.group);
        f_downgrade_group.set(p.downgrade_group);
        f_limit.set(format!("{}", p.max_per_user));
        f_sort.set(format!("{}", p.sort_order));
        f_enabled.set(p.enabled);
        f_allow_redeem.set(p.allow_redeem);
        f_allow_wallet.set(p.allow_wallet);
        f_period_val.set(format!("{}", p.period_val));
        f_period_unit.set(p.period_unit);
        f_reset_cycle.set(p.reset_cycle);
        f_stripe_id.set(p.stripe_price_id);
        f_creem_id.set(p.creem_product_id);
        f_waffo_id.set(p.waffo_product_id);
        editing_idx.set(Some(i));
        modal_tab.set(0);
        show_modal.set(true);
    };

    let open_new = move |_| {
        let next_id = plans.read().iter().map(|p| p.id).max().unwrap_or(0) + 1;
        f_id.set(next_id);
        f_title.set(String::new());
        f_subtitle.set(String::new());
        f_price.set("0".to_string());
        f_quota.set("0".to_string());
        f_currency_price.set("0".to_string());
        f_payment_method.set("仅扣菌种".to_string());
        f_group.set("不升级".to_string());
        f_downgrade_group.set("降级到购买前分组".to_string());
        f_limit.set("0".to_string());
        f_sort.set("0".to_string());
        f_enabled.set(true);
        f_allow_redeem.set(true);
        f_allow_wallet.set(true);
        f_period_val.set("1".to_string());
        f_period_unit.set("个月".to_string());
        f_reset_cycle.set("不重置".to_string());
        f_stripe_id.set(String::new());
        f_creem_id.set(String::new());
        f_waffo_id.set(String::new());
        editing_idx.set(None);
        modal_tab.set(0);
        show_modal.set(true);
    };

    let commit = move |_| {
        let t = f_title.peek().trim().to_string();
        if t.is_empty() {
            return;
        }
        let row = PlanRow {
            id: f_id(),
            title: t,
            subtitle: f_subtitle.peek().trim().to_string(),
            price: f_price.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            quota: f_quota.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            currency_price: f_currency_price
                .peek()
                .trim()
                .parse::<f64>()
                .unwrap_or(0.0)
                .max(0.0),
            payment_method: f_payment_method(),
            group: f_group(),
            downgrade_group: f_downgrade_group(),
            period_val: f_period_val
                .peek()
                .trim()
                .parse::<i32>()
                .unwrap_or(1)
                .max(1),
            period_unit: f_period_unit(),
            reset_cycle: f_reset_cycle(),
            priority: 0,
            enabled: f_enabled(),
            allow_redeem: f_allow_redeem(),
            allow_wallet: f_allow_wallet(),
            max_per_user: f_limit.peek().trim().parse::<i32>().unwrap_or(0),
            sort_order: f_sort.peek().trim().parse::<i32>().unwrap_or(0),
            stripe_price_id: f_stripe_id.peek().trim().to_string(),
            creem_product_id: f_creem_id.peek().trim().to_string(),
            waffo_product_id: f_waffo_id.peek().trim().to_string(),
        };
        match *editing_idx.peek() {
            Some(i) => {
                plans.write()[i] = row;
            }
            None => plans.write().insert(0, row),
        }
        show_modal.set(false);
    };

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            // 顶部栏: 提示横幅 + 新建按钮
            div { class: "flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-amber-500/5 px-4 py-3",
                div { class: "flex items-center gap-2 text-xs text-amber-300",
                    span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-amber-500/20 font-bold", "ℹ" }
                    span { "Stripe / Creem 需在第三方平台创建商品并填入 ID" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-lg bg-amber-400 px-3.5 py-1.5 text-xs font-semibold text-zinc-950 transition-colors hover:bg-amber-300 shadow-sm",
                    onclick: open_new,
                    span { class: "text-sm", "+" }
                    "新建套餐"
                }
            }

            // 单栏卡牌列表容器 (Web / 平板 / 手机统一一栏优雅排布)
            div { class: "flex flex-col gap-3",
                for (i, p) in plans.read().iter().enumerate() {
                    {
                        let title_txt = p.title.clone();
                        let sub_txt = p.subtitle.clone();
                        let price_str = format!("${:.2}", p.price);
                        let quota_str = if p.quota <= 0.0 { "无限制".to_string() } else { format!("{}", p.quota) };
                        let period_str = format!("{} {}", p.period_val, p.period_unit);
                        rsx! {
                            div {
                                key: "{p.id}",
                                class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/90 shadow-md",

                                // 卡片头部行: ID + 标题 + 状态/分组徽标 + 操作按钮
                                div { class: "flex flex-wrap items-start justify-between gap-2.5",
                                    div { class: "flex items-center gap-2.5 min-w-0 flex-1",
                                        span { class: "shrink-0 rounded-md border border-zinc-700/80 bg-zinc-800 px-2 py-0.5 text-xs font-mono font-bold text-zinc-300",
                                            "#{p.id}"
                                        }
                                        h3 { class: "truncate text-base font-bold text-zinc-100", "{title_txt}" }
                                        span {
                                            class: if p.enabled { "rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400" } else { "rounded-full border border-zinc-700 bg-zinc-800/80 px-2.5 py-0.5 text-[11px] font-medium text-zinc-500" },
                                            if p.enabled { "启用" } else { "禁用" }
                                        }
                                        if !p.group.is_empty() && p.group != "不升级" {
                                            span { class: "rounded-full border border-sky-500/30 bg-sky-500/10 px-2.5 py-0.5 text-[11px] font-medium text-sky-400 uppercase",
                                                "分组: {p.group}"
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2 shrink-0",
                                        ToggleSwitch {
                                            on: p.enabled,
                                            on_toggle: move |_| {
                                                let mut w = plans.write();
                                                w[i].enabled = !w[i].enabled;
                                            },
                                        }
                                        button {
                                            class: "rounded-lg border border-zinc-700 bg-zinc-800 px-2.5 py-1 text-xs text-zinc-200 transition-colors hover:bg-zinc-700 hover:text-white",
                                            onclick: move |_| open_edit(i),
                                            "编辑"
                                        }
                                        button {
                                            class: "rounded-lg border border-red-900/50 bg-red-950/20 px-2 py-1 text-xs text-red-400 transition-colors hover:bg-red-900/30 hover:text-red-300",
                                            onclick: move |_| { plans.write().remove(i); },
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
                                        span { class: "text-[11px] text-zinc-500 block", "价格" }
                                        span { class: "font-mono font-bold text-sm text-emerald-400", "{price_str}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "有效期" }
                                        span { class: "font-medium text-zinc-200", "{period_str}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "套餐额度" }
                                        span { class: "font-mono font-semibold text-amber-300 flex items-center gap-1",
                                            span { "🧀" }
                                            span { "{quota_str}" }
                                        }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "站内支付 / 渠道" }
                                        span { class: "text-zinc-300 font-medium", "{p.payment_method}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "额度重置" }
                                        span { class: "text-zinc-400", "{p.reset_cycle}" }
                                    }
                                }

                                // 第三方配置徽标展示
                                if !p.stripe_price_id.is_empty() || !p.creem_product_id.is_empty() {
                                    div { class: "mt-2.5 flex flex-wrap gap-2 text-[10px] text-zinc-500 font-mono",
                                        if !p.stripe_price_id.is_empty() {
                                            span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Stripe: {p.stripe_price_id}" }
                                        }
                                        if !p.creem_product_id.is_empty() {
                                            span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Creem: {p.creem_product_id}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ============ 多 Tab 编辑/新建弹窗 (对标 Image #6, #7, #8) ============
        if show_modal() {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm",
                onclick: move |_| show_modal.set(false),
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
                            onclick: move |_| show_modal.set(false),
                            "✕"
                        }
                    }

                    // 弹窗内部 Tab 切换条
                    div { class: "flex items-center gap-2 border-b border-zinc-800 pb-2 text-xs",
                        button {
                            class: if modal_tab() == 0 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(0),
                            "🔑 基本信息"
                        }
                        button {
                            class: if modal_tab() == 1 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(1),
                            "📅 规则与周期"
                        }
                        button {
                            class: if modal_tab() == 2 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(2),
                            "💳 第三方支付配置"
                        }
                    }

                    // ---- Tab 0: 基本信息 (Image #6) ----
                    if modal_tab() == 0 {
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
                                    span { class: "text-xs font-medium text-zinc-300", "额度 (🧀)" }
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

                    // ---- Tab 1: 规则与周期 (Image #7) ----
                    if modal_tab() == 1 {
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
                                h4 { class: "text-xs font-semibold text-amber-400 flex items-center gap-1.5", "📅 有效期设置" }
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
                                h4 { class: "text-xs font-semibold text-emerald-400 flex items-center gap-1.5", "🔄 额度重置" }
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

                    // ---- Tab 2: 第三方支付配置 (Image #8) ----
                    if modal_tab() == 2 {
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

                    // 弹窗底部操作按钮
                    div { class: "flex items-center justify-end gap-3 pt-3 border-t border-zinc-800",
                        button {
                            class: "rounded-xl border border-zinc-700 px-4 py-2 text-xs font-medium text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                            onclick: move |_| show_modal.set(false),
                            "关闭"
                        }
                        button {
                            class: "rounded-xl bg-amber-400 px-5 py-2 text-xs font-bold text-zinc-950 hover:bg-amber-300 transition-colors shadow-lg shadow-amber-500/10",
                            onclick: commit,
                            "保存更改"
                        }
                    }
                }
            }
        }
    }
}

/// 粘贴文本里抽出 (Base URL, API Key)。支持多种形式:
/// - 每行一对:`https://api.openai.com/v1\nsk-xxx`
/// - `|` / 空白 分隔:`https://x | sk-xxx`
/// - `url=https://x\nkey=sk-xxx`(或 base_url / api_key)
///   仅在能同时拿到 URL 和 Key 时返回 Some,否则 None(让用户继续手动填)。
pub fn parse_url_key(text: &str) -> Option<(String, String)> {
    let mut url = None::<String>;
    let mut key = None::<String>;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // key=value 形式
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            if k == "url" || k == "base_url" || k == "endpoint" || k == "api_base" {
                url = Some(v.to_string());
                continue;
            }
            if k == "key" || k == "api_key" || k == "apikey" || k == "token" {
                key = Some(v.to_string());
                continue;
            }
            continue;
        }
        let parts: Vec<&str> = line
            .split(|c: char| c.is_whitespace() || c == '|' || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            let first = parts[0];
            if first.starts_with("http://") || first.starts_with("https://") {
                url.get_or_insert_with(|| first.to_string());
                for p in &parts[1..] {
                    if p.starts_with("sk-") || p.starts_with("rk-") || p.len() >= 20 {
                        key.get_or_insert_with(|| p.to_string());
                        break;
                    }
                }
                continue;
            }
        }
        // 裸 key 行:sk-/rk- 前缀 + 至少 20 字符,降低误匹配短串的概率
        if (line.starts_with("sk-") || line.starts_with("rk-")) && line.len() >= 20 {
            key.get_or_insert_with(|| line.to_string());
            continue;
        }
        // 裸 URL 行
        if line.starts_with("http://") || line.starts_with("https://") {
            url.get_or_insert_with(|| line.to_string());
        }
    }
    match (url, key) {
        (Some(u), Some(k)) => Some((u, k)),
        _ => None,
    }
}
