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
use super::shared::{
    BTN_NEW_PLAN, OPT_DOWNGRADE_PREV, OPT_NO_UPGRADE, OPT_PAY_ONLY_SPECIES, OPT_RESET_NEVER,
    OPT_UNIT_MONTH, SEC_HONEST_BANNER, SEC_PAYMENT_HINT,
};
use crate::state::{EntityStore, PlanRow};

/// 订阅套餐管理页
///
/// 【是什么】订阅套餐 tab 的页面入口:诚实横幅 + 顶部栏 + 单栏套餐卡列表
/// + 三 Tab 编辑/新建弹窗。
///
/// 【做什么】持有 21 个 `f_*` 表单 signal 与弹窗开关状态,提供 `open_edit` /
/// `open_new` / `commit` 三个写回闭包,以及卡片启停/删除的本地改动。
/// 不负责卡片与弹窗的渲染细节(分别在 `card.rs` / `modal.rs`)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点卡片「编辑」→ `open_edit(i)` 把该行值回填进各 `f_*` signal,
///   `editing_idx = Some(i)`,`modal_tab = 0`,开弹窗。
/// - 点「新建套餐」→ `open_new` 把字段重置为默认值(支付方式「仅扣菌种」、
///   分组「不升级」、降级分组「降级到购买前分组」、有效期 1 个月等),
///   `editing_idx = None`,开弹窗。
/// - 弹窗内点「保存更改」→ `commit`:标题 trim 后为空则直接 return(不写回),
///   否则组 `PlanRow`,按 `editing_idx` 决定原地替换还是插到列表头,关弹窗。
/// - 卡片开关/删除 → 就地改 `plans` signal(取反 enabled / remove 该行)。
/// 数据交互:本页**不发任何网络请求**,数据全走 `EntityStore` 本地演示态
/// (订阅套餐后端暂未实现,横幅已明示)。
///
/// 【样式】根容器 `flex flex-col gap-4 w-full`;诚实横幅 `rounded-xl border
/// border-zinc-700/60 bg-zinc-900/60 px-4 py-3`;顶部栏 `rounded-xl border
/// border-amber-500/20 bg-amber-500/5`,新建按钮 `bg-amber-400` 圆角实底;
/// 卡片列表容器 `flex flex-col gap-3`(单栏,Web/平板/手机统一一栏)。
///
/// 【子组件组成】`PlanCard`(单张套餐卡)、`SubscriptionFormModal`(编辑/新建
/// 弹窗,内含三个 Tab 体);横幅与顶部栏为原生元素。
///
/// 【数据流】
/// - 对内(入):无 props;`store`(context 注入的 `EntityStore`)、
///   `plans` / `groups`(从 store 取出的 signal 句柄)与全部 `f_*` 表单
///   signal、`show_modal` / `modal_tab` / `editing_idx` 均由本页持有。
/// - 对外(出):把 `groups` 与 21 个 `f_*` signal 以 Signal prop 注入弹窗,
///   弹窗直接写这些 signal;`on_cancel` 关弹窗;`on_submit` 走本页 `commit`
///   写 `plans`。`PlanCard` 的三个回调分别指向 `open_edit` / 就地改 `plans`。
#[component]
pub fn SubscriptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut plans = store.plans;
    let groups = store.groups;

    // 弹窗开关状态:跨组件交互(顶部栏新建按钮 / 卡片编辑按钮 / 弹窗关闭按钮
    // 三处都要读写),故提升到页面层,以 Signal prop 传入弹窗。
    let mut show_modal = use_signal(|| false);
    let mut modal_tab = use_signal(|| 0u8);
    let mut editing_idx = use_signal(|| None::<usize>);

    // 基本信息表单字段:21 个 f_* signal 中以 Signal prop 注入弹窗、由弹窗
    // 内的输入框直接写回。放页面层是因为它们由「打开弹窗」这一跨组件动作
    // (open_edit / open_new)初始化,且 commit 在页面侧读取它们组装 PlanRow。
    let mut f_id = use_signal(|| 0u32);
    let mut f_title = use_signal(String::new);
    let mut f_subtitle = use_signal(String::new);
    let mut f_price = use_signal(|| "0".to_string());
    let mut f_quota = use_signal(|| "0".to_string());
    let mut f_currency_price = use_signal(|| "0".to_string());
    let mut f_payment_method = use_signal(|| OPT_PAY_ONLY_SPECIES.to_string());
    let mut f_group = use_signal(|| OPT_NO_UPGRADE.to_string());
    let mut f_downgrade_group = use_signal(|| OPT_DOWNGRADE_PREV.to_string());
    let mut f_limit = use_signal(|| "0".to_string());
    let mut f_sort = use_signal(|| "0".to_string());

    // 规则与周期字段:归属同上,由弹窗「规则与周期」Tab 读写。
    let mut f_enabled = use_signal(|| true);
    let mut f_allow_redeem = use_signal(|| true);
    let mut f_allow_wallet = use_signal(|| true);
    let mut f_period_val = use_signal(|| "1".to_string());
    let mut f_period_unit = use_signal(|| OPT_UNIT_MONTH.to_string());
    let mut f_reset_cycle = use_signal(|| OPT_RESET_NEVER.to_string());

    // 第三方支付字段:归属同上,由弹窗「第三方支付配置」Tab 读写。
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
        f_payment_method.set(OPT_PAY_ONLY_SPECIES.to_string());
        f_group.set(OPT_NO_UPGRADE.to_string());
        f_downgrade_group.set(OPT_DOWNGRADE_PREV.to_string());
        f_limit.set("0".to_string());
        f_sort.set("0".to_string());
        f_enabled.set(true);
        f_allow_redeem.set(true);
        f_allow_wallet.set(true);
        f_period_val.set("1".to_string());
        f_period_unit.set(OPT_UNIT_MONTH.to_string());
        f_reset_cycle.set(OPT_RESET_NEVER.to_string());
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
                span { {SEC_HONEST_BANNER} }
            }
            // 顶部栏: 提示横幅 + 新建按钮
            div { class: "flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-amber-500/5 px-4 py-3",
                div { class: "flex items-center gap-2 text-xs text-amber-300",
                    span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-amber-500/20 font-bold", "ℹ" }
                    span { {SEC_PAYMENT_HINT} }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-lg bg-amber-400 px-3.5 py-1.5 text-xs font-semibold text-zinc-950 transition-colors hover:bg-amber-300 shadow-sm",
                    onclick: open_new,
                    span { class: "text-sm", "+" }
                    {BTN_NEW_PLAN}
                }
            }

            // 单栏卡牌列表容器 (Web / 平板 / 手机统一一栏优雅排布)
            // PlanCard:单张套餐卡(ID 徽标 + 标题 + 状态/分组徽标 + 五格指标条)。
            // 启停/删除直接改本地演示态 plans(订阅后端暂未实现,页面横幅有说明);
            // on_edit 开弹窗回填,跨组件交互由页面闭包处理。
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
        // 外壳(遮罩+头部+tab 切换条) + 三个 tab 子组件(基本信息/规则与周期/第三方支付)。
        // 21 个 f_* 表单 signal 以 Signal 注入,commit 写回逻辑留在页面,
        // 弹窗只渲染与抛 on_submit/on_cancel。
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
