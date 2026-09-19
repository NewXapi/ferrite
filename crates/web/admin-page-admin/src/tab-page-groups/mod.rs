//! 分组管理 tab(目录化拆分)。负责分组实体的列表展示、筛选、批量启停与增删改。
//!
//! 拆分约定：
//! - `page`：状态 + 拉取/写回逻辑 + 组合（不含渲染细节）
//! - `stats`：概览统计区(编号段 1,纯渲染)
//! - `toolbar`：筛选搜索 + 分级胶囊 + 批量点选与动作条(编号段 2)
//! - `list`：卡片网格四态 + 首卡示例(编号段 3)
//! - `modal`：StatCard / Badge / GroupCard / GroupFormModal / Modal
//! - `shared`：`ModalState` / `WriteOp` / 白名单解析 + 全部文案常量
//!
//! 公开面:`pub use modal::*` 与 `pub use page::*` 对外导出页面与通用小件
//! (redemptions 复用 `StatCard` / `Badge` / `Modal`);`list` / `stats` / `toolbar`
//! 仅在本 tab 内使用。边界:不含其他 tab 的任何逻辑。

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

pub use modal::*;
pub use page::*;
pub use shared::*;
