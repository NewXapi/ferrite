//! 网关渠道健康 tab:只读观测 gateway dispatch 的渠道冷却 / 慢启动状态。
//!
//! 负责:拉取 `GET /api/gateway/health`、按需 5s 轮询、四态渲染与单行展示。
//!
//! 不负责:健康状态的判定与冷却计时(服务端算好下发);本 tab 无任何写操作,
//! 也不干预渠道启停(那在 `tab-page-entities` 的渠道卡)。
//!
//! 拆分约定:
//! - `page`:面板外壳 — 状态 (拉取/错误/刷新) + 轮询拉取
//!   + 四态分支 (loading/error/empty/data),数据态逐项组合行组件
//! - `row`:单条渠道健康行 (纯展示,行级回退计算 + 取色)
//! - `shared`:三态 Badge 语义色 + 全部用户可见文案常量

#[path = "page.rs"]
pub mod page;
#[path = "row.rs"]
pub mod row;
#[path = "shared.rs"]
pub mod shared;

pub use page::*;
pub use shared::*;
