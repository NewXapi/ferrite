use chrono::{DateTime, Duration, Local, Utc};
use dioxus::prelude::*;
use ui::ScrollSpyNav;
use ui::SegmentedCapsule;

use crate::api;

// —— 跨组件共享文案 (UsageLogsPanel / LogCard / LogDetailModal 同用) ——
const STATUS_SUCCESS: &str = "成功";
const STATUS_FAIL: &str = "失败";

/// 面板内的日志视图模型:拥有所有权,可放进 signal 做详情弹窗。
#[derive(Clone, PartialEq)]
struct LogEntry {
    id: usize,
    model: String,
    status: bool, // true = success, false = failure
    timestamp: DateTime<Utc>,
    prompt_tokens: u32,
    completion_tokens: u32,
    cached_tokens: u32,
    first_token_ms: u32,
    duration_ms: u32,
    cost: f64,
    error: Option<String>,
}

#[component]
pub fn UsageLogsPanel() -> Element {
    // —— 业务枚举 (显示 / 内部双套:对外标签是「近 7 天」/「近 30 天」, 内部 key 是「7天」/「30天」做 cutoff 计算) ——
    //  STATUS_* 提升到模块级: LogCard / LogDetailModal 的状态徽标同用
    const FILTER_ALL: &str = "全部";
    const RANGE_TODAY: &str = "今天";
    const RANGE_7D: &str = "7天";
    const RANGE_30D: &str = "30天";
    const LABEL_TODAY: &str = "今天";
    const LABEL_7D: &str = "近 7 天";
    const LABEL_30D: &str = "近 30 天";
    const SEC_STATS: &str = "用量统计";
    const SEC_LOGS: &str = "请求日志";

    let mut selected_model = use_signal(|| FILTER_ALL.to_string());
    let mut selected_status = use_signal(|| FILTER_ALL.to_string());
    let mut selected_range = use_signal(|| RANGE_7D.to_string());
    let mut detail = use_signal(|| None::<LogEntry>);
    let mut visible_count = use_signal(|| 8usize);

    let stats = api::fetch_usage_stats();
    let models = api::fetch_log_models();

    // Build filtered logs with real filtering logic
    let filtered_logs = use_memo(move || {
        let now = Utc::now();
        let cutoff = match selected_range().as_str() {
            RANGE_TODAY => now - Duration::days(1),
            RANGE_7D => now - Duration::days(7),
            _ => now - Duration::days(30), // 30天
        };

        let status_filter = match selected_status().as_str() {
            STATUS_SUCCESS => Some(true),
            STATUS_FAIL => Some(false),
            _ => None,
        };

        let model_filter = if selected_model() == FILTER_ALL {
            None
        } else {
            Some(selected_model().clone())
        };

        let mut logs: Vec<LogEntry> = api::fetch_logs()
            .iter()
            .filter(|entry| {
                let log_time = DateTime::from_timestamp(entry.timestamp, 0).unwrap_or_default();
                if log_time < cutoff {
                    return false;
                }
                if let Some(want_status) = status_filter
                    && entry.success != want_status
                {
                    return false;
                }
                if let Some(want_model) = &model_filter
                    && entry.model != want_model
                {
                    return false;
                }
                true
            })
            .enumerate()
            .map(|(i, entry)| LogEntry {
                id: i,
                model: entry.model.to_string(),
                status: entry.success,
                timestamp: DateTime::from_timestamp(entry.timestamp, 0).unwrap_or_default(),
                prompt_tokens: entry.prompt_tokens,
                completion_tokens: entry.completion_tokens,
                cached_tokens: entry.cached_tokens,
                first_token_ms: entry.first_token_ms,
                duration_ms: entry.duration_ms,
                cost: entry.cost,
                error: entry.error.map(|s| s.to_string()),
            })
            .collect();

        logs.sort_by_key(|log| std::cmp::Reverse(log.timestamp));
        logs
    });

    let displayed_logs = filtered_logs
        .read()
        .iter()
        .take(*visible_count.read())
        .cloned()
        .collect::<Vec<_>>();
    let shown_len = displayed_logs.len();
    let total_len = filtered_logs.read().len();

    rsx! {
        div { class: "pl-8",
            ScrollSpyNav {
                container: "panel-scroll",
                items: vec![
                    ({SEC_STATS}.to_string(), "usage-sec-stats".to_string()),
                    ("筛选".to_string(), "usage-sec-filter".to_string()),
                    ({SEC_LOGS}.to_string(), "usage-sec-logs".to_string()),
                ],
            }

                div { class: "flex flex-col gap-6",
            // 统计卡 - 1/3/5 grid
            section { id: "usage-sec-stats", class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                for &(value, label) in stats {
                    StatCard { value, label }
                }

                }
            }

            // 过滤器
            section { id: "usage-sec-filter", class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                div { class: "flex items-center justify-between",
                    h2 { class: "text-sm font-medium text-zinc-300", "用量日志" }
                    button {
                        class: "text-xs px-4 py-2 rounded-xl border border-zinc-700 text-zinc-400 hover:bg-zinc-800 hover:text-zinc-200 transition-colors",
                        "导出 CSV"
                    }
                }

                // 模型/状态/时间:胶囊分段,手机每行最多 3 段、超出换行,悬停滚轮可切换
                div { class: "flex flex-col gap-3",
                    SegmentedCapsule {
                        items: models.iter().map(|m| m.to_string()).collect(),
                        active: models.iter().position(|m| *m == selected_model().as_str()).unwrap_or(0),
                        on_select: move |i: usize| selected_model.set(models[i].to_string()),
                    }
                    SegmentedCapsule {
                        items: vec![FILTER_ALL.to_string(), STATUS_SUCCESS.to_string(), STATUS_FAIL.to_string()],
                        active: [FILTER_ALL, STATUS_SUCCESS, STATUS_FAIL].iter().position(|s| s == &selected_status().as_str()).unwrap_or(0),
                        on_select: move |i: usize| selected_status.set([FILTER_ALL, STATUS_SUCCESS, STATUS_FAIL][i].to_string()),
                    }
                    SegmentedCapsule {
                        items: vec![LABEL_TODAY.to_string(), LABEL_7D.to_string(), LABEL_30D.to_string()],
                        active: [RANGE_TODAY, RANGE_7D, RANGE_30D].iter().position(|s| s == &selected_range().as_str()).unwrap_or(0),
                        on_select: move |i: usize| selected_range.set([RANGE_TODAY, RANGE_7D, RANGE_30D][i].to_string()),
                    }
                }
            }

            // 日志卡片网格(宽度约定:手机 1 栏 / 平板 3 栏 / Web 5 栏)
            section { id: "usage-sec-logs", class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LOGS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5",
                    for log in displayed_logs {
                        LogCard {
                            key: "{log.id}",
                            log: log.clone(),
                            on_open: move |entry| detail.set(Some(entry)),
                        }
                    }
                }
            }

            if shown_len < total_len {
                div { class: "flex justify-center pt-4",
                    button {
                        class: "px-8 py-3 rounded-2xl border border-zinc-700 bg-zinc-900 hover:bg-zinc-800 text-sm text-zinc-400 hover:text-zinc-200 transition-all active:scale-95",
                        onclick: move |_| {
                            let next = (*visible_count.read()).min(filtered_logs.read().len()) + 6;
                            visible_count.set(next);
                        },
                        "加载更多"
                    }
                }
            } else if shown_len > 0 {
                div { class: "text-center text-xs text-zinc-500 py-6",
                    "已显示全部日志"
                }
            } else {
                div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                    p { class: "text-zinc-400", "没有找到匹配的日志记录" }
                }
            }

            // 点击日志卡 → 详情弹窗
            if let Some(entry) = detail() {
                LogDetailModal {
                    log: entry,
                    on_close: move |_| detail.set(None),
                }
            }
        }
        }
    }
}

#[component]
fn StatCard(value: &'static str, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-5 py-4 hover:border-zinc-600 transition-colors",
            p { class: "text-2xl font-semibold text-zinc-100 tabular-nums", "{value}" }
            p { class: "mt-1 text-xs text-zinc-500", "{label}" }
        }
    }
}

/// 千分位格式化
fn fmt_num(n: u32) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn fmt_sec(ms: u32) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

/// 日志卡片:放在 5/3/1 网格里的紧凑摘要卡,点击打开详情弹窗。
/// 头部 模型+状态+费用,下方 Tokens/首字·耗时 两行;失败卡片红底。
#[component]
fn LogCard(log: LogEntry, on_open: EventHandler<LogEntry>) -> Element {
    let time_str = log
        .timestamp
        .with_timezone(&Local)
        .format("%m-%d %H:%M")
        .to_string();
    let model_color = match log.model.as_str() {
        "gpt-4o" | "gpt-4o-mini" => "bg-emerald-400",
        "claude-3.5-sonnet" | "claude-3-haiku" => "bg-purple-400",
        "deepseek-r1" => "bg-blue-400",
        "qwen2.5-72b" => "bg-orange-400",
        _ => "bg-zinc-400",
    };
    let card_class = if log.status {
        "border-zinc-800 bg-zinc-900/60 hover:border-zinc-500 hover:bg-zinc-900"
    } else {
        "border-red-900/50 bg-red-950/20 hover:border-red-800/70"
    };
    let badge_class = if log.status {
        "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-red-500/15 text-red-400 border-red-500/30"
    };
    let cost_str = if log.status {
        format!("${:.4}", log.cost)
    } else {
        "—".to_string()
    };
    let timing_str = if log.status {
        format!(
            "{} / {}",
            fmt_sec(log.first_token_ms),
            fmt_sec(log.duration_ms)
        )
    } else {
        "不适用".to_string()
    };
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(log.prompt_tokens),
        fmt_num(log.completion_tokens)
    );

    rsx! {
        button {
            class: "w-full cursor-pointer rounded-2xl border p-4 text-left transition-colors {card_class}",
            onclick: move |_| on_open.call(log.clone()),

            // 头部:模型 + 状态 + 费用
            div { class: "flex items-center gap-2",
                span { class: "h-2.5 w-2.5 shrink-0 rounded-full {model_color}" }
                span { class: "truncate font-mono text-sm text-zinc-200", "{log.model}" }
                span { class: "shrink-0 rounded-full border px-2 py-0.5 text-xs {badge_class}",
                    if log.status { {STATUS_SUCCESS} } else { {STATUS_FAIL} }
                }
            }
            div { class: "mt-2 flex items-baseline justify-between gap-2",
                span { class: "font-mono text-xs text-zinc-500", "{time_str}" }
                span { class: "shrink-0 font-medium tabular-nums text-sm text-emerald-400", "{cost_str}" }
            }

            // 摘要两行:Tokens、首字/耗时
            div { class: "mt-3 space-y-1.5 text-xs",
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "Tokens" }
                    span { class: "whitespace-nowrap font-medium tabular-nums text-zinc-200",
                        "{tokens_pair}"
                    }
                }
                div { class: "flex justify-between gap-2",
                    span { class: "shrink-0 text-zinc-500", "首字 / 耗时" }
                    span { class: "whitespace-nowrap tabular-nums text-zinc-400", "{timing_str}" }
                }
            }
        }
    }
}

/// 日志详情弹窗:居中模态,手机近全宽;点遮罩或 × 关闭。
#[component]
fn LogDetailModal(log: LogEntry, on_close: EventHandler<()>) -> Element {
    let time_str = log
        .timestamp
        .with_timezone(&Local)
        .format("%Y-%m-%d %H:%M:%S")
        .to_string();
    let badge_class = if log.status {
        "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-red-500/15 text-red-400 border-red-500/30"
    };
    let secs = log.duration_ms as f64 / 1000.0;
    let tps_str = if log.status && log.duration_ms > 0 {
        format!("{:.0} t/s", log.completion_tokens as f64 / secs)
    } else {
        "—".to_string()
    };
    let cost_str = if log.status {
        format!("${:.4}", log.cost)
    } else {
        "—".to_string()
    };
    let timing_str = if log.status {
        format!(
            "{} / {}",
            fmt_sec(log.first_token_ms),
            fmt_sec(log.duration_ms)
        )
    } else {
        "不适用".to_string()
    };
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(log.prompt_tokens),
        fmt_num(log.completion_tokens)
    );
    let cached_str = fmt_num(log.cached_tokens);

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    div { class: "flex items-center gap-2",
                        h3 { class: "text-base font-semibold text-zinc-100", "日志详情" }
                        span { class: "rounded-full border px-2 py-0.5 text-xs {badge_class}",
                            if log.status { {STATUS_SUCCESS} } else { {STATUS_FAIL} }
                        }
                    }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        svg {
                            class: "h-5 w-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }

                div { class: "space-y-2.5 text-sm",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "模型" }
                        span { class: "min-w-0 break-all text-right font-mono text-zinc-200", "{log.model}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "时间" }
                        span { class: "whitespace-nowrap font-mono text-zinc-200", "{time_str}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "Tokens(提示/补全)" }
                        span { class: "whitespace-nowrap tabular-nums text-zinc-200", "{tokens_pair}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "缓存命中" }
                        span { class: "whitespace-nowrap tabular-nums text-zinc-200", "{cached_str}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "首字 / 耗时" }
                        span { class: "whitespace-nowrap tabular-nums text-zinc-200", "{timing_str}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "速度" }
                        span { class: "whitespace-nowrap tabular-nums text-zinc-200", "{tps_str}" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-500", "费用" }
                        span { class: "whitespace-nowrap font-medium tabular-nums text-emerald-400", "{cost_str}" }
                    }
                }

                if let Some(err) = &log.error {
                    div { class: "mt-4 rounded-xl bg-red-950/40 p-3 font-mono text-xs text-red-400",
                        "{err}"
                    }
                }
            }
        }
    }
}
