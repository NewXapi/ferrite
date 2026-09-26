//! 钱包区 — GET /api/user/wallet (多币种余额 + 折算 availableI64)

use dioxus::prelude::*;

use crate::usage_support::{fmt_num, fmt_quota};

use crate::components::ErrCard;

/// 【是什么】钱包区段组件，展示可用额度折算值与多币种余额列表。
///
/// 【做什么】渲染「钱包」区：标题 + 余额卡三态分发 (ErrCard 错误 / 骨架 / 数据)；数据态先给可用额度大数字与「已连后端」徽标，再逐行列出各币种余额 (symbol 为空回退 currency_code)，余额为空时给虚线空态。不发请求。
///
/// 【交互逻辑】纯展示，无点击动作；卡片 hover 边框提亮。
///
/// 【样式】外层 scroll-mt-8 space-y-4；余额卡 rounded-xl 边框卡片 p-6 + hover:border-border；可用额度 text-6xl emerald-400 tabular-nums；多币种行 divide-y 分隔 py-3。
///
/// 【子组件组成】ErrCard × 0 或 1
///
/// 【数据流】props 接收 wallet (Signal<Option<WalletView>>)、wallet_loaded (Signal<bool>)、wallet_err (Signal<String>)。无输出回调。
#[component]
pub fn WalletSection(
    wallet: Signal<Option<crate::api::WalletView>>,
    wallet_loaded: Signal<bool>,
    wallet_err: Signal<String>,
) -> Element {
    rsx! {
        section {
            id: "rewards-sec-wallet",
            class: "scroll-mt-8 space-y-4",
            role: "region",
            "aria-label": "钱包",
            h2 { class: "{ui::TYPE_TITLE}", "钱包" }

            // 余额卡 — 三态: error 红边 / loading 骨架 / 数据(空余额虚线占位)
            section { class: "rounded-xl border border-border bg-card p-6 transition-colors hover:border-border",
                if !wallet_err().is_empty() {
                    ErrCard { testid: "wallet-error", what: "钱包", msg: wallet_err() }
                } else if !wallet_loaded() {
                    div { class: "space-y-3", "data-testid": "wallet-skeleton",
                        div { class: "h-10 w-48 animate-pulse rounded bg-secondary" }
                        div { class: "h-4 w-32 animate-pulse rounded bg-secondary/70" }
                        div { class: "h-4 w-24 animate-pulse rounded bg-secondary/50" }
                    }
                } else if let Some(w) = wallet() {
                    div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                        div {
                            p { class: "{ui::TYPE_DESC}", "可用额度 (内部单位折算)" }
                            p {
                                class: "mt-1 text-6xl font-semibold tracking-tighter {ui::C_SUCCESS} tabular-nums",
                                "data-testid": "wallet-available",
                                "{fmt_num(w.available_i64)}"
                            }
                            p { class: "mt-1 {ui::TYPE_VALUE}", "≈ {fmt_quota(w.available_i64)}" }
                        }
                        div { class: "flex items-center gap-2 self-start rounded-3xl bg-success px-5 py-2 {ui::TYPE_DESC} {ui::C_SUCCESS}",
                            span { class: "text-lg leading-none {ui::C_SUCCESS}", "●" }
                            "已连后端"
                        }
                    }
                    // 多币种余额逐行: symbol (空则回退 code) + 该币种单位余额
                    if w.balances.is_empty() {
                        div {
                            class: "mt-6 rounded-2xl border border-dashed border-border bg-background/40 py-8 text-center",
                            "data-testid": "wallet-empty",
                            p { class: "{ui::TYPE_BODY}", "暂无币种余额 (新账号未 seed 或已全部消耗)" }
                        }
                    } else {
                        div { class: "mt-6 divide-y divide-zinc-800 border-t border-border",
                            for b in &w.balances {
                                div {
                                    class: "flex justify-between py-3 {ui::TYPE_BODY} first:pt-0 last:pb-0",
                                    "data-testid": format!("wallet-balance-{}", b.currency_code),
                                    span { class: "{ui::C_MUTED}",
                                        if b.symbol.is_empty() { "{b.currency_code.clone()}" } else { "{b.symbol.clone()}" }
                                    }
                                    span { class: "font-medium text-foreground tabular-nums", "{fmt_num(b.amount)}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
