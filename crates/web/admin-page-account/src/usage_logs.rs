//! 用量·日志面板 — 全部真实数据:
//! - 统计卡: GET /api/log/self/stat (今日 requests/quota + 近 60s rpm/tpm)
//! - 日志列表: GET /api/log/self (服务端分页 page++, modelName 过滤, RFC3339 时间窗)
//!
//! 后端 `usage_logs` 无 success/error 列, 状态/失败率无数据源 → 不渲染相关 UI;
//! quota 为内部额度单位 (500_000 ≈ $1), 展示按此换算并标注「估」。

use chrono::{DateTime, Duration, Local, SecondsFormat, Utc};
use dioxus::prelude::*;
use ui::SegmentedCapsule;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use contract::api::usage::{UsageLogDto, UsageStatDto};

use crate::api;

const FILTER_ALL: &str = "全部";
const RANGE_TODAY: &str = "今天";
const RANGE_7D: &str = "7天";
const RANGE_30D: &str = "30天";
const LABEL_TODAY: &str = "今天";
const LABEL_7D: &str = "近 7 天";
const LABEL_30D: &str = "近 30 天";
const SEC_STATS: &str = "用量统计";
const SEC_LOGS: &str = "请求日志";
/// 每页条数 (后端 clamp 1..=100)。
const PAGE_SIZE: i64 = 20;
/// 内部额度单位 → 美元换算基数 (后端口径)。
const QUOTA_PER_USD: f64 = 500_000.0;

/// 时间范围标签 → (start, end) RFC3339 (UTC, Z 结尾, 无 `+` 避免 URL 转义)。
fn range_bounds(label: &str) -> (String, String) {
    let end = Utc::now();
    let days = match label {
        RANGE_TODAY => 1,
        RANGE_7D => 7,
        _ => 30,
    };
    let start = end - Duration::days(days);
    (
        start.to_rfc3339_opts(SecondsFormat::Secs, true),
        end.to_rfc3339_opts(SecondsFormat::Secs, true),
    )
}

/// RFC3339 → 本地 "MM-dd HH:mm" 展示; 解析失败原样返回。
fn fmt_time(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| t.with_timezone(&Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// RFC3339 → 本地 "YYYY-MM-dd HH:mm:ss" (详情弹窗用)。
fn fmt_time_full(rfc3339: &str) -> String {
    DateTime::parse_from_rfc3339(rfc3339)
        .map(|t| {
            t.with_timezone(&Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_else(|_| rfc3339.to_string())
}

/// 千分位格式化。
fn fmt_num(n: i64) -> String {
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

/// 内部额度单位 → 估算美元展示 (500_000 ≈ $1)。
fn fmt_quota(quota: i64) -> String {
    format!("${:.4}", quota as f64 / QUOTA_PER_USD)
}

/// 拉取一页日志并写回状态。`append=true` 时追加 (加载更多), 否则重置列表并合并
/// 模型筛选项。page 为已加载页码, 成功后推进到请求页。
#[allow(clippy::too_many_arguments)]
fn load_logs(
    mut logs: Signal<Vec<UsageLogDto>>,
    mut total: Signal<i64>,
    mut page: Signal<i64>,
    mut models: Signal<Vec<String>>,
    mut loaded: Signal<bool>,
    mut load_err: Signal<String>,
    model: Option<String>,
    start: String,
    end: String,
    append: bool,
) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        let next_page = if append { page() + 1 } else { 1 };
        match api::list_self_logs_api(
            &client,
            model.as_deref(),
            Some(&start),
            Some(&end),
            Some(next_page),
            Some(PAGE_SIZE),
        )
        .await
        {
            Ok(pg) => {
                if append {
                    let mut cur = logs();
                    cur.extend(pg.items);
                    logs.set(cur);
                } else {
                    logs.set(pg.items);
                }
                total.set(pg.total);
                page.set(next_page);
                // 模型筛选项 = "全部" + 历史窗口内见过的模型去重 (后端无 self
                // distinct-models 端点)。只在「不带模型筛选」的加载时合并 —
                // 筛选某模型后的重拉若重建清单, 选项会收窄成 [全部, 该模型]。
                if !append && model.is_none() {
                    let mut seen = models();
                    for l in logs() {
                        if !seen.contains(&l.model_name) {
                            seen.push(l.model_name);
                        }
                    }
                    models.set(seen);
                }
                load_err.set(String::new());
                loaded.set(true);
            }
            Err(e) => load_err.set(e.to_string()),
        }
    });
}

#[component]
pub fn UsageLogsPanel() -> Element {
    let mut selected_model = use_signal(|| FILTER_ALL.to_string());
    let mut selected_range = use_signal(|| RANGE_7D.to_string());
    let mut detail = use_signal(|| None::<UsageLogDto>);

    let logs = use_signal(Vec::<UsageLogDto>::new);
    let total = use_signal(|| 0i64);
    let page = use_signal(|| 0i64);
    let models = use_signal(|| vec![FILTER_ALL.to_string()]);
    let loaded = use_signal(|| false);
    let load_err = use_signal(String::new);

    let stat = use_signal(|| None::<UsageStatDto>);
    let stat_loaded = use_signal(|| false);
    let stat_err = use_signal(String::new);

    // 首载: 今日统计 + 默认近 7 天日志第一页
    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        let mut s = stat;
        let mut sl = stat_loaded;
        let mut se = stat_err;
        spawn(async move {
            match api::get_self_stat_api(&client).await {
                Ok(v) => {
                    s.set(Some(v));
                    sl.set(true);
                }
                Err(e) => {
                    se.set(e.to_string());
                    sl.set(true);
                }
            }
        });
        let (start, end) = range_bounds(&selected_range());
        load_logs(
            logs, total, page, models, loaded, load_err, None, start, end, false,
        );
    });

    // 模型/时间切换 → 服务端重拉第一页
    let on_model_select = move |i: usize| {
        let m = models()[i].clone();
        selected_model.set(m.clone());
        let (start, end) = range_bounds(&selected_range());
        load_logs(
            logs,
            total,
            page,
            models,
            loaded,
            load_err,
            if m == FILTER_ALL { None } else { Some(m) },
            start,
            end,
            false,
        );
    };
    let on_range_select = move |i: usize| {
        let r = [RANGE_TODAY, RANGE_7D, RANGE_30D][i].to_string();
        selected_range.set(r.clone());
        let (start, end) = range_bounds(&r);
        let m = selected_model();
        load_logs(
            logs,
            total,
            page,
            models,
            loaded,
            load_err,
            if m == FILTER_ALL { None } else { Some(m) },
            start,
            end,
            false,
        );
    };
    let on_load_more = move |_| {
        let (start, end) = range_bounds(&selected_range());
        let m = selected_model();
        load_logs(
            logs,
            total,
            page,
            models,
            loaded,
            load_err,
            if m == FILTER_ALL { None } else { Some(m) },
            start,
            end,
            true,
        );
    };

    let s = stat();
    let stat_val = |pick: fn(&UsageStatDto) -> i64| -> String {
        match (&s, stat_loaded()) {
            (Some(v), _) => pick(v).to_string(),
            (None, true) => "—".into(),
            (None, false) => "…".into(),
        }
    };

    let shown_len = logs().len();
    let total_len = total() as usize;

    rsx! {
        div { class: "flex flex-col gap-6",
            // 统计卡 - 1/3/5 grid (今日口径)
            section { id: "usage-sec-stats", class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    StatCard { value: stat_val(|v| v.requests), label: "今日请求" }
                    StatCard { value: stat_val(|v| v.quota), label: "今日消耗 (额度单位)" }
                    StatCard { value: stat_val(|v| v.rpm), label: "RPM (近 60s)" }
                    StatCard { value: stat_val(|v| v.tpm), label: "TPM (近 60s)" }
                    StatCard { value: "—".to_string(), label: "成功率 (暂无数据)" }
                }
            }

            // 过滤器
            section { id: "usage-sec-filter", class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                div { class: "flex items-center justify-between",
                    h2 { class: "text-sm font-medium text-zinc-300", "{SEC_LOGS}" }
                    span { class: "text-xs text-zinc-500", "共 {total_len} 条" }
                }

                // 模型/时间: 胶囊分段 (状态筛选无数据源, 不渲染)
                div { class: "flex flex-col gap-3",
                    SegmentedCapsule {
                        items: models(),
                        active: models().iter().position(|m| *m == selected_model()).unwrap_or(0),
                        on_select: on_model_select,
                    }
                    SegmentedCapsule {
                        items: vec![LABEL_TODAY.to_string(), LABEL_7D.to_string(), LABEL_30D.to_string()],
                        active: [RANGE_TODAY, RANGE_7D, RANGE_30D].iter().position(|r| *r == selected_range()).unwrap_or(0),
                        on_select: on_range_select,
                    }
                }
            }

            // 日志卡片网格(宽度约定:手机 1 栏 / 平板 3 栏 / Web 5 栏)
            section { id: "usage-sec-logs", class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LOGS}" }
                if !load_err().is_empty() {
                    p { class: "text-sm text-amber-400", "无法加载日志 (未登录或请求失败): {load_err()}" }
                } else if !loaded() {
                    p { class: "text-sm text-zinc-500", "加载中…" }
                } else if shown_len == 0 {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "没有找到匹配的日志记录" }
                    }
                } else {
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for log in logs() {
                            LogCard {
                                key: "{log.id}",
                                log: log.clone(),
                                on_open: move |entry| detail.set(Some(entry)),
                            }
                        }
                    }
                }
            }

            if !load_err().is_empty() {
                // 错误态不再提供「加载更多」
            } else if shown_len < total_len {
                div { class: "flex justify-center pt-4",
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Lg,
                        onclick: on_load_more,
                        "加载更多"
                    }
                }
            } else if shown_len > 0 {
                div { class: "text-center text-xs text-zinc-500 py-6",
                    "已显示全部日志"
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

#[component]
fn StatCard(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-5 py-4 hover:border-zinc-600 transition-colors",
            p { class: "text-2xl font-semibold text-zinc-100 tabular-nums", "{value}" }
            p { class: "mt-1 text-xs text-zinc-500", "{label}" }
        }
    }
}

/// 日志卡片:模型色点 + 时间 + Tokens/耗时/消耗摘要,点击打开详情弹窗。
#[component]
fn LogCard(log: UsageLogDto, on_open: EventHandler<UsageLogDto>) -> Element {
    let time_str = fmt_time(&log.created_at);
    let model_color = match log.model_name.as_str() {
        "gpt-4o" | "gpt-4o-mini" => "bg-emerald-400",
        "claude-3.5-sonnet" | "claude-3-haiku" => "bg-purple-400",
        "deepseek-r1" => "bg-blue-400",
        "qwen2.5-72b" => "bg-orange-400",
        _ => "bg-zinc-400",
    };
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(log.prompt_tokens as i64),
        fmt_num(log.completion_tokens as i64)
    );
    let timing_str = if log.use_time_ms > 0 {
        format!("{:.1}s", log.use_time_ms as f64 / 1000.0)
    } else {
        "—".to_string()
    };
    let cost_str = fmt_quota(log.quota);

    rsx! {
        button {
            class: "w-full cursor-pointer rounded-2xl border border-zinc-800 bg-zinc-900/60 p-4 text-left transition-colors hover:border-zinc-500 hover:bg-zinc-900",
            onclick: move |_| on_open.call(log.clone()),

            // 头部:模型 + 消耗
            div { class: "flex items-center gap-2",
                span { class: "h-2.5 w-2.5 shrink-0 rounded-full {model_color}" }
                span { class: "truncate font-mono text-sm text-zinc-200", "{log.model_name}" }
            }
            div { class: "mt-2 flex items-baseline justify-between gap-2",
                span { class: "font-mono text-xs text-zinc-500", "{time_str}" }
                span { class: "shrink-0 font-medium tabular-nums text-sm text-emerald-400", "{cost_str}" }
            }

            // 摘要两行:Tokens、耗时
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

/// 日志详情弹窗:居中模态,手机近全宽;点遮罩或 × 关闭。
#[component]
fn LogDetailModal(log: UsageLogDto, on_close: EventHandler<()>) -> Element {
    let time_str = fmt_time_full(&log.created_at);
    let tokens_pair = format!(
        "{} / {}",
        fmt_num(log.prompt_tokens as i64),
        fmt_num(log.completion_tokens as i64)
    );
    let timing_str = if log.use_time_ms > 0 {
        format!("{:.1}s", log.use_time_ms as f64 / 1000.0)
    } else {
        "—".to_string()
    };
    let tps_str = if log.use_time_ms > 0 && log.completion_tokens > 0 {
        format!(
            "{:.0} t/s",
            log.completion_tokens as f64 / (log.use_time_ms as f64 / 1000.0)
        )
    } else {
        "—".to_string()
    };
    let cost_str = fmt_quota(log.quota);

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "日志详情" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-2.5 text-sm",
                    DetailRow { label: "模型", value: log.model_name.clone() }
                    DetailRow { label: "时间", value: time_str }
                    DetailRow { label: "密钥", value: log.token_name.clone() }
                    DetailRow { label: "渠道", value: log.channel_name.clone() }
                    DetailRow { label: "Tokens(提示/补全)", value: tokens_pair }
                    DetailRow { label: "耗时", value: timing_str }
                    DetailRow { label: "速度", value: tps_str }
                    DetailRow { label: "消耗(估)", value: cost_str }
                    DetailRow { label: "流式", value: if log.is_stream { "是".to_string() } else { "否".to_string() } }
                    DetailRow { label: "IP", value: log.ip.clone() }
                    DetailRow { label: "请求 ID", value: log.request_id.clone() }
                }
            }
        }
    }
}

#[component]
fn DetailRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between gap-2",
            span { class: "shrink-0 text-zinc-500", "{label}" }
            span { class: "min-w-0 break-all text-right font-mono text-zinc-200", "{value}" }
        }
    }
}
