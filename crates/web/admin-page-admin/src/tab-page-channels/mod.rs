//! 渠道管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合
//! - `card`:单渠道卡片(状态徽标 + 指标行 + 三键操作)
//! - `modal`:新建/编辑综合弹窗
//! - `shared`:`ChannelModalState` / `WriteOp` + 筛选与输入解析纯函数

#[path = "page.rs"]
pub mod page;
#[path = "card.rs"]
pub mod card;
#[path = "modal.rs"]
pub mod modal;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
// 仅再导出原先就是 pub 的纯函数(弹窗状态与写操作枚举是页面内部类型,不扩大公开面)
pub use shared::{filter_channels, parse_group_input, parse_keys_input};
