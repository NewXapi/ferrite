//! 总览 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 组件组合(不含渲染细节)
//! - `trend`:用量趋势大面板(编号段 1,时间窗 + 堆叠直方图 + 两级悬浮卡)
//! - `health`:渠道健康面板(内嵌,独立信号独立拉取)
//! - `errors`:近 24 小时错误面板(内嵌,独立信号独立拉取)
//! - `stats`:实时汇总统计卡区(编号段 3,含额度余量卡)
//! - `sparkline`:统计卡底部迷你面积线
//! - `top_lists`:消耗前十模型/用户榜(编号段 4)

#[path = "errors.rs"]
pub mod errors;
#[path = "health.rs"]
pub mod health;
#[path = "page.rs"]
pub mod page;
#[path = "sparkline.rs"]
pub mod sparkline;
#[path = "stats.rs"]
pub mod stats;
#[path = "top_lists.rs"]
pub mod top_lists;
#[path = "trend.rs"]
pub mod trend;

pub use errors::ErrorsPanel;
pub use health::ChannelHealth;
pub use page::*;
