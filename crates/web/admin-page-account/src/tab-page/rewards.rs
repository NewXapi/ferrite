//! 奖励面板 — 钱包 / 拉人统计 / 兑换码充值 / 充值开单 / 列表均接真实端点:
//! - 钱包: GET /api/user/wallet (多币种余额 + 折算 availableI64)
//! - 拉人统计: GET /api/affiliate/overview (inviteCount / totalReward;
//!   后端统计侧仍是占位值,0 即真实值,前端不造数)
//! - 兑换码: POST /api/user/topup `{"key"}` (CAS 核销入账) → 成功后刷新钱包
//! - 充值开单: POST /api/user/topup/orders — provider 可选 (manual 人工确认 /
//!   epay 在线支付);epay 开单返回 payment_url → 「去支付」跳转支付页,
//!   付款成功由 webhook 自动到账 (pending→paid);manual 仍只建 pending 单,
//!   入账需 admin 手工 settle (`POST /api/user/topup/{key}/settle`),故成功
//!   提示如实写「待管理员确认后入账」,不给假支付成功。
//! - 充值记录: GET /api/user/topup/orders (本人订单倒序,state 原样展示,
//!   provider 空串展示为 manual)
//! - 被邀人: GET /api/affiliate/invitees (joinedAt + 累计贡献奖励)
//!
//! 两块列表均三态渲染 (error 红边卡 / loading 骨架 / 空态虚线 / 真数据),
//! 空数组是正常空态。邀请链接由钱包 user_key 现拼,无需后端链接端点。

use dioxus::prelude::*;

use crate::api::{self, AffiliateOverviewView, InviteeView, TopupOrderView, WalletView};
use crate::components::{
    InviteSection, InviteesSection, RechargesSection, TopupSection, WalletSection,
};

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

#[component]
pub fn RewardsPanel() -> Element {
    let show_copied = use_signal(|| false);
    let redeem_code = use_signal(String::new);
    // 兑换码充值: 真实请求状态 (成功提示持久保留到下次操作, 失败诚实展示)
    let topup_busy = use_signal(|| false);
    let topup_ok = use_signal(|| None::<String>);
    let topup_err = use_signal(String::new);

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
    let order_currency = use_signal(String::new);
    let order_amount = use_signal(String::new);
    let order_busy = use_signal(|| false);
    let order_ok = use_signal(|| None::<String>);
    let order_err = use_signal(String::new);
    let order_provider = use_signal(|| "manual".to_string());
    let order_pay_url = use_signal(|| None::<String>);

    use_hook(move || {
        load_wallet(wallet, wallet_loaded, wallet_err);
        load_overview(overview, overview_loaded, overview_err);
        load_recharges(recharges, recharges_loaded, recharges_err);
        load_invitees(invitees, invitees_loaded, invitees_err);
    });

    rsx! {
        div { class: "flex flex-col gap-6",
            WalletSection {
                wallet: wallet,
                wallet_loaded: wallet_loaded,
                wallet_err: wallet_err,
            }

            TopupSection {
                wallet: wallet,
                wallet_loaded: wallet_loaded,
                order_currency: order_currency,
                order_amount: order_amount,
                order_busy: order_busy,
                order_ok: order_ok,
                order_err: order_err,
                order_provider: order_provider,
                order_pay_url: order_pay_url,
                redeem_code: redeem_code,
                topup_busy: topup_busy,
                topup_ok: topup_ok,
                topup_err: topup_err,
            }

            RechargesSection {
                recharges: recharges,
                recharges_loaded: recharges_loaded,
                recharges_err: recharges_err,
            }

            InviteSection {
                wallet: wallet,
                overview: overview,
                overview_loaded: overview_loaded,
                overview_err: overview_err,
                show_copied: show_copied,
            }

            InviteesSection {
                invitees: invitees,
                invitees_loaded: invitees_loaded,
                invitees_err: invitees_err,
            }
        }
    }
}
