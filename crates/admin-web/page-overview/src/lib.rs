//! Dashboard section: 总览、模型、排行榜三个 tab。
//! 数据经 `api` 取用,面板不直接持有 mock。

pub mod api;
mod leaderboard;
mod models;
mod overview;

pub use leaderboard::LeaderboardPanel;
pub use models::ModelsPanel;
pub use overview::OverviewPanel;
