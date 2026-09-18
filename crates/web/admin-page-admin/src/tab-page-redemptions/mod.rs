//! 兑换码管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `card`:兑换码卡片(状态徽标 + 面额条 + 停用操作)
//! - `modal`:批量生成弹窗 + 明文码一次性展示
//! - `shared`:`RedModalState` / `RedRowFE`
//!   / `map_redemption_view` / 区段文案

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
