//! 真实用量榜的三张口径榜:取数口径枚举 + 单卡(名次/名称/条形/数值)。
//!
//! 呈现委托 ui-components 的 `RankBoard`;本层只做口径排序、取前 10、份额分母
//! 与字段格式化(业务换算不进共享组件)。

use dioxus::prelude::*;

use super::shared::RANK_FOOTNOTE;
use crate::api::{UsageTopRow, fmt_usd, growth_of, share_text};
use crate::shared::{MODEL_COLORS, fmt_raw};
use ui::components::rank_board::{RankBoard, RankRowMeta, RankRowView};

/// 排行榜取数口径:同一批 /api/log/top 聚合行,按不同字段重排展示。
/// 用枚举而非 fn 指针传参:component 宏会为 props 生成 PartialEq,函数指针比较不可靠。
#[derive(Clone, Copy, PartialEq)]
pub enum RankMetric {
    /// prompt + completion tokens 合计
    Tokens,
    /// 消费请求数
    Calls,
    /// 计费额度(500000 = $1,复用 api.rs fmt_usd)
    Quota,
}

impl RankMetric {
    /// 该口径下行的排序键。
    fn of(self, r: &UsageTopRow) -> i64 {
        match self {
            Self::Tokens => r.tokens,
            Self::Calls => r.calls,
            Self::Quota => r.quota,
        }
    }

    /// 该口径下行的展示串。
    fn fmt(self, r: &UsageTopRow) -> String {
        match self {
            Self::Tokens => fmt_raw(r.tokens),
            Self::Calls => r.calls.to_string(),
            Self::Quota => fmt_usd(r.quota),
        }
    }
}

/// 单个排行卡:按 `metric` 从真实聚合行里取前 N,画名次 + 名称 + 条形 + 数值。
/// 呈现全部委托 ui-components 的 [`ui::components::rank_board::RankBoard`]
/// (维护者要求三张口径榜抽象为共享组件复用);本层只做口径排序/取前 10/份额分母
/// 与字段格式化(业务换算不进共享组件)。
///
/// - 是什么:三张口径榜(Tokens / Calls / Quota)共用的单卡,差异只在排序键与
///   展示格式,故用 [`RankMetric`] 枚举注入(不用函数指针:component 宏为 props
///   生成 PartialEq,函数指针比较不可靠)。
/// - 数据流通:入参 `rows` 为后端整批聚合行(不截前 10),份额分母与最大值都
///   在本层算;对外只把整形好的 `RankRowView` 交给共享组件。
/// - 样式:卡面全部委托共享组件;脚注固定为增长率/份额的口径说明。
#[component]
pub fn RankCard(
    title: &'static str,
    subtitle: &'static str,
    testid: &'static str,
    metric: RankMetric,
    rows: Vec<UsageTopRow>,
) -> Element {
    // 排序副本:后端只保证 tokens 降序,calls/quota 榜需本地重排
    let mut sorted: Vec<&UsageTopRow> = rows.iter().collect();
    sorted.sort_by_key(|r| std::cmp::Reverse(metric.of(r)));
    let max_v = sorted.first().map(|r| metric.of(r)).unwrap_or(0).max(1);
    // 份额分母 = 当榜行值合计(后端拉回的整批行,不截前 10);
    // 口径跟随所选 metric(Tokens/Calls/Quota 各自占各自口径的合计)
    let total: i64 = rows.iter().map(|r| metric.of(r)).sum();
    let view_rows: Vec<RankRowView> = sorted
        .iter()
        .take(10)
        .enumerate()
        .map(|(i, r)| {
            let r: &UsageTopRow = r;
            let v = metric.of(r);
            // 环比标签只在上一窗确有数据(previous_tokens > 0)时展示:
            // 消耗榜里「↑new」语义不成立(维护者反馈),旧 wire 缺字段或新进榜一律不标
            let meta = if r.previous_tokens > 0 {
                growth_of(r.previous_tokens, r.tokens).map(|g| RankRowMeta {
                    label: g.label().to_string(),
                    class: g.text_class(),
                })
            } else {
                None
            };
            RankRowView {
                key: r.name.clone(),
                rank: i + 1,
                name: r.name.clone(),
                value: metric.fmt(r),
                meta,
                share: share_text(v, total),
                bar_pct: (v as f64 / max_v as f64 * 100.0).max(2.0),
                bar_color: MODEL_COLORS[i % MODEL_COLORS.len()].to_string(),
            }
        })
        .collect();

    rsx! {
        RankBoard {
            title: title.to_string(),
            subtitle: subtitle.to_string(),
            testid: testid.to_string(),
            rows: view_rows,
            footnote: RANK_FOOTNOTE.to_string(),
        }
    }
}
