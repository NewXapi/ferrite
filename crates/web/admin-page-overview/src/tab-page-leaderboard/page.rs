//! 排行榜 tab 页面层:状态 + 拉取 effect + 区块组合(不含渲染细节)。
//!
//! - 是什么:排行榜页根组件,三个区块(演示实力榜 / 厂商份额 / 真实用量榜)的组合者。
//! - 负责什么:持有榜单与升降速两组取数 signal,把后端行交给子区块。
//! - 交互逻辑:进面板自动拉一次;时间窗切换与「重试」按钮驱动重拉(`reload`
//!   计数);刷新按钮已删(8090 预览反馈②)。本面板错误红盒保留重试。
//! - 样式:根容器 `flex flex-col gap-6 p-4 md:gap-8 md:p-6`;三张口径榜
//!   `grid-cols-1 xl:grid-cols-3 gap-6`,与下方升降速双卡间距 `mt-6`。
//! - 由哪些小组件组成:`DemoBoard`、`VendorShareCard`、`UsageToolbar`、
//!   三张 `RankCard`、`MoversCards`。
//! - 数据流通:对外只读 `GET /api/log/top?by=model`(当前窗 10 行 + 两窗各 20 行)。
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
use super::shared::{
    BTN_RETRY, RANK_CALLS_SUBTITLE, RANK_CALLS_TITLE, RANK_QUOTA_SUBTITLE, RANK_QUOTA_TITLE,
    RANK_TOKENS_SUBTITLE, RANK_TOKENS_TITLE, USAGE_ARIA_LABEL, USAGE_EMPTY, USAGE_EMPTY_HINT,
    USAGE_ERR, USAGE_LOADING,
};
use super::toolbar::UsageToolbar;
use crate::api::{UsageTopRow, top_usage_api, window_start};

/// 模型用量排行榜: 真实 /api/log/top(by=model) 聚合 + 演示实力榜区块。
/// 时间窗与总览页同口径(今天=24h / 本周=7d / 本月=30d / 今年=365d)。
#[component]
pub fn LeaderboardPanel() -> Element {
    let timeframe = use_signal(|| "本月");
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

    rsx! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            // 区块一: 模型实力榜(演示) — 恢复 #154 前的立绘卡牌阵列, 数值为演示数据
            DemoBoard {}

            // 区块一点五: 厂商份额(真实区上方,demo 之后、用量榜之前;复用 by=model 聚合)
            VendorShareCard { rows: data.clone(), loading: is_loading }

            // 区块二: 真实用量榜(真实 /api/log/top 聚合, #154 接线原样保留)。
            // 时间窗 tab 与标题同行(8090 预览反馈④);介绍语只留一行口径说明。
            UsageToolbar { timeframe, model_count: data.len() }

            section { "data-testid": "leaderboard-usage", role: "region", "aria-label": "{USAGE_ARIA_LABEL}",
                if let Some(e) = error {
                    div { class: "rounded-2xl border border-destructive bg-destructive px-4 py-6 text-center",
                        p { class: "text-sm {ui::C_DANGER}", "{USAGE_ERR}" }
                        p { class: "mt-1 text-xs {ui::C_DANGER}", "{e}" }
                        button {
                            class: "mt-3 rounded-xl border border-border px-3 py-1.5 {ui::TYPE_DESC} hover:bg-secondary",
                            "data-testid": "retry-leaderboard",
                            onclick: move |_| reload.set(reload() + 1),
                            "{BTN_RETRY}"
                        }
                    }
                } else if is_loading {
                    div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-16 text-center",
                        p { class: "{ui::C_MUTED}", "{USAGE_LOADING}" }
                    }
                } else if data.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-16 text-center",
                        p { class: "{ui::C_MUTED}", "{USAGE_EMPTY}" }
                        p { class: "mt-1 {ui::TYPE_DESC}", "{USAGE_EMPTY_HINT}" }
                    }
                } else {
                    // 三卡间距 gap-6(8090 预览反馈④),卡片 p-5 保持
                    div { class: "grid grid-cols-1 gap-6 xl:grid-cols-3 pt-2",
                        RankCard {
                            title: RANK_TOKENS_TITLE,
                            subtitle: RANK_TOKENS_SUBTITLE,
                            testid: "leaderboard-tokens",
                            metric: RankMetric::Tokens,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: RANK_CALLS_TITLE,
                            subtitle: RANK_CALLS_SUBTITLE,
                            testid: "leaderboard-calls",
                            metric: RankMetric::Calls,
                            rows: data.clone(),
                        }
                        RankCard {
                            title: RANK_QUOTA_TITLE,
                            subtitle: RANK_QUOTA_SUBTITLE,
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
