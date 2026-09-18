//! Dashboard section: 总览、模型、排行榜三个 tab。
//! 数据经 `api` 取用,面板不直接持有 mock。

pub mod api;

mod shared;

#[path = "tab-page-leaderboard/mod.rs"]
pub mod tab_page_leaderboard;
#[path = "tab-page-models/mod.rs"]
pub mod tab_page_models;
#[path = "tab-page-overview/mod.rs"]
pub mod tab_page_overview;

pub use tab_page_leaderboard::LeaderboardPanel;
pub use tab_page_leaderboard::insights;
pub use tab_page_models::ModelsPanel;
pub use tab_page_overview::OverviewPanel;
