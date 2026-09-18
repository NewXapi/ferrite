//! 真实用量区洞察门面:上一窗取数 / 升降速双卡 / 厂商份额。
//!
//! 一轮重构前三者同在 516 行的 `insights.rs`;现按职责拆成
//! [`prev_window`](super::prev_window) / [`movers`](super::movers) /
//! [`vendors`](super::vendors) 三个文件,本文件只做再导出 ——
//! `admin_page_overview::insights::*` 是既有公开路径(页面与
//! `tests/leaderboard_logic.rs` 依赖),不因拆分而改变。

pub use super::movers::{
    MoversCards, MoversState, RankDelta, RankMove, names_by_tokens, rank_moves, top_droppers,
    top_movers,
};
pub use super::prev_window::{
    previous_window_start, previous_window_start_from, top_usage_between,
};
pub use super::shared::{MOVERS_LIMIT, slug};
pub use super::vendors::{VendorShare, VendorShareCard, vendor_color, vendor_of, vendor_shares};
