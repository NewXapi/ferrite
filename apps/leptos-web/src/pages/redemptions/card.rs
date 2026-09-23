use crate::ui::{Card, CardContent, CardHeader};
use leptos::prelude::*;

use super::data::RedemptionCardData;

#[component]
pub fn StatusBadge(status: u8) -> impl IntoView {
    let (text, color_class) = match status {
        1 => (
            "未使用",
            "bg-emerald-500/20 text-emerald-400 border-emerald-500/30",
        ),
        2 => ("已核销", "bg-zinc-400/20 text-zinc-300 border-zinc-400/30"),
        3 => (
            "已停用",
            "bg-amber-500/20 text-amber-400 border-amber-500/30",
        ),
        _ => ("未知状态", "bg-red-500/20 text-red-400 border-red-500/30"),
    };

    view! {
        <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border {}", color_class)>
            {text}
        </span>
    }
}

// Quota indicator component
#[component]
pub fn QuotaIndicator(quota_cny: f64) -> impl IntoView {
    let width_class = {
        let normalized = (quota_cny / 1000.0).clamp(0.0, 1.0);
        let percentage = (normalized * 100.0) as u8;
        match percentage {
            0 => "w-0".to_string(),
            100 => "w-full".to_string(),
            _ => format!("w-[{}%]", percentage),
        }
    };

    let color_class = if quota_cny >= 500.0 {
        "bg-emerald-500"
    } else if quota_cny >= 100.0 {
        "bg-amber-500"
    } else if quota_cny > 0.0 {
        "bg-blue-500"
    } else {
        "bg-zinc-500"
    };

    view! {
        <div class="h-1.5 w-full rounded-full bg-zinc-800 overflow-hidden">
            <div class=format!("h-full {} transition-all duration-300 {}", color_class, width_class)></div>
        </div>
    }
}

// Redemption card component - adapted from dioxus RedemptionCard
#[component]
pub fn RedemptionCard(
    item: RedemptionCardData,
    on_copy: Callback<String>,
    on_disable: Callback<String>,
) -> impl IntoView {
    let (copy_button_text, copy_button_class) = (
        "复制预览",
        "bg-zinc-800/80 hover:bg-emerald-500/20 text-zinc-100 hover:text-emerald-300 border border-zinc-700 hover:border-emerald-500/30",
    );

    let disable_button_text = match item.status {
        1 => "停用",
        2 => "已核销",
        3 => "已停用",
        _ => "",
    };

    let disable_button_class = match item.status {
        1 => {
            "bg-zinc-800/80 hover:bg-red-500/20 text-zinc-100 hover:text-red-300 border border-zinc-700 hover:border-red-500/30"
        }
        _ => "bg-zinc-800/40 text-zinc-500 border border-zinc-800 cursor-not-allowed opacity-60",
    };

    view! {
        <Card
            attr:data-testid="redemption-card"
            attr:role="listitem"
            class="group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80"
        >
            // Header with avatar and status
            <CardHeader class="flex items-start gap-3 mb-3">
                <div class="h-9 w-9 rounded-full border border-zinc-700 bg-zinc-800 flex items-center justify-center">
                    <svg class="h-5 w-5 text-zinc-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-4.418 0-8 3.582-8 8s3.582 8 8 8 8-3.582 8-8-8-8zm0-8v4m0 4v4"></path>
                    </svg>
                </div>
                <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1">
                        <span class="font-mono text-sm text-zinc-100 truncate">{item.code_preview}</span>
                        <StatusBadge status=item.status/>
                    </div>
                    <div class="text-xs text-zinc-500 font-mono">"ID: " {item.key}</div>
                </div>
            </CardHeader>

            // Quota display
            {if item.quota_cny > 0.0 {
                view! {
                    <CardContent class="mb-3">
                        <div class="flex items-center justify-between mb-1">
                            <span class="text-xs text-zinc-400">
                                "面值 ¥" {format!("{:.2}", item.quota_cny)}
                            </span>
                            <span class="text-xs text-zinc-500">
                                {format!("{} 积分", ((item.quota_cny / 500000.0) * 100.0) as u8)}
                            </span>
                        </div>
                        <QuotaIndicator quota_cny=item.quota_cny/>
                    </CardContent>
                }.into_any()
            } else {
                view! { <span></span> }.into_any()
            }}

            // Metadata row
            <CardContent class="space-y-1 mb-4 text-xs">
                {if let Some(by) = item.redeemed_by {
                    view! {
                        <div class="flex items-center gap-2">
                            <span class="text-zinc-500">"兑换人:"</span>
                            <span class="text-zinc-100">{by}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                {if !item.redeemed_at.is_empty() {
                    view! {
                        <div class="flex items-center gap-2">
                            <span class="text-zinc-500">"核销时间:"</span>
                            <span class="text-zinc-100">{item.redeemed_at}</span>
                        </div>
                    }.into_any()
                } else {
                    view! { <span></span> }.into_any()
                }}
                <div class="flex items-center gap-2">
                    <span class="text-zinc-500">"生成时间:"</span>
                    <span class="text-zinc-100">{item.created}</span>
                </div>
            </CardContent>

            // Action buttons
            <CardContent class="mt-4 flex gap-1.5 border-t border-zinc-800 pt-3">
                <button
                    class=format!("flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", copy_button_class)
                    on:click=move |_| on_copy.run(item.key.to_string())
                >
                    {copy_button_text}
                </button>
                <button
                    class=format!("flex-1 px-3 py-2 rounded-lg text-sm font-medium transition-all duration-200 {}", disable_button_class)
                    on:click=move |_| on_disable.run(item.key.to_string())
                    disabled=item.status != 1
                >
                    {disable_button_text}
                </button>
            </CardContent>
        </Card>
    }
}

// Stats section - adapted from dioxus stats.rs
