use crate::ui::{Card, CardGrid, Dialog, DialogTrigger};
use leptos::prelude::*;
use singlestage::*;

const DEMO_USER_KEY: &str = "user_0xA1B2";
const DEMO_USER_NAME: &str = "测试用户";
const DEMO_AFF_CODE: &str = "INVITE42";
const DEMO_AVAILABLE: i64 = 12480;

#[derive(Clone)]
struct DemoBalance {
    symbol: &'static str,
    amount: i64,
}

const DEMO_BALANCES: &[DemoBalance] = &[
    DemoBalance {
        symbol: "USDT",
        amount: 8750,
    },
    DemoBalance {
        symbol: "BTC",
        amount: 42,
    },
];

const DEMO_INVITE_COUNT: i32 = 17;
const DEMO_TOTAL_REWARD: i64 = 3240;

#[derive(Clone)]
struct DemoInvitee {
    name: &'static str,
    joined: &'static str,
    reward: i64,
}

const DEMO_INVITEES: &[DemoInvitee] = &[
    DemoInvitee {
        name: "小明",
        joined: "3天前",
        reward: 420,
    },
    DemoInvitee {
        name: "小红",
        joined: "1周前",
        reward: 180,
    },
    DemoInvitee {
        name: "小刚",
        joined: "2周前",
        reward: 650,
    },
];

#[derive(Clone)]
struct DemoRecharge {
    time: &'static str,
    amount: i64,
    status: &'static str,
}

const DEMO_RECHARGES: &[DemoRecharge] = &[
    DemoRecharge {
        time: "今天 14:22",
        amount: 500,
        status: "已到账",
    },
    DemoRecharge {
        time: "昨天 09:15",
        amount: 2000,
        status: "处理中",
    },
];

#[component]
pub fn RewardsPage() -> impl IntoView {
    let open = RwSignal::new(false);

    view! {
        <div class="p-6 space-y-8 max-w-6xl mx-auto">
            <h1 class="text-3xl font-bold text-white">"奖励中心"</h1>

            <CardGrid>
                <Card>
                    <div class="p-6">
                        <div class="flex justify-between items-start">
                            <div>
                                <div class="text-sm text-zinc-400">"可用余额"</div>
                                <div class="text-5xl font-semibold text-emerald-400 tabular-nums mt-1">{DEMO_AVAILABLE}</div>
                                <div class="text-xs text-zinc-500 mt-1">"≈ ¥87,360"</div>
                            </div>
                            <div class="px-4 py-2 bg-emerald-950 text-emerald-400 text-sm rounded-full">"已连接"</div>
                        </div>

                        <div class="mt-8 grid grid-cols-2 gap-4">
                            {DEMO_BALANCES.iter().map(|b| view! {
                                <div class="bg-zinc-900 rounded-xl p-4">
                                    <div class="text-zinc-400 text-sm">{b.symbol}</div>
                                    <div class="text-2xl font-medium text-white mt-1 tabular-nums">{b.amount}</div>
                                </div>
                            }).collect_view()}
                        </div>
                    </div>
                </Card>

                <Card>
                    <div class="p-6">
                        <div class="flex items-center justify-between mb-6">
                            <div class="text-lg font-medium">"邀请奖励"</div>
                            <DialogTrigger on:click=move |_| open.set(true)>
                                <button type="button" class="text-xs px-5 py-2 bg-zinc-800 hover:bg-zinc-700 rounded-full text-white transition">"邀请好友"</button>
                            </DialogTrigger>
                        </div>

                        <div class="flex gap-8">
                            <div>
                                <div class="text-4xl font-semibold text-amber-400">{DEMO_INVITE_COUNT}</div>
                                <div class="text-xs text-zinc-500">"已邀请"</div>
                            </div>
                            <div>
                                <div class="text-4xl font-semibold text-amber-400">{DEMO_TOTAL_REWARD}</div>
                                <div class="text-xs text-zinc-500">"总奖励"</div>
                            </div>
                        </div>

                        <div class="mt-8 pt-6 border-t border-zinc-800">
                            <div class="text-xs text-zinc-400 mb-2">"您的专属邀请码"</div>
                            <div class="font-mono bg-zinc-900 p-3 rounded-xl text-lg text-white tracking-widest">{DEMO_AFF_CODE}</div>
                            <button type="button" class="mt-4 w-full py-3 text-sm bg-white text-zinc-900 rounded-2xl hover:bg-zinc-100 transition">"复制链接"</button>
                        </div>
                    </div>
                </Card>
            </CardGrid>

            <div>
                <div class="flex justify-between items-center mb-4">
                    <div class="text-lg font-medium text-white">"被邀请用户"</div>
                    <div class="text-sm text-zinc-500">{DEMO_INVITEES.len()} "人"</div>
                </div>
                <CardGrid>
                    {DEMO_INVITEES.iter().map(|inv| view! {
                        <Card>
                            <div class="p-5">
                                <div class="flex items-center gap-3">
                                    <div class="w-9 h-9 bg-gradient-to-br from-amber-400 to-orange-500 rounded-2xl flex items-center justify-center text-white font-medium text-lg">
                                        {inv.name.chars().next().unwrap_or('客')}
                                    </div>
                                    <div class="flex-1">
                                        <div class="font-medium">{inv.name}</div>
                                        <div class="text-xs text-zinc-500">{inv.joined}</div>
                                    </div>
                                    <div class="text-right">
                                        <div class="text-emerald-400 font-semibold tabular-nums">+{inv.reward}</div>
                                        <div class="text-[10px] text-zinc-500">"奖励"</div>
                                    </div>
                                </div>
                            </div>
                        </Card>
                    }).collect_view()}
                </CardGrid>
            </div>

            <div>
                <div class="flex justify-between items-center mb-4">
                    <div class="text-lg font-medium text-white">"最近充值"</div>
                </div>
                <CardGrid>
                    {DEMO_RECHARGES.iter().map(|r| view! {
                        <Card>
                            <div class="p-5 flex justify-between items-center">
                                <div>
                                    <div class="text-sm text-zinc-400">{r.time}</div>
                                    <div class="text-white font-medium mt-0.5">"充值 {r.amount}"</div>
                                </div>
                                <div class="px-4 py-1 text-xs rounded-full bg-emerald-900 text-emerald-400">{r.status}</div>
                            </div>
                        </Card>
                    }).collect_view()}
                </CardGrid>
            </div>

            <Dialog title="邀请好友".to_string() open=open.get() on_confirm=move |_| open.set(false) on_cancel=move |_| open.set(false)>
                <p class="text-zinc-400">"分享您的邀请码 "<span class="font-mono text-white">{DEMO_AFF_CODE}</span>"，好友注册后双方均可获得奖励。"</p>
            </Dialog>
        </div>
    }
}
