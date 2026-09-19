//! 模型别名管理 tab。拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `stats`:概览统计区(编号段 1,纯渲染)
//! - `toolbar`:筛选与操作区(编号段 2,Signal 绑定读写)
//! - `list`:卡片网格区(编号段 3,四态 + 网格;卡片本体为 ui-components 的 `AliasCard`)
//! - `modal`:新建/编辑弹窗(基本 / 按量 / 按次 三 tab)
//! - `shared`:`AliasItem` / `AliasModalState` / `PriceMode`
//!   / `PriceModeToggle`(后两者再导出自 ui-components) / `usable_groups_for` / 区段文案
//!
//! 边界:
//! - 本目录只服务别名这一个 tab;别名卡片的**视觉外壳**(四页签 + 多 tab 叠放
//!   高度)定义在 ui-components 的 `admin_card`,这里不复制卡片样式。
//! - 分组选择的候选来源(`parse_whitelist`)与分组页共用,取自 `tab_page_groups`,
//!   本目录不重复实现白名单解析。
//! - 表单校验与网络写回(PUT / DELETE /api/models)全部留在 `page.rs`,
//!   子组件只暴露 `EventHandler`;组件内不出现 `spawn` 请求。
//!
//! 导出面:`page` 与 `shared` 全量再导出,保持 `crate::tab_page_aliases::*` 路径稳定;
//! `stats` / `toolbar` / `list` / `modal` 的组件仅在本 tab 内部按路径直接引用。

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
