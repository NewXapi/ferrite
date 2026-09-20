//! 货币管理 tab。
//!
//! 负责:`currency_defs` 的列表 / 新增 / 编辑 / 软禁用(删除即停用),数据来自
//! 真实后端 `/api/currency`(admin bearer)。
//!
//! 不负责:汇率换算与钱包扣费(在 0014 换算层与服务端);本 tab 只维护货币定义。
//!
//! 拆分约定：
//! - `page`：状态 + 拉取/写回逻辑 + 组合（不含渲染细节）
//! - `list`：列表四态（loading/error/empty/data）+ 表格行
//! - `form`：新增/编辑录入表单
//! - `shared`：`Kind` 类型 + 全部用户可见文案常量

#[path = "form.rs"]
pub mod form;
#[path = "list.rs"]
pub mod list;
#[path = "page.rs"]
pub mod page;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
