//! 充值记录区 — GET /api/user/topup/orders 三态渲染 (error 红边卡 / loading 骨架 /
//! 空态虚线 / 真数据)。数据与三态信号由 `RewardsPanel` 持有并经 `load_recharges`
//! 填充;本组件只负责渲染,订单倒序,provider 空串回落 manual,状态机字符串原样展示。

use dioxus::prelude::*;

use crate::api::TopupOrderView;
use crate::usage_support::{fmt_num, fmt_time};

/// 最近充值记录区:订单倒序,最近在前;`provider` 为空时展示 manual。
///
/// `recharges` 为 `None` 或空数组是正常空态,非错误;仅 `recharges_err` 非空才走错误红边卡。
#[component]
pub fn RechargesSection(
    recharges: Signal<Option<Vec<TopupOrderView>>>,
    recharges_loaded: Signal<bool>,
    recharges_err: Signal<String>,
) -> Element {
    rsx! {
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
            div { class: "mb-5 flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between",
                h3 { class: "text-sm font-medium text-zinc-200", "最近充值记录" }
                span { class: "text-xs text-zinc-500", "订单倒序,最近在前" }
            }
            if !recharges_err().is_empty() {
                ErrCard {
                    testid: "recharge-error",
                    what: "充值记录",
                    msg: recharges_err(),
                }
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
}

/// 错误态统一渲染:柔和红边卡片 (非满屏红),对齐 rewards 各区的诚实降级文案。
#[component]
fn ErrCard(testid: &'static str, what: &'static str, msg: String) -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-red-500/40 bg-zinc-900 p-4",
            "data-testid": testid,
            p { class: "text-sm text-red-300", "无法加载{what} (未登录或请求失败): {msg}" }
        }
    }
}
