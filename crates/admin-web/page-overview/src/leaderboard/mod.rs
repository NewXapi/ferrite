//! Restored leaderboard page: composition + cards + charts + data.
//! 面板取数统一走 `crate::api`(它再指回本模块的 `data` 层)。
mod cards;
mod charts;
pub mod data;

use dioxus::prelude::*;

use crate::api::leaderboard::{MODELS, ModelStat, composite};
use cards::{MiniRadarCard, PosterImageCard};
use charts::{BubbleCard, RankListCard, RidgeCard};

#[component]
pub fn LeaderboardPanel() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());
    rsx! {
        div { class: "flex flex-col gap-6 p-4 md:gap-8 md:p-6",
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "text-lg font-bold tracking-tight text-zinc-100 md:text-xl", "模型实力排行榜" }
                    p { class: "mt-1 text-xs text-zinc-400", "正面展示立绘与雷达图，点击卡牌可 3D 翻转查看六维综合评测与详细指标" }
                }
                span { class: "rounded-full border border-zinc-800 bg-zinc-900 px-3 py-1 text-xs text-zinc-400",
                    "共收录 {ranked.len()} 款主流模型"
                }
            }
            // 海报翻牌卡大阵列
            section { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-5",
                for (i, m) in ranked.iter().copied().enumerate() {
                    PosterImageCard { rank: i + 1, model: m }
                }
            }
            // 底部排行榜分析组件 (图表 + 排行清单)
            section { class: "grid grid-cols-1 gap-4 md:grid-cols-3 xl:grid-cols-5 pt-4",
                RankListCard {}
                RidgeCard {}
                BubbleCard {}
            }
        }
    }
}
