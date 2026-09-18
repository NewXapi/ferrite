//! 模型别名管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `card`:别名卡片(分组倍率标签 + 定价行 + 模式 toggle)
//! - `modal`:新建/编辑弹窗(基本 / 按量 / 按次 三 tab)
//! - `shared`:`AliasItem` / `AliasModalState` / `PriceMode`
//!   / `PriceModeToggle` / `usable_groups_for` / 区段文案

#[path = "page.rs"]
pub mod page;
#[path = "card.rs"]
pub mod card;
#[path = "modal.rs"]
pub mod modal;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
