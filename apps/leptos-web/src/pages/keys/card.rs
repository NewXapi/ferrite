use crate::ui::{Card, CardContent, CardGrid, CardHeader, CardTitle, Dialog, DialogTrigger};
use leptos::prelude::*;
use singlestage::*;

#[derive(Clone, Copy)]
pub struct KeyItem {
    name: &'static str,
    key_preview: &'static str,
    status: i32,
    unlimited_quota: bool,
    used_quota: i64,
    quota: i64,
    created_at: &'static str,
}

#[component]
pub fn KeyCard(entry: KeyItem) -> impl IntoView {
    let enabled = entry.status == 1;
    let status_class = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };
    let unlimited = entry.unlimited_quota;
    let pct = if entry.quota > 0 {
        ((entry.used_quota as f64 / entry.quota as f64) * 100.0)
            .min(100.0)
            .max(0.0) as i32
    } else {
        0
    };
    let bar_tone = if pct >= 90 {
        "bg-red-500"
    } else if pct >= 70 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    };

    view! {
        div { class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{entry.name}" }
                    p { class: "min-w-0 truncate font-mono text-[11px] text-zinc-500", "{entry.key_preview}" }
                }
                span { class: format!("shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium {}", status_class),
                    {if enabled { "启用" } else { "停用" }}
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex items-center justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "已用额度" }
                    {if unlimited {
                        view! {
                            span { class: "whitespace-nowrap rounded-full border border-sky-500/30 bg-sky-500/20 px-2 py-0.5 text-[11px] font-medium text-sky-300", "无限" }
                        }.into_any()
                    } else {
                        view! {
                            span { class: "whitespace-nowrap font-medium text-zinc-200", {fmt_quota(entry.used_quota)} " / " {fmt_quota(entry.quota)} }
                        }.into_any()
                    }}
                }
                {if !unlimited {
                    view! {
                        div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                            div { class: format!("h-full rounded-full {}", bar_tone), style: format!("width: {}%", pct) }
                        }
                    }.into_any()
                } else {
                    ().into_any()
                }}
                {if !entry.created_at.is_empty() {
                    view! {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                            span { class: "whitespace-nowrap font-mono text-zinc-400", "{entry.created_at}" }
                        }
                    }.into_any()
                } else {
                    ().into_any()
                }}
            }

            div { class: "mt-4 flex items-center gap-2 border-t border-zinc-800 pt-3",
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Xs, class: "flex-1 text-zinc-400", "编辑" }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Xs, class: "flex-1 text-zinc-400", {if enabled { "停用" } else { "启用" }} }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Xs, class: "flex-1 text-red-400", "删除" }
            }
        }
    }
}
