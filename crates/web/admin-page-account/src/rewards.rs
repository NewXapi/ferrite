//! 奖励面板 — 钱包 / 拉人统计 / 兑换码充值 / 充值开单 / 列表均接真实端点:
//! - 钱包: GET /api/user/wallet (多币种余额 + 折算 availableI64)
//! - 拉人统计: GET /api/affiliate/overview (inviteCount / totalReward;
//!   后端统计侧仍是占位值,0 即真实值,前端不造数)
//! - 兑换码: POST /api/user/topup `{"key"}` (CAS 核销入账) → 成功后刷新钱包
//! - 充值开单: POST /api/user/topup/orders — 支付 provider 为占位、无支付页,
//!   开单只建 pending 订单,入账需 admin 手工 settle
//!   (`POST /api/user/topup/{key}/settle`),故成功提示为「订单已创建,
//!   待管理员确认后入账」,不给假支付成功。
//! - 充值记录: GET /api/user/topup/orders (本人订单倒序,state 原样展示,
//!   provider 空串展示为 manual)
//! - 被邀人: GET /api/affiliate/invitees (joinedAt + 累计贡献奖励)
//!
//! 两块列表均三态渲染 (error 红边卡 / loading 骨架 / 空态虚线 / 真数据),
//! 空数组是正常空态。邀请链接由钱包 user_key 现拼,无需后端链接端点。

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::api::{
    self, AffiliateOverviewView, InviteeView, OpenTopupRequest, RedeemRequest, TopupOrderView,
    WalletView,
};
use crate::usage_support::{fmt_num, fmt_quota, fmt_time};

/// 拉取钱包 (GET /api/user/wallet) 并写回三个 Signal。
/// 首载与兑换码入账后的刷新共用此入口;`Signal` 是 Rc 句柄 (Copy),按值传。
fn load_wallet(
    mut w: Signal<Option<WalletView>>,
    mut loaded: Signal<bool>,
    mut err: Signal<String>,
) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::fetch_wallet_api(&client).await {
            Ok(v) => {
                err.set(String::new());
                w.set(Some(v));
                loaded.set(true);
            }
            Err(e) => err.set(e.to_string()),
        }
    });
}

/// 拉取拉人统计 (GET /api/affiliate/overview) 并写回三个 Signal。
fn load_overview(
    mut o: Signal<Option<AffiliateOverviewView>>,
    mut loaded: Signal<bool>,
    mut err: Signal<String>,
) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::fetch_affiliate_overview_api(&client).await {
            Ok(v) => {
                err.set(String::new());
                o.set(Some(v));
                loaded.set(true);
            }
            Err(e) => err.set(e.to_string()),
        }
    });
}

/// 拉取充值记录 (GET /api/user/topup/orders) 并写回三个 Signal。
fn load_recharges(
    mut r: Signal<Option<Vec<TopupOrderView>>>,
    mut loaded: Signal<bool>,
    mut err: Signal<String>,
) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::fetch_recharges_api(&client).await {
            Ok(v) => {
                err.set(String::new());
                r.set(Some(v));
                loaded.set(true);
            }
            Err(e) => err.set(e.to_string()),
        }
    });
}

/// 拉取被邀人 (GET /api/affiliate/invitees) 并写回三个 Signal。
fn load_invitees(
    mut i: Signal<Option<Vec<InviteeView>>>,
    mut loaded: Signal<bool>,
    mut err: Signal<String>,
) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::fetch_invitees_api(&client).await {
            Ok(v) => {
                err.set(String::new());
                i.set(Some(v));
                loaded.set(true);
            }
            Err(e) => err.set(e.to_string()),
        }
    });
}

/// 错误态统一渲染:柔和红边卡片 (非满屏红),对齐 keys.rs 的诚实降级文案。
fn err_card(testid: &'static str, what: &'static str, msg: String) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-red-500/40 bg-zinc-900 p-4",
            "data-testid": testid,
            p { class: "text-sm text-red-300", "无法加载{what} (未登录或请求失败): {msg}" }
        }
    }
}

/// 当前页面 origin (`https://host[:port]`),非 wasm / 无 window 时返回空串,
/// 邀请链接退化为相对路径,仍可被注册页同源解析。
fn current_origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}

#[component]
pub fn RewardsPanel() -> Element {
    let mut show_copied = use_signal(|| false);
    let mut redeem_code = use_signal(String::new);
    // 兑换码充值: 真实请求状态 (成功提示持久保留到下次操作, 失败诚实展示)
    let mut topup_busy = use_signal(|| false);
    let mut topup_ok = use_signal(|| None::<String>);
    let mut topup_err = use_signal(String::new);

    // ---- 钱包 (GET /api/user/wallet): loading skeleton / error 红边卡 / 空态虚线 ----
    let wallet = use_signal(|| None::<WalletView>);
    let wallet_loaded = use_signal(|| false);
    let wallet_err = use_signal(String::new);

    // ---- 拉人统计 (GET /api/affiliate/overview) ----
    let overview = use_signal(|| None::<AffiliateOverviewView>);
    let overview_loaded = use_signal(|| false);
    let overview_err = use_signal(String::new);

    // ---- 充值记录 (GET /api/user/topup/orders) ----
    let recharges = use_signal(|| None::<Vec<TopupOrderView>>);
    let recharges_loaded = use_signal(|| false);
    let recharges_err = use_signal(String::new);

    // ---- 被邀人 (GET /api/affiliate/invitees) ----
    let invitees = use_signal(|| None::<Vec<InviteeView>>);
    let invitees_loaded = use_signal(|| false);
    let invitees_err = use_signal(String::new);

    // ---- 充值开单 (POST /api/user/topup/orders, pending 单) ----
    let mut order_currency = use_signal(String::new);
    let mut order_amount = use_signal(String::new);
    let mut order_busy = use_signal(|| false);
    let mut order_ok = use_signal(|| None::<String>);
    let mut order_err = use_signal(String::new);

    use_hook(move || {
        load_wallet(wallet, wallet_loaded, wallet_err);
        load_overview(overview, overview_loaded, overview_err);
        load_recharges(recharges, recharges_loaded, recharges_err);
        load_invitees(invitees, invitees_loaded, invitees_err);
    });

    // 邀请链接 = 当前站点 origin + 本人 user_key (钱包加载后才有,未加载时留空,
    // 链接区显示占位文案,不造假链接)。
    let invite_link = match wallet() {
        Some(w) => api::invite_link(&current_origin(), &w.user_key),
        None => String::new(),
    };
    // 闭包要持有链接,rsx 也要渲染;String 不能 Copy,clone 一份给闭包。
    let copy_invite_link = invite_link.clone();

    // 开单币种候选 = 钱包内已有余额的币种;未加载时禁用 (不给假选项)。
    // Rc 共享：open_order 闭包与 rsx 渲染都要读，Vec 不能 Copy。
    let currency_options: std::rc::Rc<Vec<String>> = std::rc::Rc::new(match wallet() {
        Some(w) if w.balances.is_empty() => vec!["FREE".to_string()],
        Some(w) => w.balances.iter().map(|b| b.currency_code.clone()).collect(),
        None => vec![],
    });
    let order_currency_active = if order_currency().is_empty() {
        currency_options.first().cloned().unwrap_or_default()
    } else {
        order_currency()
    };

    let copy_link = move |_| {
        if !copy_invite_link.is_empty() && ui::copy_text_to_clipboard(&copy_invite_link) {
            show_copied.set(true);
            spawn(async move {
                TimeoutFuture::new(2_000).await;
                show_copied.set(false);
            });
        }
    };

    let redeem = move |_| {
        if topup_busy() {
            return;
        }
        let code = redeem_code().trim().to_string();
        if code.is_empty() {
            topup_err.set("请先输入兑换码".into());
            topup_ok.set(None);
            return;
        }
        topup_busy.set(true);
        topup_err.set(String::new());
        topup_ok.set(None);
        let client = client::ApiClient::shared().clone();
        let req = RedeemRequest { key: code };
        let mut code_s = redeem_code;
        let mut b = topup_busy;
        let mut ok = topup_ok;
        let mut er = topup_err;
        spawn(async move {
            match api::redeem_code_api(&client, &req).await {
                // 后端成功响应 {"quota": <内部额度单位>, "success": true}:
                // 用真实入账值提示; 缺字段时降级为通用文案, 不假造数值。
                Ok(v) => {
                    let msg = match api::topup_credited_quota(&v) {
                        Some(q) => {
                            format!("兑换成功,已入账 {} 额度(约 {})", fmt_num(q), fmt_quota(q))
                        }
                        None => "兑换成功,兑换码已核销".into(),
                    };
                    ok.set(Some(msg));
                    code_s.set(String::new());
                    // 入账改变余额 → 刷新钱包区 (失败时钱包自带错误态)。
                    load_wallet(wallet, wallet_loaded, wallet_err);
                }
                Err(e) => er.set(e.to_string()),
            }
            b.set(false);
        });
    };

    // 闭包内读 signal 现值（而非捕获快照）：下单时才解析币种，
    // 避免 String 被 move 进闭包导致外层渲染拿不到值。
    // Rc 先 clone 一份给闭包，rsx 侧保留原引用。
    let closure_currency_options = std::rc::Rc::clone(&currency_options);
    let open_order = move |_| {
        if order_busy() {
            return;
        }
        order_err.set(String::new());
        order_ok.set(None);
        // 钱包未就绪时拿不到本人 user_key (后端也只允许对本人开单)。
        let Some(w) = wallet() else {
            order_err.set("钱包未加载,无法开单".into());
            return;
        };
        let amount: i64 = match order_amount().trim().parse() {
            Ok(n) if n > 0 => n,
            _ => {
                order_err.set("请输入正整数充值金额".into());
                return;
            }
        };
        let currency = if order_currency().is_empty() {
            closure_currency_options
                .first()
                .cloned()
                .unwrap_or_default()
        } else {
            order_currency()
        };
        if currency.is_empty() {
            order_err.set("无可用充值币种".into());
            return;
        }
        order_busy.set(true);
        let client = client::ApiClient::shared().clone();
        let req = OpenTopupRequest {
            user_key: w.user_key.clone(),
            currency,
            amount,
        };
        let mut b = order_busy;
        let mut ok = order_ok;
        let mut er = order_err;
        spawn(async move {
            match api::open_topup_api(&client, &req).await {
                // provider 占位:只建 pending 单,入账走 admin 手工 settle,
                // 提示如实写「待管理员确认」,不假装支付已完成。
                Ok(order) => {
                    let msg = match order.order_id {
                        Some(id) => format!("订单已创建({id}),待管理员确认后入账"),
                        None => "订单已创建,待管理员确认后入账".to_string(),
                    };
                    ok.set(Some(msg));
                }
                Err(e) => er.set(e.to_string()),
            }
            b.set(false);
        });
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                    // 钱包区
                    section {
                        id: "rewards-sec-wallet",
                        class: "scroll-mt-8 space-y-4",
                        role: "region",
                        "aria-label": "钱包",
                        h2 { class: "text-lg font-medium text-zinc-100", "钱包" }

                        // 余额卡 — 三态: error 红边 / loading 骨架 / 数据(空余额虚线占位)
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            if !wallet_err().is_empty() {
                                {err_card("wallet-error", "钱包", wallet_err())}
                            } else if !wallet_loaded() {
                                div { class: "space-y-3", "data-testid": "wallet-skeleton",
                                    div { class: "h-10 w-48 animate-pulse rounded bg-zinc-800" }
                                    div { class: "h-4 w-32 animate-pulse rounded bg-zinc-800/70" }
                                    div { class: "h-4 w-24 animate-pulse rounded bg-zinc-800/50" }
                                }
                            } else if let Some(w) = wallet() {
                                div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                                    div {
                                        p { class: "text-xs text-zinc-500", "可用额度 (内部单位折算)" }
                                        p {
                                            class: "mt-1 text-6xl font-semibold tracking-tighter text-emerald-400 tabular-nums",
                                            "data-testid": "wallet-available",
                                            "{fmt_num(w.available_i64)}"
                                        }
                                        p { class: "mt-1 text-xl text-zinc-400", "≈ {fmt_quota(w.available_i64)}" }
                                    }
                                    div { class: "flex items-center gap-2 self-start rounded-3xl bg-emerald-950/80 px-5 py-2 text-xs font-medium text-emerald-400",
                                        span { class: "text-lg leading-none text-emerald-400", "●" }
                                        "已连后端"
                                    }
                                }
                                // 多币种余额逐行: symbol (空则回退 code) + 该币种单位余额
                                if w.balances.is_empty() {
                                    div {
                                        class: "mt-6 rounded-2xl border border-dashed border-zinc-700 bg-zinc-950/40 py-8 text-center",
                                        "data-testid": "wallet-empty",
                                        p { class: "text-sm text-zinc-500", "暂无币种余额 (新账号未 seed 或已全部消耗)" }
                                    }
                                } else {
                                    div { class: "mt-6 divide-y divide-zinc-800 border-t border-zinc-800",
                                        for b in &w.balances {
                                            div {
                                                class: "flex justify-between py-3 text-sm first:pt-0 last:pb-0",
                                                "data-testid": format!("wallet-balance-{}", b.currency_code),
                                                span { class: "text-zinc-400",
                                                    if b.symbol.is_empty() { "{b.currency_code.clone()}" } else { "{b.symbol.clone()}" }
                                                }
                                                span { class: "font-medium text-zinc-100 tabular-nums", "{fmt_num(b.amount)}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 充值开单 — provider 占位无支付页,建 pending 单等 admin settle
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            div { role: "group", "aria-label": "充值开单",
                                p { class: "mb-1 text-sm font-medium text-zinc-100", "充值开单" }
                                p { class: "mb-4 text-xs text-zinc-500",
                                    "在线支付通道未开通:开单仅生成待确认订单,管理员确认后入账"
                                }
                                div { class: "flex flex-col gap-3 sm:flex-row",
                                    select {
                                        class: "rounded-2xl border border-zinc-700 bg-zinc-950 px-4 py-3.5 text-sm focus:border-zinc-500 focus:outline-none disabled:opacity-50",
                                        "data-testid": "topup-currency",
                                        "aria-label": "充值币种",
                                        value: "{order_currency_active}",
                                        onchange: move |e| order_currency.set(e.value()),
                                        disabled: currency_options.is_empty(),
                                        if currency_options.is_empty() {
                                            option { value: "", "钱包未加载" }
                                        } else {
                                            for code in currency_options.iter() {
                                                option { value: "{code}", "{code}" }
                                            }
                                        }
                                    }
                                    input {
                                        r#type: "number",
                                        class: "flex-1 rounded-2xl border border-zinc-700 bg-zinc-950 px-5 py-3.5 text-sm placeholder:text-zinc-500 focus:border-zinc-500 outline-none",
                                        placeholder: "充值金额 (币种单位,正整数)",
                                        min: "1",
                                        value: order_amount(),
                                        "data-testid": "topup-amount",
                                        oninput: move |e| order_amount.set(e.value()),
                                    }
                                    button {
                                        class: "w-full shrink-0 rounded-2xl border border-zinc-600 bg-zinc-800 px-8 py-3.5 text-sm font-semibold text-zinc-100 transition-colors hover:bg-zinc-700 disabled:cursor-not-allowed disabled:opacity-50 sm:w-auto",
                                        onclick: open_order,
                                        disabled: order_busy() || wallet().is_none(),
                                        "data-testid": "topup-order-submit",
                                        "aria-label": "充值开单",
                                        if order_busy() { "开单中…" } else { "创建充值订单" }
                                    }
                                }
                            }
                            if let Some(msg) = order_ok() {
                                p { class: "mt-4 flex items-center gap-2 text-sm text-emerald-400",
                                    "data-testid": "topup-order-result",
                                    "{msg}"
                                }
                            }
                            if !order_err().is_empty() {
                                p { class: "mt-4 text-sm text-red-400",
                                    "data-testid": "topup-order-error",
                                    "开单失败: {order_err()}"
                                }
                            }
                        }

                        // 兑换码充值
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            div { role: "group", "aria-label": "兑换码充值",
                                p { class: "mb-4 text-sm font-medium text-zinc-100", "兑换码充值" }
                                div { class: "flex flex-col gap-3 sm:flex-row",
                                    input {
                                        class: "flex-1 rounded-2xl border border-zinc-700 bg-zinc-950 px-5 py-3.5 text-sm placeholder:text-zinc-500 focus:border-zinc-500 outline-none",
                                        placeholder: "请输入兑换码",
                                        value: redeem_code(),
                                        "data-testid": "topup-code",
                                        oninput: move |e| redeem_code.set(e.value()),
                                    }
                                    button {
                                        class: "w-full shrink-0 rounded-2xl bg-white px-10 py-3.5 text-sm font-semibold text-zinc-900 transition-colors hover:bg-zinc-100 sm:w-auto",
                                        onclick: redeem,
                                        disabled: topup_busy(),
                                        "data-testid": "topup-submit",
                                        "aria-label": "兑换",
                                        if topup_busy() { "兑换中…" } else { "兑换" }
                                    }
                                }
                            }
                            if let Some(msg) = topup_ok() {
                                p { class: "mt-4 flex items-center gap-2 text-sm text-emerald-400",
                                    "data-testid": "topup-result",
                                    "{msg}"
                                }
                            }
                            if !topup_err().is_empty() {
                                p { class: "mt-4 text-sm text-red-400",
                                    "data-testid": "topup-error",
                                    "兑换失败: {topup_err()}"
                                }
                            }
                        }

                        // 最近充值记录 — GET /api/user/topup/orders (真实端点)
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            div { class: "mb-5 flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between",
                                h3 { class: "text-sm font-medium text-zinc-200", "最近充值记录" }
                                span { class: "text-xs text-zinc-500", "订单倒序,最近在前" }
                            }
                            if !recharges_err().is_empty() {
                                {err_card("recharge-error", "充值记录", recharges_err())}
                            } else if !recharges_loaded() {
                                div { class: "space-y-3", "data-testid": "recharge-skeleton",
                                    div { class: "h-12 w-full animate-pulse rounded bg-zinc-800" }
                                    div { class: "h-12 w-full animate-pulse rounded bg-zinc-800/70" }
                                }
                            } else if recharges().is_none_or(|r| r.is_empty()) {
                                div {
                                    class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-950/40 py-8 text-center",
                                    "data-testid": "recharge-empty",
                                    p { class: "text-sm text-zinc-500", "暂无充值记录 (开单后待管理员确认入账)" }
                                }
                            } else if let Some(rows) = recharges() {
                                div { class: "divide-y divide-zinc-800",
                                    for o in &rows {
                                        div { class: "flex justify-between py-4 text-sm first:pt-0 last:pb-0",
                                            "data-testid": format!("recharge-row-{}", o.key),
                                            div {
                                                div { class: "text-zinc-400", "{fmt_time(&o.created_at)}" }
                                                div { class: "mt-0.5 text-xs text-zinc-500",
                                                    if o.provider.is_empty() {
                                                        "{o.currency.clone()} · manual"
                                                    } else {
                                                        "{o.currency.clone()} · {o.provider}"
                                                    }
                                                }
                                            }
                                            div { class: "text-right",
                                                div { class: "font-medium text-emerald-400 tabular-nums",
                                                    "{fmt_num(o.amount)}"
                                                }
                                                // 状态机字符串原样展示,前端不解释
                                                div { class: "mt-0.5 text-[10px] text-zinc-500", "{o.state}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // 邀请区
                    section { id: "rewards-sec-invite", class: "scroll-mt-8 space-y-4",
                        h2 { class: "text-lg font-medium text-zinc-100", "邀请" }

                        // 邀请链接 — origin + 钱包 user_key 现拼;钱包未加载时占位
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            h3 { class: "mb-4 text-sm font-medium text-zinc-200", "邀请好友得奖励" }
                            div { class: "flex flex-col gap-3 sm:flex-row",
                                div {
                                    class: "flex-1 break-all rounded-2xl border border-zinc-700 bg-zinc-950 px-5 py-4 font-mono text-sm text-zinc-400",
                                    "data-testid": "invite-link",
                                    if invite_link.is_empty() {
                                        "钱包加载后生成邀请链接"
                                    } else {
                                        "{invite_link}"
                                    }
                                }
                                button {
                                    class: "w-full shrink-0 rounded-2xl bg-white px-8 py-4 font-medium text-zinc-900 transition-colors hover:bg-amber-200 active:bg-amber-300 sm:w-auto",
                                    onclick: copy_link,
                                    disabled: invite_link.is_empty(),
                                    "data-testid": "invite-copy",
                                    "aria-label": "复制邀请链接",
                                    if show_copied() { "已复制 ✓" } else { "复制链接" }
                                }
                            }
                        }

                        // 拉人统计 (GET /api/affiliate/overview) — 三态同钱包区
                        section {
                            class: "grid grid-cols-1 gap-3 md:grid-cols-3",
                            role: "region",
                            "aria-label": "拉人统计",
                            if !overview_err().is_empty() {
                                {err_card("affiliate-error", "拉人统计", overview_err())}
                            } else if !overview_loaded() {
                                for _ in 0..2 {
                                    div {
                                        class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                                        "data-testid": "affiliate-skeleton",
                                        div { class: "h-8 w-20 animate-pulse rounded bg-zinc-800" }
                                        div { class: "mt-3 h-4 w-28 animate-pulse rounded bg-zinc-800/70" }
                                    }
                                }
                            } else if let Some(ov) = overview() {
                                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6 transition-colors hover:border-zinc-600",
                                    p {
                                        class: "text-4xl font-semibold tracking-tight text-amber-300 tabular-nums",
                                        "data-testid": "affiliate-invite-count",
                                        "{fmt_num(ov.invite_count)}"
                                    }
                                    p { class: "mt-3 text-sm font-medium text-zinc-100", "已邀人数" }
                                    p { class: "mt-6 text-xs leading-snug text-zinc-500", "通过邀请完成注册的用户数" }
                                }
                                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6 transition-colors hover:border-zinc-600",
                                    p {
                                        class: "text-4xl font-semibold tracking-tight text-amber-300 tabular-nums",
                                        "data-testid": "affiliate-total-reward",
                                        "{fmt_num(ov.total_reward)}"
                                    }
                                    p { class: "mt-3 text-sm font-medium text-zinc-100", "累计奖励 (内部单位)" }
                                    p { class: "mt-6 text-xs leading-snug text-zinc-500", "≈ {fmt_quota(ov.total_reward)} · 拉人奖励累计" }
                                }
                            }
                        }
                    }

                    // 被邀人列表 — GET /api/affiliate/invitees (真实端点)
                    section { id: "rewards-sec-list", class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                        div { class: "mb-2 flex items-center justify-between",
                            h3 { class: "text-sm font-medium text-zinc-200", "被邀请用户" }
                            div { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                                "data-testid": "invitee-count",
                                "{invitees().map_or(0, |v| v.len())} 人"
                            }
                        }
                        if !invitees_err().is_empty() {
                            {err_card("invitee-error", "被邀人", invitees_err())}
                        } else if !invitees_loaded() {
                            div { class: "space-y-3", "data-testid": "invitee-skeleton",
                                div { class: "h-16 w-full animate-pulse rounded-2xl bg-zinc-800" }
                                div { class: "h-16 w-full animate-pulse rounded-2xl bg-zinc-800/70" }
                            }
                        } else if invitees().is_none_or(|v| v.is_empty()) {
                            div {
                                class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-950/40 py-8 text-center",
                                "data-testid": "invitee-empty",
                                p { class: "text-sm text-zinc-500", "暂无被邀请用户 (通过链接注册后在此显示)" }
                            }
                        } else if let Some(rows) = invitees() {
                            div { class: "space-y-3",
                                for i in &rows {
                                    div { class: "group flex items-center gap-4 rounded-2xl border border-zinc-800 bg-zinc-950 p-5 hover:border-amber-900",
                                        "data-testid": format!("invitee-row-{}", i.user_key),
                                        div { class: "flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-amber-900 to-zinc-700 text-xl font-semibold text-amber-200",
                                            "{i.name.chars().next().unwrap_or_default()}"
                                        }
                                        div { class: "min-w-0 flex-1",
                                            div { class: "font-medium text-zinc-100 group-hover:text-amber-100",
                                                if i.name.is_empty() { "(未命名用户)" } else { "{i.name}" }
                                            }
                                            div { class: "mt-0.5 text-xs text-zinc-500",
                                                "注册时间:{fmt_time(&i.joined_at)}"
                                            }
                                        }
                                        div { class: "text-right",
                                            div { class: "font-semibold text-emerald-400 tabular-nums",
                                                "{fmt_num(i.reward)}"
                                            }
                                            div { class: "mt-px text-[10px] text-zinc-500", "贡献奖励" }
                                        }
                                    }
                                }
                            }
                        }
                    }
            }
    }
}
