//! 定价 — TokenCounts → cost (内部单位)。
//!
//! V1: 静态倍率表 (contract catalog 快照); V2: new-api price_expression
//! 表达式引擎 (版本化编译/AST/冻结快照/请求规则)。
//!
//! 价格表来源: catalog 快照的模型价格 (对齐 mock::models::GroupPrice 三列:
//! input/output/cache, 单位 $/M tokens)。

/// 单模型价格 (对齐 mock::models::GroupPrice)。
#[derive(Debug, Clone, Copy)]
pub struct ModelPrice {
    /// $/M 输入 tokens。
    pub input: f64,
    /// $/M 输出 tokens。
    pub output: f64,
    /// $/M 缓存读 tokens。
    pub cache: f64,
    /// 分组倍率 (GroupRecord.rate_multiplier)。
    pub group_multiplier: f64,
}

/// 定价表 trait — catalog 快照的投影。
pub trait PriceTable: Send + Sync {
    fn lookup(&self, model: &str) -> Option<ModelPrice>;
}

/// 结算价计算。
///
/// 内部单位换算: 500_000 单位 = $1 (new-api 语义), 即
/// cost = tokens × price_per_million / 1e6 × 500_000 × group_multiplier。
/// TODO(#335): 定价表定型 — PriceRecord 进 contract records 还是 GroupRecord 内嵌?
pub fn price_of(counts: crate::scanner::TokenCounts, price: &ModelPrice) -> i64 {
    let _ = (counts, price);
    0
}
