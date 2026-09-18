//! 模型实力榜(演示)区块:头牌翻牌卡 + 立绘海报卡阵列 + 汇总图表。
//!
//! 全部由 `data` 层演示数值推导(后端暂无六维端点),排序口径按六维综合分降序;
//! 待真实源就绪后替换。

use dioxus::prelude::*;

use super::cards::{MiniRadarCard, PosterImageCard};
use super::charts::{GroupQuotaCard, ModelDistributionCard, PerformanceLatencyCard};
use super::data::{MODELS, ModelStat, composite};

/// 模型实力榜(演示)区块: 头牌翻牌卡 + 立绘海报卡阵列 + 汇总图表, 全部由 data 层演示数值推导。
/// 排序口径与恢复前版本一致: 按六维综合分降序。
#[component]
pub fn DemoBoard() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());

    rsx! {
        section { "data-testid": "leaderboard-demo", role: "region", "aria-label": "模型实力榜",
            class: "flex flex-col gap-6 md:gap-8",
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "模型实力榜" }
                }
                span { class: "rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs text-zinc-400",
                    "共收录 {ranked.len()} 款主流模型"
                }
            }
            // 头牌翻牌卡: 综合分前五, 立绘交替斜角
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4",
                for (i, m) in ranked.iter().take(5).copied().enumerate() {
                    MiniRadarCard {
                        rank: i + 1,
                        lean: if i % 2 == 0 { -4.0 } else { 0.0 },
                        model: m,
                    }
                }
            }
            // 海报翻牌卡大阵列
            section { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-5",
                for (i, m) in ranked.iter().copied().enumerate() {
                    PosterImageCard { rank: i + 1, model: m }
                }
            }
            // 底部数据分析图表 (参考 new-api / sub2api / wildtoken)
            section { class: "grid grid-cols-1 gap-4 xl:grid-cols-3 pt-2",
                ModelDistributionCard {}
                PerformanceLatencyCard {}
                GroupQuotaCard {}
            }
        }
    }
}
