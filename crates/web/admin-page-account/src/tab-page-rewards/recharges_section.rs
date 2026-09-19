//! 最近充值记录列表 — GET /api/user/topup/orders (真实端点,本人订单倒序)。
//!
//! 三态渲染 (error 红边卡 / loading 骨架 / 空态虚线 / 真数据);state 原样展示,
//! provider 空串展示为 manual。

use dioxus::prelude::*;

use crate::api::TopupOrderView;
use crate::usage_support::{fmt_num, fmt_time};

use super::shared::ErrCard;

#[component]
pub fn RechargesSection(
    recharges: Signal<Option<Vec<TopupOrderView>>>,
    recharges_loaded: Signal<bool>,
    recharges_err: Signal<String>,
) -> Element {
    rsx! {
        // 最近充值记录 — GET /api/user/topup/orders (真实端点)
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
            div { class: "mb-5 flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between",
                h3 { class: "text-sm font-medium text-zinc-200", "最近充值记录" }
                span { class: "text-xs text-zinc-500", "订单倒序,最近在前" }
            }
            if !recharges_err().is_empty() {
                ErrCard { testid: "recharge-error", what: "充值记录", msg: recharges_err() }
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
