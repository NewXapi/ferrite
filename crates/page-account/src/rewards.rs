use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;
use ui::ScrollSpyNav;

use crate::api::{self, Invitee, Recharge, RewardStat};

#[component]
pub fn RewardsPanel() -> Element {
    let mut show_copied = use_signal(|| false);
    let mut redeem_code = use_signal(|| String::new());
    let mut show_success = use_signal(|| false);

    let wallet = api::fetch_wallet();
    let recharges = api::fetch_recharges();
    let stats = api::fetch_reward_stats();
    let invitees = api::fetch_invitees();
    let invite_link = api::fetch_invite_link();

    let copy_link = move |_| {
        show_copied.set(true);
        spawn(async move {
            TimeoutFuture::new(2_000).await;
            show_copied.set(false);
        });
    };

    let redeem = move |_| {
        if !redeem_code().trim().is_empty() {
            show_success.set(true);
            redeem_code.set(String::new());
            spawn(async move {
                TimeoutFuture::new(2_000).await;
                show_success.set(false);
            });
        }
    };

    rsx! {
        div { class: "pl-8",
            ScrollSpyNav {
                container: "panel-scroll",
                items: vec![
                    ("钱包".to_string(), "rewards-sec-wallet".to_string()),
                    ("邀请".to_string(), "rewards-sec-invite".to_string()),
                    ("被邀人".to_string(), "rewards-sec-list".to_string()),
                ],
            }

            div { class: "flex flex-col gap-6",
                    // 钱包区
                    section { id: "rewards-sec-wallet", class: "scroll-mt-8 space-y-4",
                        h2 { class: "text-lg font-medium text-zinc-100", "钱包" }

                        // 钱包大卡 + 兑换码充值
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            div { class: "flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between",
                                div {
                                    p { class: "text-xs text-zinc-500", "当前余额" }
                                    p { class: "mt-1 text-6xl font-semibold tracking-tighter text-emerald-400", "{wallet.balance}" }
                                    p { class: "mt-1 text-xl text-zinc-400", "{wallet.currency}" }
                                }
                                div { class: "flex items-center gap-2 self-start rounded-3xl bg-emerald-950/80 px-5 py-2 text-xs font-medium text-emerald-400",
                                    span { class: "text-lg leading-none text-emerald-400", "●" }
                                    "可用"
                                }
                            }

                            div { class: "mt-10 border-t border-dashed border-zinc-700 pt-6",
                                p { class: "mb-4 text-sm font-medium text-zinc-100", "兑换码充值" }
                                div { class: "flex flex-col gap-3 sm:flex-row",
                                    input {
                                        class: "flex-1 rounded-2xl border border-zinc-700 bg-zinc-950 px-5 py-3.5 text-sm placeholder:text-zinc-500 focus:border-zinc-500 outline-none",
                                        placeholder: "请输入兑换码",
                                        value: redeem_code(),
                                        oninput: move |e| redeem_code.set(e.value()),
                                    }
                                    button {
                                        class: "w-full shrink-0 rounded-2xl bg-white px-10 py-3.5 text-sm font-semibold text-zinc-900 transition-colors hover:bg-zinc-100 sm:w-auto",
                                        onclick: redeem,
                                        "立即充值"
                                    }
                                }
                                if show_success() {
                                    p { class: "mt-4 flex items-center gap-2 text-sm text-emerald-400",
                                        "充值成功,余额已更新"
                                    }
                                }
                            }
                        }

                        // 最近充值记录
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            div { class: "mb-5 flex flex-col gap-1 sm:flex-row sm:items-baseline sm:justify-between",
                                h3 { class: "text-sm font-medium text-zinc-200", "最近充值记录" }
                                span { class: "text-xs text-zinc-500", "仅展示最近 3 笔" }
                            }
                            div { class: "divide-y divide-zinc-800",
                                for Recharge { date, method, amount } in recharges {
                                    div { class: "flex justify-between py-4 text-sm first:pt-0 last:pb-0",
                                        div {
                                            div { class: "text-zinc-400", "{date}" }
                                            div { class: "mt-0.5 text-xs text-zinc-500", "{method}" }
                                        }
                                        div { class: "text-right font-medium text-emerald-400", "{amount} {wallet.currency}" }
                                    }
                                }
                            }
                        }
                    }

                    // 邀请区
                    section { id: "rewards-sec-invite", class: "scroll-mt-8 space-y-4",
                        h2 { class: "text-lg font-medium text-zinc-100", "邀请" }

                        // 邀请链接
                        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                            h3 { class: "mb-4 text-sm font-medium text-zinc-200", "邀请好友得奖励" }
                            div { class: "flex flex-col gap-3 sm:flex-row",
                                div {
                                    class: "flex-1 break-all rounded-2xl border border-zinc-700 bg-zinc-950 px-5 py-4 font-mono text-sm text-zinc-400",
                                    "{invite_link}"
                                }
                                button {
                                    class: "w-full shrink-0 rounded-2xl bg-white px-8 py-4 font-medium text-zinc-900 transition-colors hover:bg-amber-200 active:bg-amber-300 sm:w-auto",
                                    onclick: copy_link,
                                    if show_copied() { "已复制 ✓" } else { "复制链接" }
                                }
                            }
                            p { class: "mt-4 text-xs text-zinc-500", "通过此链接注册的用户将为您贡献奖励分成,实时到账钱包" }
                        }

                        // 奖励统计 - 1/3/5 栅格
                        section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                            for RewardStat { value, label, desc } in stats {
                                div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-6 transition-colors hover:border-zinc-600",
                                    p { class: "text-4xl font-semibold tracking-tight text-amber-300 tabular-nums", "{value}" }
                                    p { class: "mt-3 text-sm font-medium text-zinc-100", "{label}" }
                                    p { class: "mt-6 text-xs leading-snug text-zinc-500", "{desc}" }
                                }
                            }
                        }
                    }

                    // 被邀人列表 - 纯卡片式
                    section { id: "rewards-sec-list", class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900 p-6",
                        div { class: "mb-6 flex items-center justify-between",
                            h3 { class: "text-sm font-medium text-zinc-200", "被邀请用户" }
                            div { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400", "{invitees.len()} 人" }
                        }
                        div { class: "space-y-3",
                            for Invitee { name, date, reward } in invitees {
                                div { class: "group flex items-center gap-4 rounded-2xl border border-zinc-800 bg-zinc-950 p-5 hover:border-amber-900",
                                    div { class: "flex h-10 w-10 flex-shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br from-amber-900 to-zinc-700 text-xl font-semibold text-amber-200",
                                        "{name.chars().next().unwrap_or_default()}"
                                    }
                                    div { class: "min-w-0 flex-1",
                                        div { class: "font-medium text-zinc-100 group-hover:text-amber-100", "{name}" }
                                        div { class: "mt-0.5 text-xs text-zinc-500", "注册时间:{date}" }
                                    }
                                    div { class: "text-right",
                                        div { class: "font-semibold text-emerald-400", "{reward}" }
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
