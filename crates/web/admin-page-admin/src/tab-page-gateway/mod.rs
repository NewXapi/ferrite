//! 网关渠道健康 tab。拆分约定:
//! - `page`:面板外壳 — 状态 (拉取/错误/刷新) + 轮询拉取
//!   + 四态分支 (loading/error/empty/data),数据态逐项组合行组件
//! - `row`:单条渠道健康行 (纯展示,行级回退计算 + 取色)
//! - `shared`:三态 Badge 语义色 (表头计数与行徽标共用)

#[path = "page.rs"]
pub mod page;
#[path = "row.rs"]
pub mod row;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
