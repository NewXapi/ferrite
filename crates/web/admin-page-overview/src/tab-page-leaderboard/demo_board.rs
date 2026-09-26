//! 模型实力榜(演示)区块:头牌翻牌卡 + 立绘海报卡阵列 + 汇总图表。
//!
//! 全部由 `data` 层演示数值推导(后端暂无六维端点),排序口径按六维综合分降序;
//! 待真实源就绪后替换。

use dioxus::prelude::*;

use super::cards::{MiniRadarCard, PosterImageCard};
use super::charts::{GroupQuotaCard, ModelDistributionCard, PerformanceLatencyCard};
use super::data::{MODELS, ModelStat, composite};
use super::shared::{DEMO_COUNT_HEAD, DEMO_COUNT_TAIL, DEMO_TITLE};

/// 模型实力榜(演示)区块: 头牌翻牌卡 + 立绘海报卡阵列 + 汇总图表, 全部由 data 层演示数值推导。
/// 排序口径与恢复前版本一致: 按六维综合分降序。
#[component]
pub fn DemoBoard() -> Element {
    let mut ranked: Vec<&ModelStat> = MODELS.iter().collect();
    ranked.sort_by(|a, b| composite(b).partial_cmp(&composite(a)).unwrap());

    rsx! {
        section { "data-testid": "leaderboard-demo", role: "region", "aria-label": DEMO_TITLE,
            class: "flex flex-col gap-6 md:gap-8",
            div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800/80 pb-4",
                div {
                    h2 { class: "{ui::T_text_lg} {ui::T_font_bold} tracking-tight {ui::T_text_zinc_100} md:{ui::T_text_xl}", "{DEMO_TITLE}" }
                }
                span { class: "rounded-full border {ui::T_border_zinc_800} {ui::T_bg_zinc_900} px-3 py-1 {ui::T_text_xs} {ui::T_text_zinc_400}",
                    "{DEMO_COUNT_HEAD}{ranked.len()}{DEMO_COUNT_TAIL}"
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
            // 维护者批注(2026-09-21): 海报阵列放到头牌卡之后(还原默认顺序)
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
