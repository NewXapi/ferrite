//! Restored leaderboard page: composition + cards + charts + data.
//! 面板取数统一走 `crate::api`(它再指回本模块的 `data` 层)。
mod cards;
mod charts;
pub mod data;

use dioxus::prelude::*;

use crate::api::{MODELS, ModelStat, composite};
use cards::{MiniRadarCard, PosterImageCard};
use charts::{BubbleCard, RankListCard, RidgeCard};

#[component]
pub fn LeaderboardPanel() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());
    rsx! {
        div { class: "flex flex-col gap-3 p-4 md:gap-4 md:p-6",
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4",
                for (i, m) in ranked.iter().take(5).copied().enumerate() {
                    MiniRadarCard { rank: i + 1, lean: if i % 2 == 0 { -4.0 } else { 0.0 }, model: m }
                }
            }
            section { class: "grid grid-cols-2 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
                for (i, m) in ranked.iter().take(5).copied().enumerate() { PosterImageCard { rank: i + 1, model: m } }
            }
            section { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 xl:grid-cols-5",
                RankListCard {}
                RidgeCard {}
                BubbleCard {}
            }
        }
    }
}
