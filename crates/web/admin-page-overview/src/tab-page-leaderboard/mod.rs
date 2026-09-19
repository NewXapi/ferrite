//! 排行榜 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 区块组合(不含渲染细节)
//! - `shared`:本 tab 文案常量(i18n)与 `slug` 纯函数
//! - `toolbar`:真实用量榜工具条(标题 + 口径副标题 + 时间窗切换)
//! - `demo_board`:模型实力榜(演示)区块组合
//! - `rank_board`:真实用量榜三口径卡(取数口径枚举 + 单卡)
//! - `cards`:演示实力榜的立绘/翻牌卡映射层
//! - `charts`:演示实力榜的三张汇总图表卡
//! - `data`:演示数值层(六维数据与派生,待真实源替换)
//! - `prev_window`:上一等长窗起点推导与 `[start, end)` 区间取数
//! - `movers`:名次变动纯判定 + 上升/下跌最快双卡
//! - `vendors`:厂商前缀推断 + 份额聚合 + 100% 堆叠条卡
//! - `insights`:洞察区公开门面(再导出上述三个,保住既有模块路径)

mod shared;

#[path = "cards.rs"]
pub mod cards;
#[path = "charts.rs"]
pub mod charts;
#[path = "data.rs"]
pub mod data;
#[path = "demo_board.rs"]
pub mod demo_board;
#[path = "insights.rs"]
pub mod insights;
#[path = "movers.rs"]
pub mod movers;
#[path = "page.rs"]
pub mod page;
#[path = "prev_window.rs"]
pub mod prev_window;
#[path = "rank_board.rs"]
pub mod rank_board;
#[path = "toolbar.rs"]
pub mod toolbar;
#[path = "vendors.rs"]
pub mod vendors;

pub use insights::{MoversCards, MoversState, VendorShareCard};
pub use page::*;
pub use rank_board::{RankCard, RankMetric};
