//! 订阅套餐 tab 页面入口(`SubscriptionsPage`)。
//!
//! 数据走真实后端 `/api/subscriptions`:列表由本页 `use_effect` 拉取进本地
//! signal;增/删/改由本页直接调 `crate::api` 的订阅端点,成功后 reload 重拉
//! 全表(后端按 `sort_order` 排序,本地插入无法保证位次),失败显示错误条。
//! 后端按 name upsert(同名保存即更新该行,改名 = 新建一行),故写回不依赖
//! 列表下标,一律按返回的 key 定位。
//!
//! 弹窗与卡片拆分到 `modal.rs` / `card.rs`;文案常量在 `shared.rs`。

use dioxus::prelude::*;

use client::ApiClient;
use contract::api::billing::SubscriptionUpsertRequest;

use crate::api::{
    delete_subscription_api, list_groups_api, list_subscriptions_api, upsert_subscription_api,
};
use crate::state::{PlanRow, map_subscription_view};

use crate::components::subscriptions_card::PlanCard;
use crate::components::subscriptions_modal::SubscriptionFormModal;
use crate::shared::{
    BTN_NEW_PLAN, MSG_CREATED, MSG_DELETED, MSG_EMPTY, MSG_ERR_NAME_REQUIRED, MSG_ERR_QUOTA,
    MSG_LOAD_FAIL_PREFIX, MSG_LOAD_FAIL_SUFFIX, MSG_LOADING, MSG_SAVING, MSG_UPDATED,
    OPT_BADGE_LOADING, SEC_LIST, SEC_NAME_DEDUP,
};

/// 以某行现有值重建 upsert 请求体(启停切换用):后端按 name 定位行
/// 并整体回写这些列,返回更新后的视图。price 取规范字符串形式。
fn rebuild_req(p: &PlanRow, enabled: bool) -> SubscriptionUpsertRequest {
    SubscriptionUpsertRequest {
        name: p.title.clone(),
        price: format!("{}", p.price),
        currency: p.currency.clone(),
        // duration_days 后端要求 >= 1;防御非法存量行(0 天)。
        duration_days: p.period_val.max(1),
        // quota 已是展示口径,原样回传(后端入库侧自己 ×500_000)。
        quota: p.quota,
        upgrade_group: if p.group.is_empty() || p.group == "不升级" {
            None
        } else {
            Some(p.group.clone())
        },
        // 0 = 不限 → None(后端 max_purchases 列可空)。
        max_purchases: if p.max_per_user > 0 {
            Some(p.max_per_user)
        } else {
            None
        },
        enabled: Some(enabled),
    }
}

/// 订阅管理: 单栏卡牌展示 (web/平板/手机均为 1 栏) + 两 Tab 编辑弹窗。
///
/// 数据走真实后端 `/api/subscriptions`:列表由本页 `use_effect` 拉取进本地
/// signal;增/删/改由本页直接调 `crate::api` 的订阅端点,成功后 reload 重拉
/// 全表,失败显示错误条。四态渲染(loading / error / empty / data)对齐
/// CurrencyPage 惯例。
///
/// 【子组件组成】`PlanCard`(单张套餐卡,纯展示 + 三个回调出口)、
/// `SubscriptionFormModal`(两 Tab 编辑弹窗,表单 signal 由本页持有并传入)。
///
/// 【数据流】
/// - 列表:`use_effect` 内 `list_subscriptions_api` → `map_subscription_view`
///   → `plans` signal;`reload` 计数器变更即重拉。
/// - 写回:`commit`(新建/编辑保存)/ 行内启停 / 行内删除均 spawn 异步调用,
///   成功后 `reload += 1` 触发重拉,失败写 `action_err` 红条。
#[component]
pub fn SubscriptionsPage() -> Element {
    // 列表数据走本地 signal(同 CurrencyPage/AliasesPage 范式):
    // EntityStore 的 hydrate 灌入的是一次性实例,上下文 store 无人填充,
    // 订阅页读 store.plans 会永远空——列表必须自己拉。
    let mut plans = use_signal(Vec::<PlanRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写回成功后 +1 触发重拉:后端按 sort_order 排序,本地插入无法保证位次。
    // 注意:外层绑定不 mutate——写路径里 `let mut reload = reload;` 各自拷贝
    // 出可变副本(Signal 是 Copy),外层只需只读。
    let reload = use_signal(|| 0u32);
    // 「升级分组」下拉候选项(真实分组名)。
    let mut group_names = use_signal(Vec::<String>::new);

    // 分页:列表内部 UI 状态(不跨组件);写回重拉后条数变小时 clamp 到最后一页,
    // 不落空页(与 aliases / groups 列表同款约定)。
    let mut page = use_signal(|| 0usize);
    let visible = ui::page_slice(&plans.read(), page(), ui::CARD_PAGE_SIZE).to_vec();

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // 分组名先拉(快、小):失败不阻塞套餐列表,下拉退化到只有「不升级」。
            match list_groups_api(&client).await {
                Ok(gs) => group_names.set(gs.into_iter().map(|g| g.name).collect()),
                Err(_) => group_names.set(Vec::new()),
            }
            match list_subscriptions_api(&client).await {
                Ok(items) => {
                    plans.set(items.into_iter().map(map_subscription_view).collect());
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let mut show_modal = use_signal(|| false);
    let mut modal_tab = use_signal(|| 0u8);
    let mut editing_idx = use_signal(|| None::<usize>);

    // 写反馈三信号(loading / error / ok)
    let mut saving = use_signal(|| false);
    let mut action_err = use_signal(|| None::<String>);
    let mut ok_msg = use_signal(|| None::<String>);

    // ---- Tab 0 基本信息:名称 / 价格 / 币种 / 额度 ----
    let mut f_title = use_signal(String::new);
    let mut f_price = use_signal(|| "0".to_string());
    let mut f_currency = use_signal(|| "CNY".to_string());
    let mut f_quota = use_signal(|| "0".to_string());
    // ---- Tab 1 规则与周期:启用 / 有效期天数 / 升级分组 / 限购 ----
    let mut f_duration = use_signal(|| "30".to_string());
    let mut f_group = use_signal(|| "不升级".to_string());
    let mut f_limit = use_signal(|| "0".to_string());
    let mut f_enabled = use_signal(|| true);

    let open_edit = move |i: usize| {
        let p = plans.read()[i].clone();
        f_title.set(p.title);
        f_price.set(format!("{}", p.price));
        f_currency.set(p.currency);
        f_quota.set(format!("{}", p.quota));
        f_duration.set(format!("{}", p.period_val.max(1)));
        f_group.set(if p.group.is_empty() {
            "不升级".into()
        } else {
            p.group
        });
        f_limit.set(format!("{}", p.max_per_user));
        f_enabled.set(p.enabled);
        editing_idx.set(Some(i));
        action_err.set(None);
        ok_msg.set(None);
        modal_tab.set(0);
        show_modal.set(true);
    };

    let open_new = move |_| {
        f_title.set(String::new());
        f_price.set("0".into());
        f_currency.set("CNY".into());
        f_quota.set("0".into());
        f_duration.set("30".into());
        f_group.set("不升级".into());
        f_limit.set("0".into());
        f_enabled.set(true);
        editing_idx.set(None);
        action_err.set(None);
        ok_msg.set(None);
        modal_tab.set(0);
        show_modal.set(true);
    };

    let mut commit = move |_| {
        let name = f_title().trim().to_string();
        if name.is_empty() {
            action_err.set(Some(MSG_ERR_NAME_REQUIRED.into()));
            return;
        }
        let currency = f_currency();
        let price = f_price().trim().to_string();
        // 后端拒绝 duration_days == 0:非法/空输入钳到 1 而不是发 0 挨 400。
        let duration_days = f_duration()
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|&d| d >= 1)
            .unwrap_or(1);
        // quota 展示口径直传;空串 = 0(无额度套餐),但显式非法值(非数字/负数)
        // 必须报错而非静默存 0——否则用户拿到「已创建」成功提示却存了错数据。
        let quota = {
            let raw = f_quota().to_string();
            let raw = raw.trim();
            if raw.is_empty() {
                0.0
            } else {
                match raw.parse::<f64>() {
                    Ok(q) if q.is_finite() && q >= 0.0 => q,
                    _ => {
                        action_err.set(Some(MSG_ERR_QUOTA.into()));
                        return;
                    }
                }
            }
        };
        let upgrade_group = {
            let g = f_group();
            if g.is_empty() || g == "不升级" {
                None
            } else {
                Some(g)
            }
        };
        let max_purchases = f_limit().trim().parse::<u32>().ok().filter(|&n| n > 0);
        let enabled = f_enabled();
        let editing = editing_idx();

        let client = ApiClient::shared().clone();
        let mut reload = reload;
        saving.set(true);
        action_err.set(None);
        ok_msg.set(None);
        spawn(async move {
            let req = SubscriptionUpsertRequest {
                name,
                price,
                currency,
                duration_days,
                quota,
                upgrade_group,
                max_purchases,
                enabled: Some(enabled),
            };
            match upsert_subscription_api(&client, &req).await {
                Ok(_) => {
                    saving.set(false);
                    ok_msg.set(Some(if editing.is_some() {
                        MSG_UPDATED.into()
                    } else {
                        MSG_CREATED.into()
                    }));
                    show_modal.set(false);
                    // 重拉全表:后端按 sort_order 排序,本地插入无法保证位次。
                    reload += 1;
                }
                Err(e) => {
                    saving.set(false);
                    action_err.set(Some(e.to_string()));
                }
            }
        });
    };

    // 行内启停:以同一 name 重建 upsert 体(enabled 取反),后端按 name 定位
    // 行整体回写。下标由 PlanCard 回调给出,请求体在闭包构造前算好 owned。
    let toggle_row = move |i: usize| {
        let p = plans.read()[i].clone();
        let client = ApiClient::shared().clone();
        action_err.set(None);
        ok_msg.set(None);
        let req = rebuild_req(&p, !p.enabled);
        let mut reload = reload;
        spawn(async move {
            match upsert_subscription_api(&client, &req).await {
                Ok(_) => reload += 1,
                Err(e) => action_err.set(Some(e.to_string())),
            }
        });
    };

    // 行内删除:按行 key 调 DELETE 端点。
    let delete_row = move |i: usize| {
        let key = plans.read()[i].key.clone();
        let client = ApiClient::shared().clone();
        action_err.set(None);
        ok_msg.set(None);
        let mut reload = reload;
        spawn(async move {
            match delete_subscription_api(&client, &key).await {
                Ok(()) => {
                    ok_msg.set(Some(MSG_DELETED.into()));
                    reload += 1;
                }
                Err(e) => action_err.set(Some(e.to_string())),
            }
        });
    };

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            role: "region",
            "aria-label": "订阅套餐管理",
            "data-testid": "subscriptions-page",

            // ---- error:写请求(upsert / delete)失败的红条 ----
            if let Some(e) = action_err() {
                div { class: "rounded-xl border border-destructive bg-destructive p-3 text-sm {ui::C_DANGER}",
                    "data-testid": "subscriptions-action-error",
                    "{e}"
                }
            }
            // ---- ok:写成功反馈(点击后立即可见,列表也已同步刷新) ----
            if let Some(m) = ok_msg() {
                div { class: "rounded-xl border border-emerald-800 bg-success p-3 text-sm {ui::C_SUCCESS}",
                    "data-testid": "subscriptions-action-ok",
                    "{m}"
                }
            }
            // ---- loading:写请求在途(保存中…,弹窗保存按钮同时禁用) ----
            if saving() {
                div { class: "rounded-xl border border-border bg-card/60 px-4 py-2.5 {ui::TYPE_DESC}",
                    "data-testid": "subscriptions-saving",
                    "{MSG_SAVING}"
                }
            }

            // 顶部栏: 提示横幅 + 新建按钮
            div { class: "flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-warning px-4 py-3",
                div { class: "flex items-center gap-2 text-xs {ui::C_WARNING}",
                    span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-warning font-bold", "ℹ" }
                    span { "{SEC_NAME_DEDUP}" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-lg bg-warning px-3.5 py-1.5 {ui::TYPE_DESC} transition-colors hover:bg-warning shadow-sm",
                    "data-testid": "subscriptions-new",
                    onclick: open_new,
                    span { class: "{ui::TYPE_BODY}", "+" }
                    "{BTN_NEW_PLAN}"
                }
            }

            // ---- loading:首次/重拉在途(与写请求的 saving 指示区分开) ----
            if loading() {
                div { class: "rounded-lg border border-border bg-card/60 p-6 text-center {ui::TYPE_BODY}",
                    "data-testid": "subscriptions-loading",
                    "{MSG_LOADING}"
                }
            }
            // ---- error:列表拉取失败(与写请求的 action_err 红条区分开) ----
            if let Some(e) = err() {
                div { class: "rounded-xl border border-destructive bg-destructive p-3 text-sm {ui::C_DANGER}",
                    "data-testid": "subscriptions-load-error",
                    "{MSG_LOAD_FAIL_PREFIX}{e}{MSG_LOAD_FAIL_SUFFIX}"
                }
            }

            // ---- empty / data ----
            if !loading() && plans.read().is_empty() {
                div { class: "rounded-lg border border-dashed border-border p-6 text-center {ui::TYPE_BODY}",
                    "data-testid": "subscriptions-empty",
                    "{MSG_EMPTY}"
                }
            } else {
                section { class: "scroll-mt-8 space-y-4",
                    ui::SectionHeader {
                        title: SEC_LIST.to_string(),
                        badge: if loading() { OPT_BADGE_LOADING.to_string() } else { format!("{} 个", plans.read().len()) },
                        trailing: rsx! {
                            ui::Pager {
                                total: plans.read().len(),
                                page,
                                on_change: move |p| page.set(p),
                                testid: "subscriptions-pager",
                            }
                        },
                    }
                    // 单栏卡牌列表容器 (Web / 平板 / 手机统一一栏优雅排布)
                    div { class: "flex flex-col gap-3",
                        "data-testid": "subscriptions-list",
                        for (idx, p) in visible.iter().enumerate() {
                            PlanCard {
                                // key 用套餐 key(稳定标识):按下标会在翻页/排序后错配
                                key: "{p.key}",
                                plan: p.clone(),
                                // 回调按 plans 全表下标定位行,翻页后要加回页偏移
                                index: page() * ui::CARD_PAGE_SIZE + idx,
                                on_edit: open_edit,
                                on_toggle: toggle_row,
                                on_delete: delete_row,
                            }
                        }
                    }
                }
            }
        }

        SubscriptionFormModal {
            show_modal,
            modal_tab,
            editing_idx,
            saving,
            action_err,
            group_names,
            f_title,
            f_price,
            f_currency,
            f_quota,
            f_duration,
            f_group,
            f_limit,
            f_enabled,
            on_commit: move |_| commit(()),
        }
    }
}
