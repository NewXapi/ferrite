//! 邀请链接 / 拉人统计区

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use crate::api::{self, AffiliateOverviewView, WalletView};
use crate::usage_support::{fmt_num, fmt_quota};

use super::shared::ErrCard;

#[component]
pub fn InviteSection(
    wallet: Signal<Option<WalletView>>,
    overview: Signal<Option<AffiliateOverviewView>>,
    overview_loaded: Signal<bool>,
    overview_err: Signal<String>,
    mut show_copied: Signal<bool>,
) -> Element {
    fn current_origin() -> String {
        web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .unwrap_or_default()
    }

    let invite_link = match wallet() {
        Some(w) => api::invite_link(&current_origin(), &w.user_key, w.aff_code.as_deref()),
        None => String::new(),
    };
    let copy_invite_link = invite_link.clone();

    let copy_link = move |_| {
        if !copy_invite_link.is_empty() && ui::copy_text_to_clipboard(&copy_invite_link) {
            show_copied.set(true);
            spawn(async move {
                TimeoutFuture::new(2_000).await;
                show_copied.set(false);
            });
        }
    };

    rsx! {
        // 邀请区
        section { id: "rewards-sec-invite", class: "scroll-mt-8 space-y-4",
            h2 { class: "text-lg font-medium text-zinc-100", "邀请" }

            // 邀请链接 — origin + 钱包 user_key 现拼;钱包未加载时占位
            section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6 transition-colors hover:border-zinc-600",
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
                    ErrCard { testid: "affiliate-error", what: "拉人统计", msg: overview_err() }
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
    }
}
