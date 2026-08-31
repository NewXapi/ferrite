//! Overview page: stats grid, calendar heatmap, tool/model breakdown.
//! 数据经 `api` 取用,面板不直接持有 mock。

mod api;
mod overview;

pub use overview::OverviewPanel;
