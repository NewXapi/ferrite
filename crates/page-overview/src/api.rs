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

    /// 用量趋势的一个时间桶: [label 显示文本, 是否在轴上标出, 桶总量(百万), 各模型用量]
    /// `per_model` 与 `fetch_models()` 同序同长。
    pub struct TrendBucket {
        pub label: String,
        pub show_label: bool,
        pub total: f64,
        pub per_model: Vec<f64>,
    }

    // ponytail: 确定性伪随机;接真实后端时整段换成按窗口聚合即可,面板不动。
    fn hash01(a: u64, b: u64) -> f64 {
        let mut x = a.wrapping_mul(374761393).wrapping_add(b.wrapping_mul(668265263));
        x = (x ^ (x >> 13)).wrapping_mul(1274126177);
        ((x ^ (x >> 16)) % 1000) as f64 / 1000.0
    }

    /// 用量趋势: 今天→24小时, 本周→7天, 本月→30天, 今年→12月。单位百万 tokens。
    pub fn fetch_trend(timeframe: &str) -> Vec<TrendBucket> {
        let (n, scale, labels): (usize, f64, Vec<String>) = match timeframe {
            "今天" => (24, 320.0, (0..24).map(|h| format!("{h:02}:00")).collect()),
            "本周" => (
                7,
                3500.0,
                ["周一", "周二", "周三", "周四", "周五", "周六", "周日"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            ),
            "本月" => (30, 9000.0, (1..=30).map(|d| format!("{d}")).collect()),
            _ => (12, 30000.0, (1..=12).map(|m| format!("{m}月")).collect()),
        };
        let shares: Vec<f64> = fetch_models().iter().map(|m| m.2 / 100.0).collect();

        (0..n)
            .map(|b| {
                let b = b as u64;
                let total = scale * (0.30 + 0.70 * hash01(b, 77));
                // 各模型分桶扰动后归一化,使 per_model 之和恰为 total
                let weights: Vec<f64> = shares
                    .iter()
                    .enumerate()
                    .map(|(i, &s)| s * (0.4 + 0.6 * hash01(b, i as u64)))
                    .collect();
                let wsum: f64 = weights.iter().sum();
                let per_model = weights.iter().map(|w| total * w / wsum).collect();
                let show_label = match n {
                    24 => b % 4 == 0,
                    30 => b % 5 == 0,
                    _ => true,
                };
                TrendBucket {
                    label: labels[b as usize].clone(),
                    show_label,
                    total,
                    per_model,
                }
            })
            .collect()
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
