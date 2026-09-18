//! 充值开单 / 兑换码充值区

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::api::{self, OpenTopupRequest, RedeemRequest, TopupOrderView, WalletView};
use crate::usage_support::{fmt_num, fmt_quota};

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

#[component]
pub fn TopupSection(
    wallet: Signal<Option<WalletView>>,
    wallet_loaded: Signal<bool>,
    // 充值开单状态
    mut order_currency: Signal<String>,
    mut order_amount: Signal<String>,
    mut order_busy: Signal<bool>,
    mut order_ok: Signal<Option<String>>,
    mut order_err: Signal<String>,
    mut order_provider: Signal<String>,
    mut order_pay_url: Signal<Option<String>>,
    // 兑换码充值状态
    mut redeem_code: Signal<String>,
    mut topup_busy: Signal<bool>,
    mut topup_ok: Signal<Option<String>>,
    mut topup_err: Signal<String>,
) -> Element {
    // 重新加载钱包的闭包 - 供子组件调用
    let reload_wallet = move || load_wallet(wallet, wallet_loaded, topup_err);

    // 开单币种候选 = 钱包内已有余额的币种;未加载时禁用 (不给假选项)。
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

    let closure_currency_options = std::rc::Rc::clone(&currency_options);

    let open_order = move |_| {
        if order_busy() {
            return;
        }
        order_err.set(String::new());
        order_ok.set(None);
        order_pay_url.set(None);
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
        let provider = order_provider();
        order_busy.set(true);
        let client = client::ApiClient::shared().clone();
        let req = OpenTopupRequest {
            user_key: w.user_key.clone(),
            currency,
            amount,
            provider: if provider == "manual" {
                None
            } else {
                Some(provider)
            },
        };
        let mut b = order_busy;
        let mut ok = order_ok;
        let mut er = order_err;
        let mut pay = order_pay_url;
        spawn(async move {
            match api::open_topup_api(&client, &req).await {
                Ok(order) => {
                    let url = order.payment_url.clone();
                    let msg = match (&order.order_id, url.is_some()) {
                        (Some(id), true) => format!("订单已创建({id}),完成支付后自动到账"),
                        (Some(id), false) => format!("订单已创建({id}),待管理员确认后入账"),
                        (None, _) => "订单已创建".to_string(),
                    };
                    ok.set(Some(msg));
                    pay.set(url);
                }
                Err(e) => er.set(e.to_string()),
            }
            b.set(false);
        });
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
                Ok(v) => {
                    let msg = match api::topup_credited_quota(&v) {
                        Some(q) => {
                            format!("兑换成功,已入账 {} 额度(约 {})", fmt_num(q), fmt_quota(q))
                        }
                        None => "兑换成功,兑换码已核销".into(),
                    };
                    ok.set(Some(msg));
                    code_s.set(String::new());
                    reload_wallet();
                }
                Err(e) => er.set(e.to_string()),
            }
            b.set(false);
        });
    };

    rsx! {
        // 充值开单 — provider 可选:epay 在线支付(跳转支付页) / manual 人工确认
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
            div { role: "group", "aria-label": "充值开单",
                p { class: "mb-1 text-sm font-medium text-zinc-100", "充值开单" }
                p { class: "mb-4 text-xs text-zinc-500",
                    "在线支付:开单后去支付页完成付款,到账自动入账;人工确认:管理员审核后入账"
                }
                div { class: "flex flex-col gap-3 sm:flex-row",
                    select {
                        class: "rounded-2xl border border-zinc-700 bg-zinc-950 px-4 py-3.5 text-sm focus:border-zinc-500 focus:outline-none",
                        "data-testid": "topup-provider",
                        "aria-label": "支付方式",
                        value: "{order_provider()}",
                        onchange: move |e| order_provider.set(e.value()),
                        option { value: "manual", "人工确认 (管理员入账)" }
                        option { value: "epay", "在线支付 (易支付)" }
                    }
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
                        placeholder:
                            if order_provider() == "epay" {
                                "充值金额 (人民币元,正整数)"
                            } else {
                                "充值金额 (币种单位,正整数)"
                            },
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
            if let Some(url) = order_pay_url() {
                a {
                    href: "{url}",
                    target: "_blank",
                    class: "mt-3 inline-flex items-center gap-1 rounded-2xl border border-emerald-700 bg-emerald-950 px-6 py-3 text-sm font-semibold text-emerald-300 transition-colors hover:bg-emerald-900",
                    "data-testid": "topup-pay-button",
                    "去支付 →"
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
                div { class: "flex flex-col gap-3 sm:flex_row",
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
    }
}