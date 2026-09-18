//! 货币管理 tab。拆分约定：
//! - `page`：状态 + 拉取/写回逻辑 + 组合（不含渲染细节）
//! - `list`：列表四态（loading/error/empty/data）+ 表格行
//! - `form`：新增/编辑录入表单
//! - `shared`：`Kind` 类型与区段文案

#[path = "page.rs"]
pub mod page;
#[path = "list.rs"]
pub mod list;
#[path = "form.rs"]
pub mod form;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
