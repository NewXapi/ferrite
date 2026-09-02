//! Dashboard 各页的数据来源。面板只从这里取数,不认识数据是怎么来的。

/// 总览页:统计卡与分布。
pub mod overview {
    /// 统计卡:(值, 标签)
    pub fn fetch_stats() -> &'static [(&'static str, &'static str)] {
        mock::overview::STATS
    }

    /// 按用户分布:(名称, 消耗量, 占比 %)
    pub fn fetch_users() -> &'static [(&'static str, &'static str, f64)] {
        mock::overview::USERS_TOP
    }

    /// 按模型分布:(名称, 消耗量, 占比 %)
    pub fn fetch_models() -> &'static [(&'static str, &'static str, f64)] {
        mock::overview::MODELS
    }

    // ponytail: mock timeframe stats
    pub fn fetch_timeframe_stats(
        _timeframe: &str,
    ) -> (
        Vec<(&'static str, &'static str)>,
        Vec<(&'static str, &'static str, f64)>, // users
        Vec<(&'static str, &'static str, f64)>, // models
    ) {
        let stats = fetch_stats().to_vec();
        let users = fetch_users().to_vec();
        let models = fetch_models().to_vec();

        (stats, users, models)
    }
}

/// 模型页:模型卡片。
pub mod models {
    pub use mock::models::ModelInfo;

    pub fn fetch_models() -> &'static [ModelInfo] {
        mock::models::MODELS
    }
}

/// 排行榜页:六维评分与立绘。
pub mod leaderboard {
    pub use crate::leaderboard::data::{
        DIMS, MODELS, ModelStat, avg_norms, composite, dim_rank, dim_raw, key_stats, norms,
    };
}
