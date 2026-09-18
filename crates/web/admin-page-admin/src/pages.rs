//! 管理区功能页:每个 tab 一个操作面板页,面板按 1/3/5 栏响应式铺开
//! (手机 1 栏 / 平板 3 栏 / 桌面 5 栏)。交互对齐 new-api 对应功能区:
//! 渠道的状态速览/编辑/调度/批量,别名的计费,订阅与兑换码的生成与审计。
//!
//! 数据由各面板自己拉取（本地 signal + `use_effect`，同 CurrencyPage/AliasesPage
//! 范式）;订阅页的增删改直接调 `crate::api` 的 `/api/subscriptions` 端点,
//! 成功后 reload 重拉全表（后端按 sort_order 排序，本地插入无法保证位次）。
//!
//! 布局约定(与项目 gate-checklist 一致):
//! - 桌面端面板间用「分隔线 + 独占行」表达从属关系,不占标签页;
//! - 交互控件以原生为主(select / number input / checkbox),自定义件必须带状态语义;
//! - 反馈一致:确认用「已保存/已生成/已测速」文字,危险操作用红色。

use crate::api::{
    delete_subscription_api, list_groups_api, list_subscriptions_api, upsert_subscription_api,
};
use crate::state::{PlanRow, map_subscription_view};
use client::ApiClient;
use contract::api::billing::SubscriptionUpsertRequest;
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

/// 订阅管理: 单栏卡牌展示 (web/平板/手机均为 1 栏) + 两 Tab 编辑弹窗。
///
/// 数据走真实后端 `/api/subscriptions`：列表由本页 `use_effect` 在
/// 本页 `use_effect` 拉取进本地 signal；增/删/改由本页直接调 `crate::api` 的订阅
/// 端点，成功后用返回的视图按 **key** 就地刷新 plans signal（响应式 UI
/// 立即更新），失败显示错误条。后端按 name upsert（同名保存即更新该行，
/// 改名 = 新建一行），故写回不依赖列表下标，一律按返回的 key 定位。
///
/// 弹窗字段即后端 `SubscriptionUpsertRequest` 的 8 个入参
/// （name/price/currency/durationDays/quota/upgradeGroup/maxPurchases/
/// enabled）；后端表不存的字段（副标题、第三方支付 ID、重置周期等）不进
/// 表单——填了存不下、刷新就丢，是误导性 UI。
///
/// 四态渲染（loading / error / empty / data）对齐 CurrencyPage 惯例：
/// - loading：写请求在途的「同步中」指示 + 弹窗保存按钮禁用；
/// - error：写请求失败的红条（`action_err`）；
/// - empty：列表为空时的空态提示；
/// - data：卡牌列表（列表本身的启动加载态由下面的 use_effect 拉取承担）。
#[component]
pub fn SubscriptionsPage() -> Element {
    // 列表数据走本地 signal（同 CurrencyPage/AliasesPage 范式）：
    // EntityStore 的 hydrate 灌入的是一次性实例，上下文 store 无人填充，
    // 订阅页读 store.plans 会永远空——列表必须自己拉。
    let mut plans = use_signal(Vec::<PlanRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写回成功后 +1 触发重拉：后端按 sort_order 排序，本地插入无法保证位次。
    let mut reload = use_signal(|| 0u32);
    // 「升级分组」下拉候选项（真实分组名）。
    let mut group_names = use_signal(Vec::<String>::new);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            // 分组名先拉（快、小）：失败不阻塞套餐列表，下拉退化到只有「不升级」。
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

    // 写反馈三信号（loading / error / ok）
    let mut saving = use_signal(|| false);
    let mut action_err = use_signal(|| None::<String>);
    let mut ok_msg = use_signal(|| None::<String>);

    // ---- Tab 0 基本信息：名称 / 价格 / 币种 / 额度 ----
    let mut f_title = use_signal(String::new);
    let mut f_price = use_signal(|| "0".to_string());
    let mut f_currency = use_signal(|| "CNY".to_string());
    let mut f_quota = use_signal(|| "0".to_string());
    // ---- Tab 1 规则与周期：启用 / 有效期天数 / 升级分组 / 限购 ----
    let mut f_duration = use_signal(|| "30".to_string());
    let mut f_group = use_signal(|| "不升级".to_string());
    let mut f_limit = use_signal(|| "0".to_string());
    let mut f_enabled = use_signal(|| true);

    /// 以某行现有值重建 upsert 请求体（启停切换用）：后端按 name 定位行
    /// 并整体回写这些列，返回更新后的视图。price 取规范字符串形式。
    fn rebuild_req(p: &PlanRow, enabled: bool) -> SubscriptionUpsertRequest {
        SubscriptionUpsertRequest {
            name: p.title.clone(),
            price: format!("{}", p.price),
            currency: p.currency.clone(),
            // duration_days 后端要求 >= 1；防御非法存量行（0 天）。
            duration_days: p.period_val.max(1),
            // quota 已是展示口径，原样回传（后端入库侧自己 ×500_000）。
            quota: p.quota,
            upgrade_group: if p.group.is_empty() || p.group == "不升级" {
                None
            } else {
                Some(p.group.clone())
            },
            // 0 = 不限 → None（后端 max_purchases 列可空）。
            max_purchases: if p.max_per_user > 0 {
                Some(p.max_per_user)
            } else {
                None
            },
            enabled: Some(enabled),
        }
    }

    let mut open_edit = move |i: usize| {
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

    let commit = move |_| {
        let name = f_title().trim().to_string();
        if name.is_empty() {
            action_err.set(Some("套餐名称必填".into()));
            return;
        }
        let currency = f_currency();
        let price = f_price().trim().to_string();
        // 后端拒绝 duration_days == 0：非法/空输入钳到 1 而不是发 0 挨 400。
        let duration_days = f_duration()
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|&d| d >= 1)
            .unwrap_or(1);
        // quota 展示口径直传；空串 = 0（无额度套餐），但显式非法值（非数字/负数）
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
                        action_err.set(Some("额度必须是数字且不小于 0".into()));
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
                        "套餐已更新".into()
                    } else {
                        "套餐已创建".into()
                    }));
                    show_modal.set(false);
                    // 重拉全表：后端按 sort_order 排序，本地插入无法保证位次。
                    reload += 1;
                }
                Err(e) => {
                    saving.set(false);
                    action_err.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            role: "region",
            "aria-label": "订阅套餐管理",
            "data-testid": "subscriptions-page",

            // ---- error：写请求（upsert / delete）失败的红条 ----
            if let Some(e) = action_err() {
                div { class: "rounded-xl border border-red-800 bg-red-950/40 p-3 text-sm text-red-300",
                    "data-testid": "subscriptions-action-error",
                    "{e}"
                }
            }
            // ---- ok：写成功反馈（点击后立即可见，列表也已同步刷新） ----
            if let Some(m) = ok_msg() {
                div { class: "rounded-xl border border-emerald-800 bg-emerald-950/40 p-3 text-sm text-emerald-300",
                    "data-testid": "subscriptions-action-ok",
                    "{m}"
                }
            }
            // ---- loading：写请求在途（保存中…，弹窗保存按钮同时禁用） ----
            if saving() {
                div { class: "rounded-xl border border-zinc-700 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400",
                    "data-testid": "subscriptions-saving",
                    "正在与后端同步…"
                }
            }

            // 顶部栏: 提示横幅 + 新建按钮
            div { class: "flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-amber-500/5 px-4 py-3",
                div { class: "flex items-center gap-2 text-xs text-amber-300",
                    span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-amber-500/20 font-bold", "ℹ" }
                    span { "套餐按名称去重：同名保存即更新现有套餐，改名会新建一行" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-lg bg-amber-400 px-3.5 py-1.5 text-xs font-semibold text-zinc-950 transition-colors hover:bg-amber-300 shadow-sm",
                    "data-testid": "subscriptions-new",
                    onclick: open_new,
                    span { class: "text-sm", "+" }
                    "新建套餐"
                }
            }

            // ---- loading：首次/重拉在途（与写请求的 saving 指示区分开） ----
            if loading() {
                div { class: "rounded-lg border border-zinc-800 bg-zinc-900/60 p-6 text-center text-sm text-zinc-500",
                    "data-testid": "subscriptions-loading",
                    "正在加载订阅套餐…"
                }
            }
            // ---- error：列表拉取失败（与写请求的 action_err 红条区分开） ----
            if let Some(e) = err() {
                div { class: "rounded-xl border border-red-800 bg-red-950/40 p-3 text-sm text-red-300",
                    "data-testid": "subscriptions-load-error",
                    "加载失败：{e}"
                }
            }

            // ---- empty / data ----
            if !loading() && plans.read().is_empty() {
                div { class: "rounded-lg border border-dashed border-zinc-700 p-6 text-center text-sm text-zinc-500",
                    "data-testid": "subscriptions-empty",
                    "还没有订阅套餐。点击右上角「新建套餐」创建第一个。"
                }
            } else {
                // 单栏卡牌列表容器 (Web / 平板 / 手机统一一栏优雅排布)
                div { class: "flex flex-col gap-3",
                    "data-testid": "subscriptions-list",
                    for (i, p) in plans.read().iter().enumerate() {
                        {
                            // 预构建每行所需 owned 值：onclick 闭包在 rsx 构造
                            // 之后才触发，那时 plans.read() 的借用 guard 已释放，
                            // 闭包只能捕获 owned 数据（同 ChannelsPage 惯例）。
                            let title_txt = p.title.clone();
                            let price_sym = if p.currency == "USD" { "$" } else { "¥" };
                            let price_str = format!("{price_sym}{:.2}", p.price);
                            // 后端无 new-api 数字 id 列（恒 None）：徽标改用 UUID
                            // 前缀，保证每行有稳定可辨识的标识（hover 见完整 key）。
                            let badge_txt = p
                                .id
                                .map(|n| format!("#{n}"))
                                .unwrap_or_else(|| p.key.chars().take(8).collect());
                            let period_str = format!("{} 天", p.period_val);
                            let quota_str = if p.quota <= 0.0 {
                                "无限制".to_string()
                            } else {
                                format!("{}", p.quota)
                            };
                            let group_txt = if p.group.is_empty() {
                                "不升级".to_string()
                            } else {
                                p.group.clone()
                            };
                            let limit_txt = if p.max_per_user > 0 {
                                format!("{}", p.max_per_user)
                            } else {
                                "不限".to_string()
                            };
                            let cur_enabled = p.enabled;
                            let edit_idx = i;
                            let row_key = p.key.clone();
                            let del_key = p.key.clone();
                            let edit_id = format!("subscriptions-edit-{}", p.key);
                            let del_id = format!("subscriptions-delete-{}", p.key);
                            // 启停 = 以同一 name 重建 upsert 体（enabled 取反）：
                            // 后端按 name 定位行整体回写，返回更新后的视图。
                            let toggle_req = rebuild_req(p, !cur_enabled);
                            rsx! {
                                div {
                                    key: "{row_key}",
                                    class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/90 shadow-md",

                                    // 卡片头部行: 标识 + 标题 + 状态/分组徽标 + 操作按钮
                                    div { class: "flex flex-wrap items-start justify-between gap-2.5",
                                        div { class: "flex items-center gap-2.5 min-w-0 flex-1",
                                            span { class: "shrink-0 rounded-md border border-zinc-700/80 bg-zinc-800 px-2 py-0.5 text-xs font-mono font-bold text-zinc-300",
                                                title: "{row_key}",
                                                "{badge_txt}"
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
                                                on: cur_enabled,
                                                on_toggle: move |_| {
                                                    let client = ApiClient::shared().clone();
                                                    action_err.set(None);
                                                    ok_msg.set(None);
                                                    // clone 再进 async：事件处理闭包必须是 FnMut，
                                                    // 直接 move 会让它退化成 FnOnce（E0525）。
                                                    let req = toggle_req.clone();
                                                    let mut reload = reload;
                                                    spawn(async move {
                                                        match upsert_subscription_api(&client, &req).await {
                                                            Ok(_) => reload += 1,
                                                            Err(e) => action_err.set(Some(e.to_string())),
                                                        }
                                                    });
                                                },
                                            }
                                            button {
                                                class: "rounded-lg border border-zinc-700 bg-zinc-800 px-2.5 py-1 text-xs text-zinc-200 transition-colors hover:bg-zinc-700 hover:text-white",
                                                "data-testid": edit_id,
                                                onclick: move |_| open_edit(edit_idx),
                                                "编辑"
                                            }
                                            button {
                                                class: "rounded-lg border border-red-900/50 bg-red-950/20 px-2 py-1 text-xs text-red-400 transition-colors hover:bg-red-900/30 hover:text-red-300",
                                                "data-testid": del_id,
                                                onclick: move |_| {
                                                    let client = ApiClient::shared().clone();
                                                    action_err.set(None);
                                                    ok_msg.set(None);
                                                    let key = del_key.clone();
                                                    let mut reload = reload;
                                                    spawn(async move {
                                                        match delete_subscription_api(&client, &key).await {
                                                            Ok(()) => {
                                                                ok_msg.set(Some("套餐已删除".into()));
                                                                reload += 1;
                                                            }
                                                            Err(e) => action_err.set(Some(e.to_string())),
                                                        }
                                                    });
                                                },
                                                "✕"
                                            }
                                        }
                                    }

                                    // 关键指标条 — 只展示后端实际返回的字段
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
                                            span { class: "font-mono font-semibold text-amber-300", "{quota_str}" }
                                        }
                                        div {
                                            span { class: "text-[11px] text-zinc-500 block", "升级分组" }
                                            span { class: "text-zinc-300 font-medium", "{group_txt}" }
                                        }
                                        div {
                                            span { class: "text-[11px] text-zinc-500 block", "限购" }
                                            span { class: "text-zinc-400", "{limit_txt}" }
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
                            p { class: "mt-0.5 text-xs text-zinc-400",
                                if editing_idx().is_some() {
                                    "保存即按名称更新现有套餐"
                                } else {
                                    "新建后可在列表中启停、编辑或删除"
                                }
                            }
                        }
                        button {
                            class: "rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                            onclick: move |_| show_modal.set(false),
                            "✕"
                        }
                    }

                    // 写请求失败提示（名称必填 / 后端 400 校验失败）
                    if let Some(e) = action_err() {
                        div { class: "rounded-lg border border-red-800 bg-red-950/40 p-3 text-sm text-red-300",
                            "data-testid": "subscriptions-form-error",
                            "{e}"
                        }
                    }

                    // 弹窗内部 Tab 切换条（两个 Tab：字段即后端入参，见组件文档）
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
                    }

                    // ---- Tab 0: 基本信息 ----
                    if modal_tab() == 0 {
                        div { class: "space-y-4 pt-1",
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "套餐标题" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                    "data-testid": "subscriptions-form-title",
                                    value: "{f_title()}",
                                    placeholder: "例如：月度会员",
                                    oninput: move |e| f_title.set(e.value()),
                                }
                                p { class: "text-[11px] text-zinc-500", "名称唯一：同名保存会更新已有套餐，而非新建" }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "套餐价格" }
                                    input {
                                        r#type: "number",
                                        step: "0.01",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        "data-testid": "subscriptions-form-price",
                                        value: "{f_price()}",
                                        oninput: move |e| f_price.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "用户购买该套餐需支付的金额" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "计价币种" }
                                    select {
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        "data-testid": "subscriptions-form-currency",
                                        value: "{f_currency()}",
                                        onchange: move |e| f_currency.set(e.value()),
                                        option { value: "CNY", "CNY（¥）" }
                                        option { value: "USD", "USD（$）" }
                                    }
                                    p { class: "text-[11px] text-zinc-500", "决定列表价格符号；后端要求非空" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "套餐额度" }
                                    input {
                                        r#type: "number",
                                        step: "0.01",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        "data-testid": "subscriptions-form-quota",
                                        value: "{f_quota()}",
                                        oninput: move |e| f_quota.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "套餐包含的总额度；0 表示不限量" }
                                }
                            }
                        }
                    }

                    // ---- Tab 1: 规则与周期 ----
                    if modal_tab() == 1 {
                        div { class: "space-y-4 pt-1",
                            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                                span { class: "text-sm text-zinc-200 font-medium", "启用状态" }
                                ToggleSwitch { on: f_enabled(), on_toggle: move |_| f_enabled.set(!f_enabled()) }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4 pt-2",
                                label { class: "block space-y-1",
                                    span { class: "text-xs text-zinc-400", "有效期（天）" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        "data-testid": "subscriptions-form-duration",
                                        value: "{f_duration()}",
                                        oninput: move |e| f_duration.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "后端按天存储有效期；至少 1 天" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs text-zinc-400", "限购数量" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        "data-testid": "subscriptions-form-limit",
                                        value: "{f_limit()}",
                                        oninput: move |e| f_limit.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "单个用户可购买的次数；0 表示不限" }
                                }
                            }
                            label { class: "block space-y-1",
                                span { class: "text-xs text-zinc-400", "升级分组" }
                                select {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                    "data-testid": "subscriptions-form-group",
                                    value: "{f_group()}",
                                    onchange: move |e| f_group.set(e.value()),
                                    option { value: "不升级", "不升级" }
                                    for g in group_names.read().iter() {
                                        option { value: "{g}", "{g}" }
                                    }
                                }
                                p { class: "text-[11px] text-zinc-500", "购买该套餐后升级到该分组；「不升级」表示不改变分组" }
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
                            class: "rounded-xl bg-amber-400 px-5 py-2 text-xs font-bold text-zinc-950 hover:bg-amber-300 transition-colors shadow-lg shadow-amber-500/10 disabled:opacity-50 disabled:cursor-not-allowed",
                            "data-testid": "subscriptions-form-submit",
                            disabled: saving(),
                            onclick: commit,
                            if saving() { "保存中…" } else { "保存更改" }
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
