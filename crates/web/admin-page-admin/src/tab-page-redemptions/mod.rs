//! 兑换码管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `stats`:概览统计区(编号段 1,纯渲染)
//! - `toolbar`:筛选与操作区(编号段 2,Signal 绑定读写)
//! - `list`:卡片网格区(编号段 3,四态 + 网格;copied_key 组件内部持有)
//! - `card`:兑换码卡片
//! - `modal`:生成弹窗 / 明文码展示
//! - `shared`:`RedModalState` / `RedRowFE` / `map_redemption_view` / 区段文案

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
