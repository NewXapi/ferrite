//! 兑换码管理共享类型、映射与文案。page / card / modal 三处复用。

use crate::api::RedemptionView;

pub const SEC_STATS: &str = "兑换码概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST: &str = "兑换码列表";

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum RedModalState {
    Closed,
    Generate,
    /// 生成成功后的一次性明文码展示
    Codes(Vec<String>),
}

/// 页面内兑换码视图模型 (金额已换算为 ¥)。
#[derive(Debug, Clone, PartialEq)]
pub struct RedRowFE {
    pub key: String,
    pub code_preview: String,
    /// 页面展示使用的 CNY 金额（后端 `quota` 按 500000 单位换算）。
    pub quota_cny: f64,
    pub status: u8, // 1 未使用 / 2 已核销 / 3 已停用
    pub redeemed_by: Option<String>,
    pub redeemed_at: String,
    pub created: String,
}

/// 把后端 `RedemptionView` 映射为页面视图模型。
///
/// 金额换算:后端 `quota` 是内部计费单位(500000 = ¥1),
/// 页面统一以 ¥ 展示。`redeemed_at` 缺省为空串(卡片据此隐藏核销时间行)。
pub fn map_redemption_view(v: RedemptionView) -> RedRowFE {
    RedRowFE {
        key: v.key,
        code_preview: v.code_preview,
        quota_cny: v.quota as f64 / 500_000.0,
        status: v.status as u8,
        redeemed_by: v.redeemed_by,
        redeemed_at: v.redeemed_at.unwrap_or_default(),
        created: v.created_at,
    }
}
