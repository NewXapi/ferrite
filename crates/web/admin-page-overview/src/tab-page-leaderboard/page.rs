//! 排行榜 tab 页面层:状态 + 拉取 effect + 区块组合(不含渲染细节)。
//!
//! - 模型实力榜(演示):[`super::demo_board`] 组合 [`super::cards`] / [`super::charts`],
//!   数值来自 [`super::data`] 演示层,待真实源就绪后替换。
//! - 真实用量榜:数据来自真实 `GET /api/log/top?by=model`(按模型聚合的消费日志),
//!   三张口径榜见 [`super::rank_board`],升降速双卡与厂商份额卡见 [`super::insights`]。

use dioxus::prelude::*;

use super::demo_board::DemoBoard;
use super::insights::{
    MoversCards, MoversState, VendorShareCard, previous_window_start, top_usage_between,
};
use super::rank_board::{RankCard, RankMetric};
use crate::api::{UsageTopRow, top_usage_api, window_start};

/// 模型用量排行榜: 真实 /api/log/top(by=model) 聚合 + 演示实力榜区块。
/// 时间窗与总览页同口径(今天=24h / 本周=7d / 本月=30d / 今年=365d)。
#[component]
pub fn LeaderboardPanel() -> Element {
    let mut timeframe = use_signal(|| "本月");
    let mut rows = use_signal(Vec::<UsageTopRow>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 本面板错误红盒保留「重试」(不在 8090 反馈①的三面板范围内),reload 仅由
    // 重试按钮驱动;刷新按钮已删(反馈②),数据进面板自动拉一次。
    let mut reload = use_signal(|| 0u32);
    let mut movers = use_signal(|| MoversState::Loading);

    // 进面板自动拉一次;时间窗切换 / 重试仍驱动重拉。
    use_effect(move || {
        let tf = timeframe();
        let _ = reload();
        loading.set(true);
        err.set(None);
        movers.set(MoversState::Loading);
        spawn(async move {
            let start = window_start(tf);
            match top_usage_api("model", &start, 10).await {
                Ok(items) => {
                    rows.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
        // 上升/下跌最快:当前窗与上一等长窗各一次 by=model top20 聚合,
        // 名次变动在前端算(口径 tokens,与增长环比一致)。与主榜单独立
        // 拉取,失败只降级本区,不拖垮三张榜单
        spawn(async move {
            let cur_start = window_start(tf);
            let prev_start = previous_window_start(tf);
            let cur = top_usage_api("model", &cur_start, 20).await;
            let prev = top_usage_between("model", &prev_start, &cur_start, 20).await;
            movers.set(match (cur, prev) {
                (Ok(c), Ok(p)) => MoversState::Ready { cur: c, prev: p },
                (Err(e), _) | (_, Err(e)) => MoversState::Failed(e.to_string()),
            });
        });
    });

    let data = rows();
    let is_loading = loading();
    let error = err();
    let tf = timeframe();

    rsx! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            // 区块一: 模型实力榜(演示) — 恢复 #154 前的立绘卡牌阵列, 数值为演示数据
            DemoBoard {}

            // 区块一点五: 厂商份额(真实区上方,demo 之后、用量榜之前;复用 by=model 聚合)
            VendorShareCard { rows: data.clone(), loading: is_loading }

            // 区块二: 真实用量榜(真实 /api/log/top 聚合, #154 接线原样保留)。
            // 时间窗 tab 与标题同行(8090 预览反馈④);介绍语只留一行口径说明。
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "真实用量榜" }
                    p { class: "mt-1 text-xs text-zinc-400", "后端消费日志聚合 · 窗口内 {data.len()} 个模型有调用" }
                }
                div { class: "flex items-center gap-1.5 rounded-lg border border-zinc-800 bg-zinc-950 p-1",
                    for t in ["今天", "本周", "本月", "今年"] {
                        button {
                            key: "{t}",
                            "data-testid": "leaderboard-timeframe-{t}",
                            class: "rounded-md px-2.5 py-1 text-xs font-medium transition-colors",
                            class: if tf == t { "bg-zinc-800 text-zinc-100 shadow-sm" } else { "text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| timeframe.set(t),
                            "{t}"
                        }
                    }
                }
            }

            section { "data-testid": "leaderboard-usage", role: "region", "aria-label": "模型用量排行",
                if let Some(e) = error {
                    div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                        p { class: "text-sm text-red-300", "加载排行数据失败" }
                        p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                            "data-testid": "retry-leaderboard",
                            onclick: move |_| reload.set(reload() + 1),
                            "重试"
                        }
                    }
                } else if is_loading {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "正在加载排行数据…" }
                    }
                } else if data.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                        p { class: "text-zinc-400", "该时间窗内暂无调用数据" }
                        p { class: "mt-1 text-xs text-zinc-600", "发起一次 /v1 调用后这里会展示真实用量排行" }
                    }
                } else {
                    // 三卡间距 gap-6(8090 预览反馈④),卡片 p-5 保持
                    div { class: "grid grid-cols-1 gap-6 xl:grid-cols-3 pt-2",
                        RankCard {
                            title: "Token 消耗 Top",
                            subtitle: "窗口内 prompt + completion tokens 合计",
                            testid: "leaderboard-tokens",
                            metric: RankMetric::Tokens,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: "调用次数 Top",
                            subtitle: "窗口内消费请求数",
                            testid: "leaderboard-calls",
                            metric: RankMetric::Calls,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: "费用消耗 Top",
                            subtitle: "窗口内计费额度(500000 = $1)",
                            testid: "leaderboard-quota",
                            metric: RankMetric::Quota,
                            rows: data,
                        }
                    }
                    // 前3卡与升降速双卡间距加大(8090 预览反馈①:三面板跟下面两面板没间距);
                    // 跟随当前 timeframe,数据进面板自动拉取
                    div { class: "mt-6",
                        MoversCards { state: movers() }
                    }
                }
            }
        }
    }
}
