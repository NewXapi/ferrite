//! 网关渠道健康面板:实时观测 gateway dispatch 的渠道冷却 / 慢启动状态。
//!
//! 数据源:`GET /api/gateway/health` (admin 白名单,见
//! `admin-observe/gateway_health.rs`)。响应**只含有记录的渠道**——
//! 正常态是空数组,不是错误。
//!
//! 渲染约定(仓库铁律):
//! - 卡片面板模式,禁 `<table>`:`rounded-xl border bg-card divide-y`
//!   容器 + `flex flex-wrap` 行;
//! - 每行:渠道名 (channelName,缺省 channelKey 前 8 位) + 模型 Badge
//!   + 三态 Badge (cooling=红 / slow_start=黄 / ok=绿,禁 emoji)
//!   + 冷却剩余秒 + lastCoolingOutcome 小字;
//! - 三态:loading=skeleton、error=柔和红边卡、empty=虚线占位卡;
//! - 轮询:挂载即拉一次;仅当存在 cooling / slow_start 条目时按 5s
//!   间隔继续轮询,全 ok 或空时停止 (防无意义轮询)。
//!
//! 交互元素带 `data-testid`,容器带 `role` + `aria-label`
//! (仓库 UI 验证约定,PR smoke 走 ariaSnapshot)。

use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

use client::fetch_gateway_health;
use client::{ApiClient, GatewayHealthItem, HealthItemState};

/// 轮询间隔 (仅存在 cooling / slow_start 条目时)。
const POLL_INTERVAL_MS: u32 = 5000;

/// 三态 Badge 语义色 (shadcn dashboard 既有 token 风格,禁 emoji):
/// cooling=红系 / slow_start=黄系 / ok=绿系。
const TONE_COOLING: &str = "border-red-500/30 bg-red-500/15 text-red-300";
const TONE_SLOW_START: &str = "border-amber-500/30 bg-amber-500/15 text-amber-300";
const TONE_OK: &str = "border-emerald-500/30 bg-emerald-500/15 text-emerald-400";

/// 渠道名回退:channelName 缺省显示 channelKey 前 8 位。
fn display_name(item: &GatewayHealthItem) -> String {
    if let Some(name) = item.channel_name.as_deref()
        && !name.is_empty()
    {
        return name.to_string();
    }
    item.channel_key
        .as_deref()
        .map(|k| k.chars().take(8).collect())
        .unwrap_or_else(|| {
            format!(
                "未同步渠道 ({}…)",
                item.unit_key.chars().take(6).collect::<String>()
            )
        })
}

/// 剩余冷却秒 (冷却中才显示倒计时数字;非冷却 = 0 不展示)。
fn remaining_seconds(item: &GatewayHealthItem) -> Option<u64> {
    if item.state == HealthItemState::Cooling && item.remaining_cooldown_ms > 0 {
        Some(item.remaining_cooldown_ms / 1000)
    } else {
        None
    }
}

/// 模型徽标文案:publicModel 缺省回退 unitKey 末段。
fn model_label(item: &GatewayHealthItem) -> String {
    item.public_model.clone().unwrap_or_else(|| {
        item.unit_key
            .rsplit_once(':')
            .map(|(_, m)| m.to_string())
            .unwrap_or_else(|| "未知模型".to_string())
    })
}

/// 三态 Badge 语义色 (match 而非 if-chain,类型收敛为 `&'static str`)。
fn state_tone(state: HealthItemState) -> &'static str {
    match state {
        HealthItemState::Cooling => TONE_COOLING,
        HealthItemState::SlowStart => TONE_SLOW_START,
        HealthItemState::Ok => TONE_OK,
    }
}

/// 网关渠道健康面板。
///
/// 挂载即拉一次;响应含 cooling / slow_start 条目时按 5s 间隔轮询,
/// 全 ok 或空时停。错误显示柔和红边卡 (非满屏红),不影响其他页面。
///
/// 轮询纪律:`use_effect` 依赖 `reload` 计数,循环内联 async (信号句柄
/// 直接 move 进 `spawn` 块,无闭包自续),全 ok / 空时循环 break 停止。
/// 「刷新」「重试」按钮自增 `reload` 重启循环,避免网络抖动摘掉
/// 冷却中的渠道。
///
/// # 用法
/// ```ignore
/// GatewayHealthPanel {}
/// ```
#[component]
pub fn GatewayHealthPanel() -> Element {
    let items = use_signal(Vec::<GatewayHealthItem>::new);
    let loading = use_signal(|| true);
    let err = use_signal(|| None::<String>);

    // 手动刷新计数:每次点「刷新/重试」自增,use_effect 依赖它重跑循环。
    let mut reload = use_signal(|| 0u32);

    // 轮询循环:挂载或手动刷新时启动一次 (reload 不变则 effect 不重跑,
    // 无重复循环)。每轮拉快照,有 cooling / slow_start 则 5s 后继续,
    // 无 (全 ok / 空) 则停止,防无意义轮询。错误时保留快照的轮询意愿,
    // 网络抖动不应把冷却中的渠道从面板上摘掉。
    use_effect(move || {
        let _ = reload();
        let mut items_sig = items;
        let mut loading_sig = loading;
        let mut err_sig = err;
        spawn(async move {
            loop {
                let client = ApiClient::shared().clone();
                let keep_polling = match fetch_gateway_health(&client).await {
                    Ok(view) => {
                        items_sig.set(view.items.clone());
                        err_sig.set(None);
                        loading_sig.set(false);
                        view.items.iter().any(|i| i.state != HealthItemState::Ok)
                    }
                    Err(e) => {
                        err_sig.set(Some(e.to_string()));
                        loading_sig.set(false);
                        items_sig
                            .read()
                            .iter()
                            .any(|i| i.state != HealthItemState::Ok)
                    }
                };
                if !keep_polling {
                    break;
                }
                TimeoutFuture::new(POLL_INTERVAL_MS).await;
            }
        });
    });

    let list = items();
    let cooling_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::Cooling)
        .count();
    let slow_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::SlowStart)
        .count();
    let ok_count = list
        .iter()
        .filter(|i| i.state == HealthItemState::Ok)
        .count();
    let polling = cooling_count + slow_count > 0;

    rsx! {
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
                        if polling { "轮询中 · 5s" } else { "已同步" }
                    }
                }
                div { class: "flex flex-wrap items-center gap-1.5 text-[11px]",
                    span { class: "rounded-full border border-red-500/30 bg-red-500/15 px-2 py-0.5 text-red-300",
                        "冷却 {cooling_count}" }
                    span { class: "rounded-full border border-amber-500/30 bg-amber-500/15 px-2 py-0.5 text-amber-300",
                        "慢启动 {slow_count}" }
                    span { class: "rounded-full border border-emerald-500/30 bg-emerald-500/15 px-2 py-0.5 text-emerald-400",
                        "正常 {ok_count}" }
                    button {
                        class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-1 text-xs text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                        "data-testid": "refresh-gateway-health",
                        onclick: move |_| reload.set(reload() + 1),
                        "刷新"
                    }
                }
            }

            // 卡片面板容器 (禁 table;rounded-xl border bg-card divide-y + flex-wrap 行)
            div {
                class: "rounded-xl border border-zinc-800 bg-card p-4 divide-y divide-zinc-800",
                "data-testid": "gateway-health-list",

                if let Some(e) = err() {
                    // 错误态:柔和红边卡 (非满屏红),保留重试入口
                    div { class: "rounded-lg border border-red-900/50 bg-red-950/20 px-4 py-6 text-center",
                        "data-testid": "gateway-health-error",
                        p { class: "text-sm text-red-300", "网关健康拉取失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "retry-gateway-health",
                            onclick: move |_| reload.set(reload() + 1),
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
                } else if list.is_empty() {
                    // 空态:虚线占位卡 (正常态,后端只返回有记录渠道)
                    div { class: "rounded-lg border border-dashed border-zinc-700 bg-zinc-900/40 px-4 py-8 text-center",
                        "data-testid": "gateway-health-empty",
                        p { class: "text-sm text-zinc-400", "暂无渠道健康记录——正常态" }
                        p { class: "mt-1 text-xs text-zinc-500",
                            "网关只上报发生过错的渠道;全部健康时列表为空" }
                    }
                } else {
                    for item in list {
                        {
                            let name = display_name(&item);
                            let model = model_label(&item);
                            let tone = state_tone(item.state);
                            let state_text = item.state.label().to_string();
                            let remaining = remaining_seconds(&item);
                            let outcome = item.last_cooling_outcome.clone();
                            rsx! {
                                div {
                                    key: "{item.unit_key}",
                                    class: "flex flex-wrap items-center gap-x-2 gap-y-1 py-2.5 first:pt-1 last:pb-1",
                                    "data-testid": "gateway-health-row",
                                    // 渠道名 (channelName,缺省 channelKey 前 8 位)
                                    span { class: "min-w-0 truncate text-sm font-medium text-zinc-100",
                                        title: "{item.unit_key}", "{name}" }
                                    // 模型 Badge
                                    span { class: "rounded-full border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300",
                                        "{model}" }
                                    // 三态 Badge (cooling=红 / slow_start=黄 / ok=绿)
                                    span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
                                        "{state_text}" }
                                    // 冷却中才显示倒计时秒
                                    if let Some(sec) = remaining {
                                        span { class: "font-mono text-xs text-red-300",
                                            "余 {sec}s" }
                                    }
                                    // lastCoolingOutcome (有则小字)
                                    if let Some(o) = outcome {
                                        span { class: "text-[11px] text-zinc-500", "{o}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
