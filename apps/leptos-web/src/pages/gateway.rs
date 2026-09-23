//! 网关页面 - 基于 dioxus admin-page-admin 的 tab-page-gateway 设计
//! 仅包含演示数据和 UI结构，不调用任何 API
use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

/// 演示用网关健康数据
fn default_gateway_health() -> Vec<crate::wire::GatewayHealthView> {
    use crate::wire::{GatewayHealthItem, GatewayHealthView, HealthItemState};

    vec![
        GatewayHealthView {
            items: vec![
                GatewayHealthItem {
                    unit_key: "channel-abc123:def-model".to_string(),
                    channel_key: Some("chan-abc123".to_string()),
                    channel_name: Some("默认渠道A".to_string()),
                    public_model: Some("gpt-4".to_string()),
                    state: HealthItemState::Cooling,
                    last_cooling_outcome: Some("throttled".to_string()),
                    remaining_cooldown_ms: 120_000, // 2 minutes
                    slow_start_factor: 0.3,
                },
                GatewayHealthItem {
                    unit_key: "channel-xyz789:gemini-pro".to_string(),
                    channel_key: Some("chan-xyz789".to_string()),
                    channel_name: None,
                    public_model: Some("gemini-pro".to_string()),
                    state: HealthItemState::SlowStart,
                    last_cooling_outcome: None,
                    remaining_cooldown_ms: 0,
                    slow_start_factor: 0.8,
                },
                GatewayHealthItem {
                    unit_key: "channel-qwerty:openai".to_string(),
                    channel_key: Some("chan-qwerty".to_string()),
                    channel_name: Some("OpenAI 通道".to_string()),
                    public_model: None,
                    state: HealthItemState::Ok,
                    last_cooling_outcome: None,
                    remaining_cooldown_ms: 0,
                    slow_start_factor: 0.0,
                },
            ],
        },
        GatewayHealthView {
            items: vec![GatewayHealthItem {
                unit_key: "channel-1111:claude".to_string(),
                channel_key: Some("chan-1111".to_string()),
                channel_name: None,
                public_model: None,
                state: HealthItemState::Ok,
                last_cooling_outcome: None,
                remaining_cooldown_ms: 0,
                slow_start_factor: 0.0,
            }],
        },
    ]
}

/// 网关渠道健康单行 - 基于 dioxus tab-page-gateway/row.rs 设计
#[component]
pub fn GatewayHealthRow(item: crate::wire::GatewayHealthItem) -> impl IntoView {
    let name = {
        let channel_key = item.channel_key.clone();
        let unit_key = item.unit_key.clone();
        move || {
            if let Some(name) = item.channel_name.as_deref().filter(|n| !n.is_empty()) {
                name.to_string()
            } else {
                channel_key
                    .as_deref()
                    .map(|k| k.chars().take(8).collect::<String>())
                    .unwrap_or_else(|| format!("{}", unit_key.chars().take(6).collect::<String>()))
            }
        }
    };

    let model = {
        let public_model = item.public_model.clone();
        let unit_key = item.unit_key.clone();
        move || {
            public_model.clone().unwrap_or_else(|| {
                unit_key
                    .rsplit_once(':')
                    .map(|(_, m)| m.to_string())
                    .unwrap_or_else(|| "未知模型".to_string())
            })
        }
    };

    let tone = move || match item.state {
        crate::wire::HealthItemState::Cooling => crate::wire::TONE_COOLING,
        crate::wire::HealthItemState::SlowStart => crate::wire::TONE_SLOW_START,
        crate::wire::HealthItemState::Ok => crate::wire::TONE_OK,
    };

    let state_text = item.state.label();
    let remaining = move || {
        if item.state == crate::wire::HealthItemState::Cooling && item.remaining_cooldown_ms > 0 {
            Some(item.remaining_cooldown_ms / 1000)
        } else {
            None
        }
    };
    let outcome = item.last_cooling_outcome.clone();

    view! {
        div {
            class: "flex flex-wrap items-center gap-x-2 gap-y-1 py-2.5 first:pt-1 last:pb-1",
            "data-testid": "gateway-health-row",
            // 渠道名 (channelName,缺省 channelKey 前 8 位)
            span { class: "min-w-0 truncate text-sm font-medium text-zinc-100",
                title: "{item.unit_key}", {name()} }
            // 模型 Badge
            span { class: "rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300",
                "{model()}" }
            // 三态 Badge (cooling=红 / slow_start=黄 / ok=绿)
            span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone()}",
                "{state_text}" }
            // 冷却中才显示倒计时秒
            {if let Some(sec) = remaining() {
                view! {
                    span { class: "font-mono text-xs text-red-300",
                        {format!("余 {}s", sec)} }
                }
            } else {
                view! { }
            }}
            // lastCoolingOutcome (有则小字)
            {if let Some(o) = outcome {
                view! {
                    span { class: "text-[11px] text-zinc-500", "{o}" }
                }
            } else {
                view! { }
            }}
        }
    }
}

#[component]
pub fn GatewayPage() -> impl IntoView {
    // 面板状态:全部只服务本组件
    let items = RwSignal::new(default_gateway_health());
    let loading = RwSignal::new(false);
    let err = RwSignal::new(String::new());

    // 手动刷新计数:每次点「刷新/重试」自增
    let mut reload = RwSignal::new(0u32);

    // 根据是否存在 cooling / slow_start 项决定是否需要轮询
    let polling = Memo::new(move |_| {
        items()
            .iter()
            .flat_map(|view| view.items.iter())
            .any(|i| i.state != crate::wire::HealthItemState::Ok)
    });

    // 计算三个状态的计数
    let cooling_count = Memo::new(move |_| {
        items()
            .iter()
            .flat_map(|view| view.items.iter())
            .filter(|i| i.state == crate::wire::HealthItemState::Cooling)
            .count()
    });

    let slow_count = Memo::new(move |_| {
        items()
            .iter()
            .flat_map(|view| view.items.iter())
            .filter(|i| i.state == crate::wire::HealthItemState::SlowStart)
            .count()
    });

    let ok_count = Memo::new(move |_| {
        items()
            .iter()
            .flat_map(|view| view.items.iter())
            .filter(|i| i.state == crate::wire::HealthItemState::Ok)
            .count()
    });

    let list = Memo::new(move |_| {
        items()
            .into_iter()
            .flat_map(|view| view.items.clone())
            .collect::<Vec<_>>()
    });

    view! {
        CardGrid {
            // 网关页面整体容器
            section {
                class: "flex flex-col gap-3",
                role: "region",
                "aria-label": "网关渠道健康",
                "data-testid": "gateway-health-panel",

                // 标题 + 统计 + 手动刷新 (交互元素带 data-testid)
                div { class: "flex flex-wrap items-center justify-between gap-2",
                    div { class: "flex items-center gap-2",
                        h2 { class: "text-lg font-medium text-zinc-100", "网关渠道健康" }
                        span { class: "rounded-full bg-zinc-800 px-2 py-0.5 text-[11px] text-zinc-400",
                            {if polling() { "轮询中 · 5s" } else { "已同步" }}
                        }
                    }
                    div { class: "flex flex-wrap items-center gap-1.5 text-[11px]",
                        span { class: "rounded-full border px-2 py-0.5 border-red-500/30 bg-red-500/15 text-red-300",
                            {"冷却 ", cooling_count()} }
                        span { class: "rounded-full border px-2 py-0.5 border-amber-500/30 bg-amber-500/15 text-amber-300",
                            {"慢启动 ", slow_count()} }
                        span { class: "rounded-full border px-2 py-0.5 border-emerald-500/30 bg-emerald-500/15 text-emerald-400",
                            {"正常 ", ok_count()} }
                        Button {
                            button_type: "button",
                            class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                            "data-testid": "refresh-gateway-health",
                            on:click: move |_| reload.set(reload() + 1),
                            "刷新"
                        }
                    }
                }

                // 卡片面板容器 (禁 table;rounded-xl border bg-card divide-y + flex-wrap 行)
                div {
                    class: "rounded-xl border border-zinc-800 bg-card p-4 divide-y divide-zinc-800",
                    "data-testid": "gateway-health-list",

                    if !err().is_empty() {
                        // 错误态:柔和红边卡 (非满屏红),保留重试入口
                        div { class: "rounded-lg border border-red-900/50 bg-red-950/20 px-4 py-6 text-center",
                            "data-testid": "gateway-health-error",
                            p { class: "text-sm text-red-300", "网关健康拉取失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{err()}" }
                            Button {
                                button_type: "button",
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                "data-testid": "retry-gateway-health",
                                on:click: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        // loading=skeleton (animate-pulse 骨架行 ×3)
                        for _ in 0..3 {
                            div { class: "flex flex-wrap items-center gap-2 py-3 first:pt-1 last:pb-1",
                                div { class: "h-4 w-32 animate-pulse rounded bg-zinc-800" }
                                div { class: "h-4 w-24 animate-pulse rounded bg-zinc-800/70" }
                                div { class: "h-4 w-16 animate-pulse rounded bg-zinc-800/50" }
                            }
                        }
                    } else if list().is_empty() {
                        // 空态:虚线占位卡 (正常态,后端只返回有记录渠道)
                        div { class: "rounded-lg border border-dashed border-zinc-700 bg-zinc-900/40 px-4 py-8 text-center",
                            "data-testid": "gateway-health-empty",
                            p { class: "text-sm text-zinc-400", "暂无渠道健康记录——正常态" }
                            p { class: "mt-1 text-xs text-zinc-500",
                                "网关只上报发生过错的渠道;全部健康时列表为空" }
                        }
                    } else {
                        // 数据态:每个上报过错的渠道一行;行渲染与状态徽标在 GatewayHealthRow,
                        // 页面只负责拉取/轮询与四态分支。
                        for item in list() {
                            {
                                view! {
                                    crate::pages::gateway::GatewayHealthRow { key: "{item.unit_key}", item }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
