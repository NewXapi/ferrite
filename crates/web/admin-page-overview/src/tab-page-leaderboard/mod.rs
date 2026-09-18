//! 排行榜 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 区块组合(不含渲染细节)
//! - `demo_board`:模型实力榜(演示)区块组合
//! - `rank_board`:真实用量榜三口径卡(取数口径枚举 + 单卡)
//! - `cards`:演示实力榜的立绘/翻牌卡映射层
//! - `charts`:演示实力榜的三张汇总图表卡
//! - `data`:演示数值层(六维数据与派生,待真实源替换)
//! - `insights`:真实用量榜的升降速双卡与厂商份额卡

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
#[path = "page.rs"]
pub mod page;
#[path = "rank_board.rs"]
pub mod rank_board;

pub use page::*;
