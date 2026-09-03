use chrono::{Datelike, Duration, Local, NaiveDate};
use dioxus::prelude::*;

use crate::api;

// Layout convention (共享给所有面板组件, 详见仓库 README.md):
//   页面网格  `grid-cols-1 md:grid-cols-3 xl:grid-cols-5`  —— 手机 1 栏 / 平板 3 栏 / Web 5 栏。
//   小卡片(统计卡)占 1 栏; 宽面板并排: 热力图类 `md:col-span-2 xl:col-span-3`,
//   列表/分布类 `md:col-span-1 xl:col-span-2`; 手机端一律堆叠, 定宽内容用横向滚动。

/// Overview canvas (总览): responsive odd-column grid — 2 cols on mobile,
/// 3 on md, 5 on xl; wide sections span every column.
/// 数据经 `api` 取用;grayscale only.
#[component]
pub fn OverviewPanel() -> Element {
    let stats = api::overview::fetch_stats();

    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
                for &(value, label) in stats {
                    StatCard { value, label }
                }
            }
            // 面板区: 同一套栏数, 热力图按时间范围占 3/2/1 栏
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
            ActivityGrid {}
            BreakdownTabs {}
        }
        }
    }
}

/// Compact single-stat card occupying one grid column.
#[component]
fn StatCard(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900 px-4 py-3",
            p { class: "truncate text-base font-semibold text-zinc-100 md:text-lg", "{value}" }
            p { class: "mt-0.5 truncate text-xs text-zinc-500", "{label}" }
        }
    }
}

/// 热力图日内单元格:日模计算 (level, 显示 tokens/cost) + 是否落在选定范围
#[derive(Clone)]
struct DayCell {
    in_range: bool,
    level: u8,
    date: String,
    tokens: String,
    cost: String,
}

/// Token activity heatmap: GitHub-style calendar, responsive (no scrollbar).
/// Cells are square via aspect-square, week columns flex-1 to fill the panel.
/// Range tabs (全年 / 半年 / 近3月) sit centered under the month labels; the
/// panel's grid span shrinks with the range (xl: 3栏 → 2栏 → 1栏).
#[component]
fn ActivityGrid() -> Element {
    let mut range = use_signal(|| 0u8);

    let today = Local::now().date_naive();
    // 全年 364d / 半年 182d / 近3月 91d, aligned back to Sunday for full weeks.
    let days = match range() {
        0 => 364,
        1 => 182,
        _ => 91,
    };
    let first = today - Duration::days(days);
    let aligned = first - Duration::days(first.weekday().num_days_from_sunday() as i64);
    let n_weeks = ((today - aligned).num_days() / 7 + 1) as usize;

    // 5栏网格里的占位: 3个月是最小单位 (1栏), 半年 2栏, 全年 3栏。
    let span = match range() {
        0 => "md:col-span-2 xl:col-span-3",
        1 => "md:col-span-2 xl:col-span-2",
        _ => "xl:col-span-1",
    };

    let weeks: Vec<Vec<DayCell>> = (0..n_weeks)
        .map(|w| {
            (0..7)
                .map(|r| {
                    let day = aligned + Duration::days((w * 7 + r) as i64);
                    let in_range = day >= first && day <= today;
                    let (level, tokens, cost) = day_mock(day);
                    DayCell {
                        in_range,
                        level,
                        date: day.format("%b %-d, %Y").to_string(),
                        tokens,
                        cost,
                    }
                })
                .collect()
        })
        .collect();

    // Month labels: the week containing the 1st of a month carries its label;
    // positions are percentages so they follow the flex-stretched columns.
    let month_labels: Vec<(usize, String)> = weeks
        .iter()
        .enumerate()
        .filter_map(|(w, _col)| {
            let day = aligned + Duration::days((w * 7) as i64);
            (0..7)
                .map(|r| day + Duration::days(r as i64))
                .find(|d| d.day() == 1)
                .map(|d| (w, format!("{}\u{6708}", d.month())))
        })
        .collect();

    // One shared tooltip; fixed positioning is immune to any scroll offset.
    // ponytail: single tooltip + full re-render per hover, fine at mock scale.
    let mut tip = use_signal(|| None::<(f64, f64, String, String, String)>);

    let ranges = [
        (0u8, "\u{5168}\u{5E74}"),
        (1u8, "\u{534A}\u{5E74}"),
        (2u8, "\u{8FD1}3\u{4E2A}\u{6708}"),
    ];

    rsx! {
        section { class: "self-start {span} rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            h2 { class: "mb-4 text-sm font-medium text-zinc-300", "Token \u{6D3B}\u{52A8}" }
            div { class: "relative",
                div {
                    div { class: "flex", style: "gap: 3px",
                        for week in weeks {
                            div { class: "flex flex-1 flex-col", style: "gap: 3px",
                                for day in week {
                                    if day.in_range {
                                        {
                                            let date = day.date.clone();
                                            let tokens = day.tokens.clone();
                                            let cost = day.cost.clone();
                                            rsx! {
                                                div {
                                                    class: "aspect-square w-full cursor-default rounded-[2px] transition-shadow hover:ring-1 hover:ring-zinc-400 {heat_shade(day.level)}",
                                                    onmouseenter: move |evt| {
                                                        let p = evt.data.client_coordinates();
                                                        tip.set(Some((p.x, p.y, date.clone(), tokens.clone(), cost.clone())));
                                                    },
                                                    onmouseleave: move |_| tip.set(None),
                                                }
                                            }
                                        }
                                    } else {
                                        div { class: "aspect-square w-full" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "relative mt-2 h-4 text-xs text-zinc-600",
                        for (w, label) in month_labels {
                            span {
                                class: "absolute",
                                style: "left: {(w as f64 / n_weeks as f64) * 100.0:.2}%",
                                "{label}"
                            }
                        }
                    }
                }
                if let Some((x, y, date, tokens, cost)) = tip() {
                    div {
                        class: "pointer-events-none fixed z-50 whitespace-nowrap rounded-lg border border-zinc-700 bg-zinc-950/95 px-3 py-2 shadow-xl",
                        style: "left: {x}px; top: {y - 10.0}px; transform: translate(-50%, -100%)",
                        p { class: "text-xs font-semibold text-zinc-100", "{date}" }
                        p { class: "mt-1 flex justify-between gap-4 text-xs text-zinc-400",
                            span { "Tokens" }
                            span { class: "text-zinc-300", "{tokens}" }
                        }
                        p { class: "flex justify-between gap-4 text-xs text-zinc-400",
                            span { "Cost" }
                            span { class: "text-zinc-300", "{cost}" }
                        }
                    }
                }
            }
            // Range switcher: pill-style segmented dots (横向版), centered.
            div { class: "mt-4 flex justify-center",
                div { class: "flex items-center gap-2 rounded-full border border-zinc-800 bg-zinc-950 px-3 py-2",
                    for (id, name) in ranges {
                        button {
                            class: if range() == id {
                                "h-2 w-5 rounded-full bg-zinc-100 transition-all"
                            } else {
                                "h-2 w-2 rounded-full bg-zinc-700 transition-all hover:bg-zinc-500"
                            },
                            "aria-label": "{name}",
                            title: "{name}",
                            onclick: move |_| range.set(id),
                        }
                    }
                }
            }
        }
    }
}
/// Deterministic per-day mock: activity level 0..=4 plus display values.
fn day_mock(day: NaiveDate) -> (u8, String, String) {
    // Knuth multiplicative hash: same date always yields the same mock level.
    let d = day.num_days_from_ce() as u64;
    let mix = d.wrapping_mul(2654435761) >> 13;
    let level = (mix % 5) as u8;
    let tokens = level as f64 * 9.4 + (mix % 9) as f64 * 0.6;
    let cost = tokens * 0.83;
    (level, format!("{tokens:.1}M"), format!("${cost:.2}"))
}

fn heat_shade(level: u8) -> &'static str {
    match level {
        0 => "bg-zinc-800/60",
        1 => "bg-zinc-700",
        2 => "bg-zinc-600",
        3 => "bg-zinc-500",
        _ => "bg-zinc-400",
    }
}

#[derive(Clone, Copy, PartialEq)]
enum BreakdownTab {
    Users,
    Models,
}

/// Tool/model usage breakdown in one card, switched by a segmented tab control.
#[component]
fn BreakdownTabs() -> Element {
    let mut tab = use_signal(|| BreakdownTab::Users);
    let (title, items) = match tab() {
        BreakdownTab::Users => ("按用户", api::overview::fetch_users()),
        BreakdownTab::Models => ("按模型", api::overview::fetch_models()),
    };
    let tools_cls = tab_class(tab() == BreakdownTab::Users);
    let models_cls = tab_class(tab() == BreakdownTab::Models);

    rsx! {
        section { class: "self-start md:col-span-1 xl:col-span-2 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
            div { class: "mb-4 flex items-center justify-between gap-3",
                h2 { class: "text-sm font-medium text-zinc-300", "{title}" }
                div { class: "flex gap-0.5 rounded-lg border border-zinc-800 bg-zinc-950 p-0.5",
                    button {
                        class: "{tools_cls}",
                        onclick: move |_| tab.set(BreakdownTab::Users),
                        "用户"
                    }
                    button {
                        class: "{models_cls}",
                        onclick: move |_| tab.set(BreakdownTab::Models),
                        "模型"
                    }
                }
            }
            BreakdownList { items: items.to_vec() }
        }
    }
}

fn tab_class(active: bool) -> &'static str {
    if active {
        "rounded-md bg-zinc-800 px-3 py-1 text-xs text-zinc-100"
    } else {
        "rounded-md px-3 py-1 text-xs text-zinc-500 transition-colors hover:text-zinc-300"
    }
}

#[component]
fn BreakdownList(items: Vec<(&'static str, &'static str, f64)>) -> Element {
    rsx! {
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
