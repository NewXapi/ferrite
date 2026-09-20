//! 渠道管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `stats`:概览统计区(编号段 1,纯渲染)
//! - `toolbar`:筛选与操作区(编号段 2,Signal 绑定读写)
//! - `list`:卡片网格区(编号段 3,四态 + 网格)
//! - `modal`:新建/编辑弹窗
//! - `shared`:`ChannelModalState` / `WriteOp` / 筛选与解析纯函数
//!
//! 边界:
//! - 网格卡片是 ui-components 的 `ChannelCard`(展示卡 + ≤3 tab + 行内 Popover
//!   编辑,UI 决策记录 §2.2/§2.3);密钥/模型/分组等多字段能力保留在弹窗。
//! - 分组候选取自 `tab_page_groups` 的接口,本目录不重复实现分组拉取逻辑。
//! - 弹窗的保存请求(创建 / 最小 diff 更新)与列表的启停 / 删除请求都留在
//!   `page.rs` 与 `modal.rs` 的提交闭包里;`list` 只暴露 `EventHandler`。
//!
//! 导出面:`page` 与 `shared` 全量再导出,保持 `crate::tab_page_channels::*` 路径稳定;
//! `stats` / `toolbar` / `list` / `modal` 的组件在本 tab 内部按路径直接引用。

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
