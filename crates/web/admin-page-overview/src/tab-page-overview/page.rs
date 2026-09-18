//! 总览 tab 页面层:状态 + 拉取 effect + 组件组合(无渲染细节)。
//!
//! - 是什么:总览页根组件,四个区块(趋势 / 渠道健康 / 错误 / 统计 + 双榜)的组合者。
//! - 负责什么:持有全部取数 signal 与两个 `use_effect`,把后端行整形为视图结构。
//! - 交互逻辑:时间窗 `timeframe` 变化即重拉趋势 + 两个 Top 榜 + sparkline 源;
//!   汇总 / 健康 / 错误各自独立 effect 独立拉取,失败互不影响。
//! - 样式:根容器 `flex flex-col gap-3 p-4 md:gap-4 md:p-6`;双榜
//!   `grid-cols-1 md:grid-cols-2`。
//! - 由哪些小组件组成:`TrendPanel`、`ChannelHealth`、`ErrorsPanel`、
//!   `StatsSection`、两张 `TopListCard`。
//! - 数据流通:对外只读后端(`/api/dashboard`、`/api/log/trend`、`/api/log/top`);
//!   对内把 DTO 派生为 `StatCardView` / `TopRowFE` 交给子组件。

use dioxus::prelude::*;

use client::ApiClient;
use contract::api::usage::DashboardSummaryDto;

use super::shared::{
    LBL_CHANNELS, LBL_GROUPS, LBL_QUOTA_TODAY, LBL_REQUESTS_TODAY, LBL_TOKENS, LBL_USERS,
    QuotaView, StatCardView, TESTID_TOP_MODELS_TOTAL, TESTID_TOP_USERS_TOTAL, TOP_MODELS_TITLE,
    TOP_MODELS_UNIT, TOP_USERS_TITLE, TOP_USERS_UNIT,
};
use super::stats::StatsSection;
use super::top_lists::{TopListCard, TopRowFE};
use super::trend::TrendPanel;

use crate::api::{self, TrendBucketFE};
use crate::shared::fmt_raw;

/// 两张带 sparkline 的卡各自的 SVG 渐变 id(与标签一一对应,避免 url(#id) 串线)。
const SPARK_GRADIENT_REQUESTS: &str = "sparkline-requests";
const SPARK_GRADIENT_QUOTA: &str = "sparkline-quota";

/// Layout convention (共享给所有面板组件, 详见仓库 README.md):
///   页面网格  `grid-cols-1 md:grid-cols-3 lg:grid-cols-5`  —— 手机 1 栏 / 平板 3 栏 / Web 5 栏。
///   小卡片(统计卡)占 1 栏; 宽面板并排: 热力图类 `md:col-span-2 xl:col-span-3`,
///   列表/分布类 `md:col-span-1 xl:col-span-2`; 手机端一律堆叠, 定宽内容用横向滚动。
///
/// 数据全部来自真实后端：/api/dashboard + /api/log/trend + /api/log/top。
#[component]
pub fn OverviewPanel() -> Element {
    // ponytail: full UI overhaul to add breakdown cards for all timeframes in one go.
    let timeframe = use_signal(|| "今天"); // "今天", "本周", "本月", "今年"

    // 实时汇总:挂载时 use_effect 拉 GET /api/dashboard,写入 summary signal。
    let mut summary = use_signal(|| None::<DashboardSummaryDto>);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);

    // 趋势 + Top10 共用的窗口数据源：timeframe 变化即重拉。
    let mut top_users = use_signal(Vec::<TopRowFE>::new);
    let mut top_models = use_signal(Vec::<TopRowFE>::new);
    let mut top_users_total = use_signal(|| 0i64);
    let mut top_models_total = use_signal(|| 0i64);
    // 统计卡 sparkline（今日 24h → 12 点）：额度卡用 tokens 序列、请求卡用 calls 序列。
    let mut spark_tokens = use_signal(Vec::<f64>::new);
    let mut spark_calls = use_signal(Vec::<f64>::new);
    let mut buckets = use_signal(Vec::<TrendBucketFE>::new);
    let mut model_order = use_signal(Vec::<String>::new);
    let mut data_loading = use_signal(|| true);
    let mut data_err = use_signal(|| None::<String>);

    use_effect(move || {
        let tf = timeframe();
        data_loading.set(true);
        data_err.set(None);
        spawn(async move {
            let start = api::window_start(tf);
            let granularity = match tf {
                "今天" => "hour",
                "今年" => "month",
                _ => "day",
            };
            // sparkline 固定取「今天」24h 窗：与两张今日卡的口径一致，且与当前
            // 切换的 timeframe 解耦；start 只算一次、拉数与重切共用，避免跨小时漂移。
            let spark_start = api::window_start("今天");
            // 趋势 + 两个 Top 榜 + sparkline 源并行拉；主数据一个失败即报错（数据完整性优先）。
            let (trend_r, users_r, models_r, spark_r) = (
                api::trend_api(granularity, &start).await,
                api::top_usage_api("user", &start, 10).await,
                api::top_usage_api("model", &start, 10).await,
                api::trend_api("hour", &spark_start).await,
            );
            match (trend_r, users_r, models_r) {
                (Ok(rows), Ok(users), Ok(models)) => {
                    let (b, order) = api::pivot_trend(rows, tf);
                    buckets.set(b);
                    model_order.set(order);
                    // 用户榜按消耗折 $ 展示、模型榜按 tokens 展示；份额 = 行值占前 10
                    // 名合计；增长率 = 本窗 tokens 环比上一等长窗口 previous_tokens。
                    let u_tot: i64 = users.iter().map(|r| r.quota).sum();
                    top_users_total.set(u_tot);
                    top_users.set(
                        users
                            .iter()
                            .map(|r| TopRowFE {
                                name: r.name.clone(),
                                amount: api::fmt_usd(r.quota),
                                share: api::share_text(r.quota, u_tot),
                                growth: api::growth_of(r.previous_tokens, r.tokens),
                            })
                            .collect(),
                    );
                    let m_tot: i64 = models.iter().map(|r| r.tokens).sum();
                    top_models_total.set(m_tot);
                    top_models.set(
                        models
                            .iter()
                            .map(|r| TopRowFE {
                                name: r.name.clone(),
                                amount: fmt_raw(r.tokens),
                                share: api::share_text(r.tokens, m_tot),
                                growth: api::growth_of(r.previous_tokens, r.tokens),
                            })
                            .collect(),
                    );
                    // sparkline 是装饰性最佳努力：拉失败保持空 → 卡内渲染等高占位
                    // （诚实降级），不影响主数据展示。
                    if let Ok(srows) = spark_r {
                        let (tokens, calls) = api::hourly_sums(&srows, &spark_start);
                        spark_tokens.set(api::reslice_sum(&tokens, 12));
                        spark_calls.set(api::reslice_sum(&calls, 12));
                    }
                    data_loading.set(false);
                }
                (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => {
                    data_err.set(Some(e.to_string()));
                    data_loading.set(false);
                }
            }
        });
    });

    // 汇总拉取:进面板自动拉一次(use_effect 无信号依赖 → 仅挂载执行)。
    use_effect(move || {
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match api::get_dashboard_summary_api(&client).await {
                Ok(d) => {
                    summary.set(Some(d));
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    // 把实时 DTO 展开成 (值, 中文标签) 卡片列表(rsx! 之外计算,避免宏内 let)。
    // 拉取失败 → 中性占位(8090 预览反馈①):不渲染红色错误盒,改用全零 DTO
    // 照常渲染 7 张统计卡(额度卡 $0.00、runway 卡「无近期消耗」灰点)、顶部
    // asOf 位随 summary 为 None 自然隐藏;失败文案 / HTTP 状态码 / 重试按钮均
    // 不上 UI。重拉时机:本面板随 tab 卸载/重挂(use_effect 重新执行即重新拉取);
    // err 仅留在内存不渲染。
    let summary_err = err();
    let effective_summary = summary().or_else(|| {
        if summary_err.is_some() {
            Some(DashboardSummaryDto::default())
        } else {
            None
        }
    });
    let stats_opt: Option<Vec<(String, &'static str)>> =
        effective_summary.as_ref().map(dashboard_stats);
    // 统计卡四元组:值 / 标签 / sparkline 序列(仅今日两卡) / SVG 渐变 id。
    // 仅「今日请求 / 今日额度」两张卡带 12 点迷你面积线;
    // 空序列(拉数失败或窗口无数据)由 Sparkline 渲染等高占位。
    let stat_cards: Vec<StatCardView> = stats_opt
        .iter()
        .flatten()
        .map(|(value, label)| {
            let (spark, gid) = if *label == LBL_REQUESTS_TODAY {
                (Some(spark_calls()), SPARK_GRADIENT_REQUESTS)
            } else if *label == LBL_QUOTA_TODAY {
                (Some(spark_tokens()), SPARK_GRADIENT_QUOTA)
            } else {
                (None, "")
            };
            StatCardView {
                value: value.clone(),
                label,
                sparkline: spark,
                gradient_id: gid,
            }
        })
        .collect();
    // 第 7 张卡(额度余量)只在拿到 effective_summary 时渲染
    let quota: Option<QuotaView> = effective_summary.map(|d| QuotaView {
        remaining: d.quota_remaining,
        today: d.quota_today,
    });
    // 数据新鲜度: asOf 的本地时间直接亮在统计卡区头部;解析失败不显示(诚实降级)。
    let as_of_time = summary()
        .as_ref()
        .and_then(|d| api::as_of_local_time(&d.as_of));
    let trend_loading = data_loading();
    let trend_err = data_err();
    // 用计算后的 buckets 判空 (单一数据源): pivot 后如果每桶 total 都是 0,
    // 则以渲染为准 — 与 setter 里的行判零语义一致, 但不会错位。
    let empty_window =
        !trend_loading && trend_err.is_none() && buckets().iter().all(|b| b.total <= 0.0);

    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            // 用量趋势大面板(含时间窗切换) —— 真实 /api/log/trend 聚合
            TrendPanel { timeframe, buckets, model_order, loading: trend_loading, err: trend_err, empty_window }

            // 渠道健康度(真实 /api/monitor 探活聚合)
            super::health::ChannelHealth {}

            // 近 24 小时错误(真实 /api/log/errors 聚合)——独立信号独立拉取,不阻塞面板其它数据
            super::errors::ErrorsPanel {}

            // 实时汇总统计卡(数据来自真实后端 /api/dashboard)
            StatsSection {
                loading: loading(),
                cards: stat_cards,
                quota,
                as_of: as_of_time,
            }

            // Top 10 breakdowns —— 真实 /api/log/top 聚合
            // 面板卡挂 hoverable（维护者要求悬停边框变亮的动态全站回归）
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-2 lg:gap-4",
                // Top 10 Models(合计口径 tokens)
                TopListCard {
                    title: TOP_MODELS_TITLE,
                    total_text: fmt_raw(top_models_total()),
                    total_label: TOP_MODELS_UNIT,
                    total_testid: TESTID_TOP_MODELS_TOTAL,
                    rows: top_models(),
                }

                // Top 10 Users(合计口径 $)
                TopListCard {
                    title: TOP_USERS_TITLE,
                    total_text: api::fmt_usd(top_users_total()),
                    total_label: TOP_USERS_UNIT,
                    total_testid: TESTID_TOP_USERS_TOTAL,
                    rows: top_users(),
                }
            }

        }
    }
}

/// 把实时 `DashboardSummaryDto` 展开成 (值, 中文标签) 卡片列表,供总览统计区渲染。
///
/// 顺序与标签:`总用户=users`、`启用渠道=channels_enabled`、`令牌=tokens`、
/// `分组=groups`、`今日请求=requests_today`、`今日额度=quota_today`。
/// 今日额度按 `fmt_usd` 折算 $ 展示(500_000 = $1),与 Top10 用户榜同口径。
fn dashboard_stats(d: &DashboardSummaryDto) -> Vec<(String, &'static str)> {
    vec![
        (d.users.to_string(), LBL_USERS),
        (d.channels_enabled.to_string(), LBL_CHANNELS),
        (d.tokens.to_string(), LBL_TOKENS),
        (d.groups.to_string(), LBL_GROUPS),
        (d.requests_today.to_string(), LBL_REQUESTS_TODAY),
        (api::fmt_usd(d.quota_today), LBL_QUOTA_TODAY),
    ]
}
