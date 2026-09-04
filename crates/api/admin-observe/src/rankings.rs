//! 模型排行 — 周期份额 + 环比 (对齐 mock::overview 的展示形状)。
//!
//! 产出: period (日/周/月) × model → tokens/cost/requests + prev_tokens 环比。
//! 参考: new-api rank_usage.go + store_usedata_rankings.go (movement 对比)。

use store::StoreError;

/// 重建某周期的排行 (幂等全量重算, 数据源 usage_hourly 而非原始表)。
/// TODO(#702): period 解析 ("day"/"week"/"month") + 上期值回填。
pub async fn rebuild(_period: &str) -> Result<(), StoreError> {
    todo!("TODO(#702)")
}

/// 查询排行 (面板: overview 的 TOOLS/MODELS 分布卡片)。
pub async fn query(_period: &str, _limit: u32) -> Result<serde_json::Value, StoreError> {
    todo!("TODO(#702)")
}
