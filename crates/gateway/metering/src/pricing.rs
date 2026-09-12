//! 定价 — TokenCounts → cost (内部单位)。
//!
//! V1: 静态倍率表 (contract catalog 快照); V2: new-api price_expression
//! 表达式引擎 (版本化编译/AST/冻结快照/请求规则)。
//!
//! 价格表来源: catalog 快照的模型价格 (对齐 mock::models::GroupPrice 三列:
//! input/output/cache, 单位 $/M tokens)。

use std::collections::HashMap;

/// 单模型价格 (对齐 mock::models::GroupPrice)。
///
/// 可从配置反序列化：`apps/gateway` 的 `[metering.prices.<model>]` 段直接读成本类型，
/// 避免在 binary 侧重复定义同字段结构。
#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ModelPrice {
    /// $/M 输入 tokens。
    pub input: f64,
    /// $/M 输出 tokens。
    pub output: f64,
    /// $/M 缓存读 tokens。
    pub cache: f64,
    /// 分组倍率 (GroupRecord.rate_multiplier)。
    #[serde(default = "default_group_multiplier")]
    pub group_multiplier: f64,
}

fn default_group_multiplier() -> f64 {
    1.0
}

/// 定价表 trait — catalog 快照的投影。
pub trait PriceTable: Send + Sync {
    /// 查 `(model, group)` 的价。
    ///
    /// `group` 是请求分组名（如 "default" / "vip"）；实现可按组返回不同价。
    /// 返回值里的 [`ModelPrice::group_multiplier`] 参与 [`price_of`] 计算；
    /// 组倍率的**请求侧真实来源**（GroupRecord.rate_multiplier）由调用方作为
    /// `group_ratio` 传给 [`price_of`] / [`crate::settle_event`]，不经本表。
    fn lookup(&self, model: &str, group: &str) -> Option<ModelPrice>;
}

/// 结算价计算。
///
/// 内部单位换算: 500_000 单位 = $1 (new-api 语义), 即
/// cost = (in+out+cache 美元) × price.group_multiplier × group_ratio × 500_000，
/// 向上取整（`.ceil()`：非零消耗至少 1 单位，零消耗仍为 0）。
///
/// `group_ratio` 是请求分组倍率（GroupRecord.rate_multiplier），缺省 1.0；
/// 与表内 `group_multiplier` 相乘，两级倍率语义各自独立。
pub fn price_of(counts: crate::scanner::TokenCounts, price: &ModelPrice, group_ratio: f64) -> i64 {
    let input_cost = counts.prompt as f64 * price.input / 1e6;
    let output_cost = counts.completion as f64 * price.output / 1e6;
    let cache_cost = counts.cached as f64 * price.cache / 1e6;
    let total_dollars =
        (input_cost + output_cost + cache_cost) * price.group_multiplier * group_ratio;
    (total_dollars * 500_000.0).ceil() as i64
}
/// 来自配置的定价表实现。
#[derive(Debug, Clone)]
pub struct ConfigPriceTable {
    prices: HashMap<String, ModelPrice>,
}

impl ConfigPriceTable {
    pub fn new(prices: HashMap<String, ModelPrice>) -> Self {
        Self { prices }
    }
}

impl PriceTable for ConfigPriceTable {
    /// 配置表按 model 命中；组倍率的真实来源（`group_ratio`）由调用方在
    /// settle 时传入，本实现不使用 `group` 参数。
    fn lookup(&self, model: &str, _group: &str) -> Option<ModelPrice> {
        self.prices.get(model).copied()
    }
}
