//! 分组管理 tab。拆分约定：
//! - `page`：状态 + 拉取/写回逻辑 + 组合（不含渲染细节）
//! - `toolbar`：筛选搜索 + 分级胶囊 + 批量点选与动作条
//! - `list`：卡片网格四态（loading/error/empty/data）
//! - `modal`：StatCard / Badge / GroupCard / GroupFormModal / Modal
//! - `shared`：`ModalState` / `WriteOp` / 白名单解析 + 区段文案

#[path = "page.rs"]
pub mod page;
#[path = "modal.rs"]
pub mod modal;
#[path = "toolbar.rs"]
pub mod toolbar;
#[path = "list.rs"]
pub mod list;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
pub use modal::*;
