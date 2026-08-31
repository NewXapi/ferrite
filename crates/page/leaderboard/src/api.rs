//! 排行榜页的数据来源。卡片与图表只从这里取数。
//!
//! 与 account / users / overview / models 不同,这里不走 `mock` crate:排行榜的
//! 立绘用 `asset!()` 宏注册,搬进 mock 会给那个零依赖 crate 拖上 dioxus。
//! 数据层就留在 `leaderboard::data`,本文件是面板侧的统一入口 —— 接上真实
//! 后端时只改本文件(数值换成请求,立绘仍从 `data` 取)。

// `key_stats` 返回的 `KeyStat` 面板只读字段、不点名类型,故不转出(否则一条 unused 警告)。
pub use crate::leaderboard::data::{
    avg_norms, composite, dim_rank, dim_raw, key_stats, norms, ModelStat, DIMS, MODELS,
};
