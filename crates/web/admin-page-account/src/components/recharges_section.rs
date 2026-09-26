//! 最近充值记录列表 — GET /api/user/topup/orders (真实端点,本人订单倒序)。
//!
//! 三态渲染 (error 红边卡 / loading 骨架 / 空态虚线 / 真数据);state 原样展示,
//! provider 空串展示为 manual。

use dioxus::prelude::*;

use crate::api::TopupOrderView;
use crate::usage_support::{fmt_num, fmt_time};

use crate::components::ErrCard;

/// 【是什么】最近充值记录区段组件，展示本人充值订单 (时间 / 币种·渠道 / 金额 / 状态)。
///
/// 【做什么】渲染「最近充值记录」区：标题行 + 四态分发 (错误 / 骨架 / 空态 / 列表)；列表按后端倒序逐行展示，provider 空串展示为 manual，state 状态机字符串原样透传不做解释。不发请求。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】卡片 rounded-xl p-6 + hover:border-border；行 divide-y 分隔 py-4；金额 emerald-400 tabular-nums 右对齐；状态 10px 灰字。
///
/// 【子组件组成】ErrCard × 0 或 1
///
/// 【数据流】props 接收 recharges (Signal<Option<Vec<TopupOrderView>>>)、recharges_loaded (Signal<bool>)、recharges_err (Signal<String>)。无输出回调。
#[component]
pub fn RechargesSection(
    recharges: Signal<Option<Vec<TopupOrderView>>>,
    recharges_loaded: Signal<bool>,
    recharges_err: Signal<String>,
) -> Element {
    rsx! {
        // 最近充值记录 — GET /api/user/topup/orders (真实端点)
        section { class: "rounded-xl border border-border bg-card p-6 transition-colors hover:border-border",
            div { class: "mb-5 flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between",
                h3 { class: "{ui::TYPE_CARD_TITLE}", "最近充值记录" }
                span { class: "{ui::TYPE_DESC}", "订单倒序,最近在前" }
            }
            if !recharges_err().is_empty() {
                ErrCard { testid: "recharge-error", what: "充值记录", msg: recharges_err() }
            } else if !recharges_loaded() {
                div { class: "space-y-3", "data-testid": "recharge-skeleton",
                    div { class: "h-12 w-full animate-pulse rounded bg-secondary" }
                    div { class: "h-12 w-full animate-pulse rounded bg-secondary/70" }
                }
            } else if recharges().is_none_or(|r| r.is_empty()) {
                div {
                    class: "rounded-2xl border border-dashed border-border bg-background/40 py-8 text-center",
                    "data-testid": "recharge-empty",
                    p { class: "{ui::TYPE_BODY}", "暂无充值记录 (开单后待管理员确认入账)" }
                }
            } else if let Some(rows) = recharges() {
                div { class: "divide-y divide-zinc-800",
                    for o in &rows {
                        div { class: "flex justify-between py-4 {ui::TYPE_BODY} first:pt-0 last:pb-0",
                            "data-testid": format!("recharge-row-{}", o.key),
                            div {
                                div { class: "{ui::C_MUTED}", "{fmt_time(&o.created_at)}" }
                                div { class: "mt-0.5 {ui::TYPE_DESC}",
                                    if o.provider.is_empty() {
                                        "{o.currency.clone()} · manual"
                                    } else {
                                        "{o.currency.clone()} · {o.provider}"
                                    }
                                }
                            }
                            div { class: "text-right",
                                div { class: "font-medium {ui::C_SUCCESS} tabular-nums",
                                    "{fmt_num(o.amount)}"
                                }
                                // 状态机字符串原样展示,前端不解释
                                div { class: "mt-0.5 {ui::TYPE_LABEL}", "{o.state}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
