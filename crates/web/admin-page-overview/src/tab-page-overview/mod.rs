//! 总览 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 组件组合(不含渲染细节)
//! - `shared`:本 tab 文案常量(i18n)与跨区块共享类型(`TrendTip` / `StatCardView`)
//! - `trend`:用量趋势面板骨架(标题 + 时间窗 + 四态分支 + 组合,编号段 1)
//! - `histogram`:趋势左栏堆叠直方图(网格 + 柱 + 两级悬浮事件)
//! - `summary`:趋势右栏数据位四宫格 + 主力模型 Top5 图例
//! - `tooltip`:两级悬浮卡(通用外框 + 整列/单段两种卡体)
//! - `health`:渠道健康面板(内嵌,独立信号独立拉取)
//! - `errors`:近 24 小时错误面板(内嵌,独立信号独立拉取)
//! - `stats`:实时汇总统计区(编号段 3,区头 + 统计卡 + 额度余量卡)
//! - `sparkline`:统计卡底部迷你面积线
//! - `top_lists`:消耗前十模型/用户榜的行视图、行组件与共用卡外壳(编号段 4)

mod shared;

#[path = "errors.rs"]
pub mod errors;
#[path = "health.rs"]
pub mod health;
#[path = "histogram.rs"]
pub mod histogram;
#[path = "page.rs"]
pub mod page;
#[path = "sparkline.rs"]
pub mod sparkline;
#[path = "stats.rs"]
pub mod stats;
#[path = "summary.rs"]
pub mod summary;
#[path = "tooltip.rs"]
pub mod tooltip;
#[path = "top_lists.rs"]
pub mod top_lists;
#[path = "trend.rs"]
pub mod trend;

pub use errors::ErrorsPanel;
pub use health::ChannelHealth;
pub use page::*;
pub use stats::{QuotaRemainingCard, StatCard, StatsSection};
pub use top_lists::{TopListCard, TopRowFE, TopRowItem};
pub use trend::TrendPanel;
