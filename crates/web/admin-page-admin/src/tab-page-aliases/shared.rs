//! 模型别名管理共享类型、判定与文案。page / list / modal 三处复用。

use contract::api::admin::GroupDto;

use crate::state::AliasRow;
use crate::tab_page_groups::parse_whitelist;

pub const SEC_STATS: &str = "别名概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST: &str = "别名列表";
/// 别名列表项:后端 ModelView 的 key(UUID) + 页面展示行 + per-card 定价模式。
/// key 不并入 AliasRow — AliasRow 被 entities.rs 结构体字面量构造,
/// 本页独立持有 key 以定位 PUT/DELETE 路径;price_mode 是 UI 层本地状态,
/// 后端 models 域无 pricing_mode 列,保存不写回。
#[derive(Clone, PartialEq)]
pub struct AliasItem {
    pub key: String,
    pub row: AliasRow,
    pub price_mode: PriceMode,
}

/// 弹窗状态(Edit 携带后端模型 UUID key)
#[derive(Clone, PartialEq)]
pub enum AliasModalState {
    Closed,
    New,
    Edit(String),
}

/// 计算「可用此别名的分组及其倍率」。
///
/// 后端 `model_whitelist` 是「分组内可用的模型名列表」;空白名单 = 该分组
/// 可用全部模型。因此「可用此别名」= 白名单为空(默认全可用)或显式包含
/// 该别名。原型新卡与旧卡片网格共用此判定,避免两处逻辑分叉。
pub fn usable_groups_for(alias: &str, groups: &[GroupDto]) -> Vec<(String, f64)> {
    groups
        .iter()
        .filter(|g| {
            let names = parse_whitelist(&g.model_whitelist);
            names.is_empty() || names.iter().any(|n| n.as_str() == alias)
        })
        .map(|g| (g.name.clone(), g.ratio))
        .collect()
}

// ============ 定价模式 toggle（已上提到 ui-components，卡片/弹窗/本页共用） ============

// `PriceMode` 与 `PriceModeToggle` 原定义于此；卡片改由 ui-components 的
// `AliasCard` 承载后上提到该 crate（避免 ui-components 反向依赖页面 crate）。
// 此处再导出，保证 `crate::tab_page_aliases::{PriceMode, PriceModeToggle}`
// 这一既有路径继续可用，公开面零变化。
pub use ui::{PriceMode, PriceModeToggle};
