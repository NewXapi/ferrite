//! 排行榜卡牌映射层: 把 data 层的 [`ModelStat`] 及其派生数值映射成 ui-components
//! showcase 卡牌组件的 props.
//!
//! 雷达几何 (RadarGeo)、角标三档样式 (badge_style)、倾斜 hover (tilt_from)、立绘
//! 占位 (art_img) 与关键数据行渲染均已抽入 `ui::components::showcase`, 本文件不再
//! 持有任何几何与样式; 两个组件的对外签名保持不变 (调用方 `leaderboard::mod` 零改动)。
//! 卡内交互 (翻牌 / 鼠标倾斜) 与统一 hover 边框变亮也随组件下沉, 不在本层重复。

use dioxus::prelude::*;

use ui::components::showcase::{PosterCard, RadarFlipCard};
use ui::components::showcase::{poster_card::KeyStatLine, radar_flip_card::DimRow};

use super::data::{DIMS, ModelStat, avg_norms, composite, dim_rank, dim_raw, key_stats, norms};

/// 立绘海报卡: 正面 = 立绘 + 浓缩雷达 (角标/光点/均值虚线) + 三条关键数据,
/// 点击顺时针翻 180°, 背面 = 暗化立绘 + 六维横向直方图 + 综合分。
/// 把 [`ModelStat`] 的六维数值与派生结果 (名次 / 原始值 / 关键数据) 映射为
/// [`PosterCard`] props, 数据口径与映射前一致。
#[component]
pub fn PosterImageCard(rank: usize, model: &'static ModelStat) -> Element {
    let key_stat_lines: Vec<KeyStatLine> = key_stats(model)
        .into_iter()
        .map(|s| KeyStatLine {
            short: s.short.to_string(),
            full: s.full,
            text: s.text,
        })
        .collect();
    let dim_labels: [String; 6] = std::array::from_fn(|i| DIMS[i].to_string());

    rsx! {
        PosterCard {
            rank: rank,
            name: model.name.to_string(),
            desc: model.desc.to_string(),
            art: model.art,
            radar_values: norms(model),
            radar_avg: avg_norms(),
            dim_ranks: dim_rank(model),
            dim_labels: dim_labels,
            dim_raws: dim_raw(model),
            score: composite(model),
            key_stats: key_stat_lines,
        }
    }
}

/// 头牌翻牌卡: 左侧翻牌立绘 (正面立绘 / 背面六维明细), 右侧信息面板 (名牌 + 描述)。
/// 把 [`ModelStat`] 的六维原始值映射为 [`RadarFlipCard`] props。
#[component]
pub fn MiniRadarCard(
    rank: usize,
    /// 立绘斜角 (度), 0 = 直立
    lean: f64,
    model: &'static ModelStat,
) -> Element {
    let raw = dim_raw(model);
    let dim_rows: Vec<DimRow> = (0..6)
        .map(|i| DimRow {
            label: DIMS[i].to_string(),
            value: raw[i].clone(),
        })
        .collect();

    rsx! {
        RadarFlipCard {
            rank: rank,
            lean: lean,
            name: model.name.to_string(),
            desc: model.desc.to_string(),
            art: model.art,
            dim_rows: dim_rows,
            score: composite(model),
        }
    }
}
