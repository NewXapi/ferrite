use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, Dialog, DialogTrigger, CardHeader, CardTitle, CardContent};

#[derive(Clone)]
pub struct UsageLog {
    id: &'static str,
    model_name: &'static str,
    token_name: &'static str,
    channel_name: &'static str,
    created_at: &'static str,
    prompt_tokens: i64,
    completion_tokens: i64,
    use_time_ms: i64,
    quota: i64,
    is_stream: bool,
    ip: &'static str,
    request_id: &'static str,
}


#[component]
pub fn UsageCard(entry: UsageLog) -> impl IntoView {
    let model_color = match entry.model_name.as_str() {
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

    rsx! {
        Card {
            class: "w-full cursor-pointer rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4 text-left transition-colors hover:border-zinc-500 hover:bg-zinc-900",
            onclick: move |_| {},
            div { class: "flex items-center gap-2",
                span { class: "h-2.5 w-2.5 shrink-0 rounded-full {model_color}" }
                span { class: "truncate font-mono text-sm text-zinc-200", "{entry.model_name}" }
            }
            div { class: "mt-2 flex items-baseline justify-between gap-2",
                span { class: "font-mono text-xs text-zinc-500", "{entry.created_at}" }
                span { class: "shrink-0 font-medium tabular-nums text-sm text-emerald-400", "{cost_str}" }
            }
            div { class: "mt-3 space-y-1.5 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "Tokens" }
                    span { class: "whitespace-nowrap font-medium tabular-nums text-zinc-200",
                        "{tokens_pair}"
                    }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "耗时" }
                    span { class: "whitespace-nowrap tabular-nums text-zinc-400", "{timing_str}" }
                }
            }
        }
    }
}

