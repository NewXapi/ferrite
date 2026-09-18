//! 网关渠道健康单行:渠道展示名 + 模型 Badge + 三态 Badge + 冷却倒计时 +
//! lastCoolingOutcome 小字。纯展示组件,单项数据由
//! `page` 的 `GatewayHealthPanel` 遍历注入,无交互回调。
//!
//! 行级回退计算 (channelName / publicModel 缺省) 与三态取色都在本文件内;
//! 语义色常量与面板表头计数共用,见 `shared`。

use dioxus::prelude::*;

use client::{GatewayHealthItem, HealthItemState};

use super::shared::{TONE_COOLING, TONE_OK, TONE_SLOW_START};

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

/// 单条渠道健康行。
///
/// 渲染单个 [`GatewayHealthItem`]:渠道名 (缺省回退 channelKey 前 8 位) +
/// 模型 Badge + 三态 Badge (cooling=红 / slow_start=黄 / ok=绿) + 冷却
/// 剩余秒 + lastCoolingOutcome 小字。纯展示,无回调。
///
/// 由 `GatewayHealthPanel` 在数据态逐项渲染,`key` 取 `unit_key`。
#[component]
pub fn GatewayHealthRow(item: GatewayHealthItem) -> Element {
    let name = display_name(&item);
    let model = model_label(&item);
    let tone = state_tone(item.state);
    let state_text = item.state.label().to_string();
    let remaining = remaining_seconds(&item);
    let outcome = item.last_cooling_outcome.clone();

    rsx! {
        div {
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
