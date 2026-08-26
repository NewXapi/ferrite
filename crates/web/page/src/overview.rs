use dioxus::prelude::*;

/// Overview canvas (总览): stat band + activity grid + breakdown bars.
/// All data is mocked; grayscale only.
#[component]
pub fn OverviewPanel() -> Element {
    rsx! {
        div { class: "space-y-8  p-6",
            StatBand {}
            ActivityGrid {}
            Breakdowns {}
        }
    }
}

/// Top metric band.
#[component]
fn StatBand() -> Element {
    let stats = [
        ("38.42B", "TOKEN 总量"),
        ("$31.86K", "总成本"),
        ("312", "活跃天数"),
        ("97", "当前连续天数"),
        ("3,101h", "活跃时间"),
        ("430.2M", "单日峰值"),
        ("gpt-5.6-sol", "最常用模型"),
        ("283.0K", "消息数"),
    ];

    rsx! {
        section { class: "grid grid-cols-2 gap-px overflow-hidden rounded-xl border border-zinc-800 bg-zinc-800 sm:grid-cols-4",
            for (value, label) in stats {
                div { class: "bg-zinc-900 px-4 py-5",
                    p { class: "truncate text-lg font-semibold text-zinc-100", "{value}" }
                    p { class: "mt-1 truncate text-xs text-zinc-500", "{label}" }
                }
            }
        }
    }
}

/// Year-long token activity heat grid (deterministic mock pattern).
#[component]
fn ActivityGrid() -> Element {
    const ROWS: usize = 8;
    const COLS: usize = 52;

    rsx! {
        section { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "Token 活动" }
            div { class: "space-y-1.5",
                for row in 0..ROWS {
                    div { class: "flex gap-1.5",
                        for col in 0..COLS {
                            div { class: "h-3 flex-1 cursor-default rounded-[3px] transition-shadow hover:ring-2 hover:ring-zinc-400 {grid_shade(row, col)}" }
                        }
                    }
                }
                div { class: "flex justify-between pt-2 text-xs text-zinc-600",
                    span { "7月" }
                    span { "9月" }
                    span { "11月" }
                    span { "1月" }
                    span { "3月" }
                    span { "5月" }
                    span { "7月" }
                }
            }
        }
    }
}

/// Deterministic shade so the grid looks varied without randomness.
fn grid_shade(row: usize, col: usize) -> &'static str {
    match (row * 7 + col * 13) % 5 {
        0 => "bg-zinc-700",
        1 => "bg-zinc-800",
        2 => "bg-zinc-600",
        3 => "bg-zinc-800/60",
        _ => "bg-zinc-500",
    }
}

/// Usage breakdown by tool and by model.
#[component]
fn Breakdowns() -> Element {
    let tools = [
        ("Codex", "24.96B", 65.0),
        ("Claude Code", "9.84B", 25.6),
        ("Hermes", "2.74B", 7.1),
        ("Cursor", "880.0M", 2.3),
    ];
    let models = [
        ("gpt-5.6-sol", "15.80B", 41.1),
        ("claude-fable-5", "10.92B", 28.4),
        ("kimi-k3", "6.84B", 17.8),
        ("glm-5.2", "4.86B", 12.7),
    ];

    rsx! {
        section { class: "grid gap-4 lg:grid-cols-2",
            BreakdownCard { title: "按工具", items: tools.to_vec() }
            BreakdownCard { title: "按模型", items: models.to_vec() }
        }
    }
}

#[component]
fn BreakdownCard(title: String, items: Vec<(&'static str, &'static str, f64)>) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "{title}" }
            div { class: "space-y-3",
                for (name, value, pct) in items {
                    div { class: "grid grid-cols-[1fr_auto_auto] items-center gap-3",
                        span { class: "truncate text-sm text-zinc-300", "{name}" }
                        span { class: "text-sm text-zinc-400", "{value}" }
                        span { class: "w-12 text-right text-sm text-zinc-500", "{pct}%" }
                        div { class: "col-span-3 h-1.5 overflow-hidden rounded-full bg-zinc-800",
                            div {
                                class: "h-full rounded-full bg-zinc-400",
                                style: "width: {pct}%",
                            }
                        }
                    }
                }
            }
        }
    }
}
