//! 兑换码管理 tab(目录化拆分)。负责兑换码的列表展示、筛选、批量生成与停用。
//!
//! 后端语义:核销 (status→2)、DELETE 即停用 (status→3);无硬删、无重新启用。
//!
//! 拆分约定:
//! - `page`:状态 + 拉取/写回逻辑 + 组件组合(无渲染细节)
//! - `stats`:概览统计区(编号段 1,纯渲染)
//! - `toolbar`:筛选与操作区(编号段 2,Signal 绑定读写)
//! - `list`:卡片网格区(编号段 3,四态 + 网格;copied_key 组件内部持有)
//! - `card`:兑换码卡片
//! - `modal`:生成弹窗 / 明文码展示
//! - `shared`:`RedModalState` / `RedRowFE` / `map_redemption_view` / 全部文案常量
//!
//! 公开面:`pub use page::*` 与 `pub use shared::*` 对外导出页面与共享类型;
//! `stats` / `toolbar` / `list` / `card` / `modal` 仅在本 tab 内使用。
//! 边界:不含其他 tab 的任何逻辑;弹窗外壳与 `StatCard` / `Badge` 借用
//! `tab-page-groups`(反向依赖页面 crate 内兄弟模块,非 ui-components)。

#[path = "card.rs"]
pub mod card;
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
