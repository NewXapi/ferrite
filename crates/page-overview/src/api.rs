//! Dashboard 各页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 三个页各占一个子模块 —— `fetch_models` 在总览页(分布占比)和模型页(卡片)
//! 是两码事,同名不同类型,靠子模块分开而不是改名。
//!
//! 现在多是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! 面板渲染无需改动。

/// 总览页:统计卡与工具/模型分布。
pub mod overview {
    /// 统计卡:(值, 标签)
    pub fn fetch_stats() -> &'static [(&'static str, &'static str)] {
        mock::overview::STATS
    }

    /// 按工具分布:(名称, 用量, 占比 %)
    pub fn fetch_tools() -> &'static [(&'static str, &'static str, f64)] {
        mock::overview::TOOLS
    }

    /// 按模型分布:(名称, 用量, 占比 %)
    pub fn fetch_models() -> &'static [(&'static str, &'static str, f64)] {
        mock::overview::MODELS
    }
}

/// 模型页:模型卡片。
pub mod models {
    // `ModelInfo.groups` 的行类型是 `mock::models::GroupPrice`;面板只读字段,不点名该类型,
    // 所以这里不转出去(转了就是一条 unused 警告)。真要点名时从 `mock::models` 取。
    pub use mock::models::ModelInfo;

    pub fn fetch_models() -> &'static [ModelInfo] {
        mock::models::MODELS
    }
}

/// 排行榜页:六维评分与立绘。
///
/// 与 overview / models 不同,这里不走 `mock` crate:排行榜的立绘用 `asset!()`
/// 宏注册,搬进 mock 会给那个零依赖 crate 拖上 dioxus。数据层就留在
/// `leaderboard::data`,本模块是面板侧的统一入口 —— 接上真实后端时只改这里
/// (数值换成请求,立绘仍从 `data` 取)。
pub mod leaderboard {
    // `key_stats` 返回的 `KeyStat` 面板只读字段、不点名类型,故不转出(否则一条 unused 警告)。
    pub use crate::leaderboard::data::{
        DIMS, MODELS, ModelStat, avg_norms, composite, dim_rank, dim_raw, key_stats, norms,
    };
}
