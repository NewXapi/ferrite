//! Leaderboard page restored from ui/overview.
//! 卡片与图表经 `api` 取数;数据层 `leaderboard::data` 因 `asset!()` 立绘留在本 crate。

mod api;
mod leaderboard;
pub use leaderboard::LeaderboardPanel;
