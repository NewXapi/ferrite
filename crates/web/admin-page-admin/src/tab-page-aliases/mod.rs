//! 模型别名管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `stats`:概览统计区(编号段 1,纯渲染)
//! - `toolbar`:筛选与操作区(编号段 2,Signal 绑定读写)
//! - `list`:卡片网格区(编号段 3,四态 + 网格)
//! - `card`:别名卡片(分组倍率标签 + 定价行 + 模式 toggle)
//! - `modal`:新建/编辑弹窗(基本 / 按量 / 按次 三 tab)
//! - `shared`:`AliasItem` / `AliasModalState` / `PriceMode`
//!   / `PriceModeToggle` / `usable_groups_for` / 区段文案

#[path = "card.rs"]
pub mod card;
#[path = "list.rs"]
pub mod list;
#[path = "modal.rs"]
pub mod modal;
#[path = "page.rs"]
pub mod page;
#[path = "shared.rs"]
pub mod shared;
#[path = "stats.rs"]
pub mod stats;
#[path = "toolbar.rs"]
pub mod toolbar;

pub use page::*;
pub use shared::*;
