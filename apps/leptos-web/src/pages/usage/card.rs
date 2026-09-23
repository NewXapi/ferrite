use leptos::prelude::*;

use super::data::{UsageLog, fmt_num, fmt_quota};

/// 单条用量日志卡片。
#[component]
pub fn UsageCard(entry: UsageLog) -> impl IntoView {
    let model_color = match entry.model_name {
        "gpt-4o" | "gpt-4o-mini" => "bg-emerald-400",
        "claude-3.5-sonnet" | "claude-3-haiku" => "bg-purple-400",
        "deepseek-r1" => "bg-blue-400",
        "qwen2.5-72b" => "bg-orange-400",
        _ => "bg-zinc-400",
    };
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(entry.prompt_tokens),
        fmt_num(entry.completion_tokens)
    );
    let timing_str = if entry.use_time_ms > 0 {
        format!("{:.1}s", entry.use_time_ms as f64 / 1000.0)
    } else {
        "—".to_string()
    };
    let cost_str = fmt_quota(entry.quota);
    let model_name = entry.model_name;
    let created_at = entry.created_at;

    view! {
        <div class="w-full rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4 text-left">
            <div class="flex items-center gap-2">
                <span class=format!("h-2.5 w-2.5 shrink-0 rounded-full {model_color}")></span>
                <span class="truncate font-mono text-sm text-zinc-200">{model_name}</span>
            </div>
            <div class="mt-2 flex items-baseline justify-between gap-2">
                <span class="font-mono text-xs text-zinc-500">{created_at}</span>
                <span class="shrink-0 font-medium tabular-nums text-sm text-emerald-400">{cost_str}</span>
            </div>
            <div class="mt-3 space-y-1.5 text-xs">
                <div class="flex justify-between gap-2">
                    <span class="shrink-0 text-zinc-500">"Tokens"</span>
                    <span class="whitespace-nowrap font-medium tabular-nums text-zinc-200">{tokens_pair}</span>
                </div>
                <div class="flex justify-between gap-2">
                    <span class="shrink-0 text-zinc-500">"耗时"</span>
                    <span class="whitespace-nowrap tabular-nums text-zinc-400">{timing_str}</span>
                </div>
            </div>
        </div>
    }
}
