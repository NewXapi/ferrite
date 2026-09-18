//! 订阅套餐管理页:单栏卡片列表 + 三 Tab 编辑/新建弹窗(本地演示态,
//! 订阅套餐后端暂未实现)。
//!
//! 本文件只保留状态与写回逻辑(`open_edit` / `open_new` / `commit` /
//! 卡片启停删除),以及页面级布局(诚实横幅 + 顶部栏 + 卡片列表容器);
//! 渲染细节拆给 `tab-page-subscriptions-card`(套餐卡片)与
//! `tab-page-subscriptions-modal`(编辑弹窗 + 三个 Tab 体)。

use dioxus::prelude::*;

use super::card::PlanCard;
use super::modal::SubscriptionFormModal;
use crate::state::{EntityStore, PlanRow};

/// 订阅套餐管理页
#[component]
pub fn SubscriptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut plans = store.plans;
    let groups = store.groups;

    let mut show_modal = use_signal(|| false);
    let mut modal_tab = use_signal(|| 0u8);
    let mut editing_idx = use_signal(|| None::<usize>);

    // 基本信息表单字段
    let mut f_id = use_signal(|| 0u32);
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

    let open_edit = move |i: usize| {
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
                .parse::<u32>()
                .unwrap_or(1)
                .max(1),
            period_unit: f_period_unit(),
            reset_cycle: f_reset_cycle(),
            priority: 0,
            enabled: f_enabled(),
            allow_redeem: f_allow_redeem(),
            allow_wallet: f_allow_wallet(),
            max_per_user: f_limit.peek().trim().parse::<u32>().unwrap_or(0),
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
            // 诚实横幅: 订阅套餐后端暂未实现
            div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-3 text-xs text-zinc-400",
                span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-zinc-800 font-bold text-zinc-300", "i" }
                span { "订阅套餐后端暂未实现——此页暂无真实数据,以下为本地演示态" }
            }
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
                    PlanCard {
                        key: "{p.id}",
                        plan: p.clone(),
                        index: i,
                        on_edit: open_edit,
                        on_toggle: move |i: usize| {
                            let mut w = plans.write();
                            w[i].enabled = !w[i].enabled;
                        },
                        on_delete: move |i: usize| {
                            plans.write().remove(i);
                        },
                    }
                }
            }
        }

        // ============ 多 Tab 编辑/新建弹窗 (对标 Image #6, #7, #8) ============
        if show_modal() {
            SubscriptionFormModal {
                editing_idx,
                modal_tab,
                groups,
                f_title, f_subtitle, f_price, f_quota, f_currency_price,
                f_payment_method, f_group, f_downgrade_group, f_limit, f_sort,
                f_enabled, f_allow_redeem, f_allow_wallet,
                f_period_val, f_period_unit, f_reset_cycle,
                f_stripe_id, f_creem_id, f_waffo_id,
                on_cancel: move |_| show_modal.set(false),
                on_submit: commit,
            }
        }
    }
}
