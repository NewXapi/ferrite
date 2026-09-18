//! 模型 tab。拆分约定:
//! - `page`:状态 + 拉取 effect + 四态分支与卡片网格组合
//! - `card`:`ModelCard` —— `ModelCardView` → `StatTabsCard` 的数据映射
//! - `shared`:本 tab 文案常量(i18n,含卡面标签与状态值)

mod shared;

#[path = "card.rs"]
pub mod card;
#[path = "page.rs"]
pub mod page;

pub use card::*;
pub use page::*;
