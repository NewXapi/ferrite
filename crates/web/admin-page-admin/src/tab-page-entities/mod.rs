//! 实体设置 tab:分组 / 模型别名 / 渠道三张可折叠卡,各占拓扑里对应的节点内容。
//!
//! 负责:三张卡的展开态组装与实体 CRUD 的界面入口(分组/渠道走 `drawer_write`
//! 真实端点,别名卡为演示态本地行)。
//!
//! 不负责:拓扑画布本身的渲染(在 `tab-page-network`),以及写路径的实现细节
//! (在 `crate::drawer_write`)。文件名与职责对应:`page.rs` 薄组装 + 展开态;
//! `cards.rs` 分组卡与别名卡;`channels.rs` 渠道卡;`shared.rs` 卡外壳、输入原子件、
//! 数值解析与全部用户可见文案常量。

#[path = "cards.rs"]
pub mod cards;
#[path = "channels.rs"]
pub mod channels;
#[path = "page.rs"]
pub mod page;
#[path = "shared.rs"]
pub mod shared;
pub use cards::*;
pub use channels::*;
pub use page::EntitiesPanel;
pub use shared::*;
