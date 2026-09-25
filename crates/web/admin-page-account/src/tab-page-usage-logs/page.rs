//! 用量·日志面板 — 全部真实数据:
//! - 统计卡: GET /api/log/self/stat (今日 requests/quota + 近 60s rpm/tpm)
//! - 日志列表: GET /api/log/self (服务端分页 page++, modelName 过滤, RFC3339 时间窗)
//!
//! 后端 `usage_logs` 无 success/error 列, 状态/失败率无数据源 → 不渲染相关 UI;
//! quota 为内部额度单位 (500_000 ≈ $1), 展示按此换算并标注「估」。

use dioxus::prelude::*;
use ui::SegmentedCapsule;
use ui::StatCard;
use ui::StatSize;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use contract::api::usage::{UsageLogDto, UsageStatDto};

use crate::api;
use crate::tab_page_usage_logs::{LogCard, LogDetailModal};
use crate::usage_support::{RANGE_7D, RANGE_30D, RANGE_TODAY, range_bounds};

const FILTER_ALL: &str = "全部";
const LABEL_TODAY: &str = "今天";
const LABEL_7D: &str = "近 7 天";
const LABEL_30D: &str = "近 30 天";
const SEC_STATS: &str = "用量统计";
const SEC_LOGS: &str = "请求日志";
/// 每页条数 (后端 clamp 1..=100)。
const PAGE_SIZE: i64 = 20;

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
                    // 本页统计卡统一用 Lg 档（px-5 py-4 + text-2xl 等宽值），与迁移前标记逐字一致。
                    StatCard { value: stat_val(|v| v.requests), label: "今日请求", size: StatSize::Lg }
                    StatCard { value: stat_val(|v| v.quota), label: "今日消耗 (额度单位)", size: StatSize::Lg }
                    StatCard { value: stat_val(|v| v.rpm), label: "RPM (近 60s)", size: StatSize::Lg }
                    StatCard { value: stat_val(|v| v.tpm), label: "TPM (近 60s)", size: StatSize::Lg }
                    StatCard { value: "—".to_string(), label: "成功率 (暂无数据)", size: StatSize::Lg }
                }
            }

            // 过滤器
            section { id: "usage-sec-filter", class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5 transition-colors hover:border-zinc-600",
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
