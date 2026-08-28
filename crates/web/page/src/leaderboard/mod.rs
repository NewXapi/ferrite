//! 排行榜面板: 数据层 + 卡牌组件 + 汇总图表.
//! 加新模型 = 在 `data::MODELS` 里加一行 (含立绘 asset), 其余 UI 全部由数值推导.

pub mod data;
mod cards;
mod charts;

pub use cards::{MiniRadarCard, PosterImageCard};
pub use charts::{BubbleCard, RankListCard, RidgeCard};
pub use data::{composite, ModelStat, MODELS};

use dioxus::prelude::*;

#[component]
pub fn LeaderboardPanel() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());

    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4",
                for (i, m) in ranked.iter().take(5).copied().enumerate() {
                    MiniRadarCard {
                        rank: i + 1,
                        lean: if i % 2 == 0 { -4.0 } else { 0.0 },
                        model: m,
                    }
                }
            }
            section { class: "grid grid-cols-2 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
                for (i, m) in ranked.iter().take(5).copied().enumerate() {
                    PosterImageCard { rank: i + 1, model: m }
                }
            }
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
                RankListCard {}
                RidgeCard {}
                BubbleCard {}
            }
        }
    }
}
