//! 管理页卡片族及其共享基元。
//!
//! 本模块提供统一的卡片外壳（`AdminCard` / `CardShell` / `SectionHeader` /
//! 分页器）与真实实体卡：`aliases` / `channels` / `users`。早期只读原型卡
//! （分组 / 兑换码）已在列表接入共用样式壳后删除，避免与真实卡重复维护。

/// 别名实体卡原型。
pub mod aliases;
/// 管理卡片外壳。
pub mod card;
/// 渠道实体卡原型。
pub mod channels;
/// 圆点页签切换器。
pub mod dot_tab;
/// 行内 Popover 编辑原语（展示行 → 点击编辑 → 保存）。
pub mod editable;
/// 原地编辑原语（点值 → 值变无边框输入框，Quasar borderless 等价物）。
pub mod inline_edit;
/// 卡片网格分页器（tab 式页码条）与切片纯函数。
pub mod pager;
/// 别名定价模式（按量 / 按次）及分段 toggle。
pub mod price_mode;
/// tab 内容区共享壳组件（区段 / 空态 / 卡片外壳等）。
pub mod shell;
/// 用户实体卡（真实用户网格卡，3 tab + 操作插槽）。
pub mod users;

/// 供管理页展示别名摘要的卡片组件。
pub use aliases::AliasCard;
/// 别名卡行内 Popover 可编辑字段枚举。
pub use aliases::AliasEditField;
/// 提供标题与只读圆点页签的共享卡片组件。
pub use card::AdminCard;
/// 供管理页展示渠道摘要的卡片组件。
pub use channels::ChannelCard;
/// 渠道卡行内 Popover 可编辑字段枚举。
pub use channels::ChannelEditField;
/// 用普通按钮呈现只读内容页签的圆点切换组件。
pub use dot_tab::DotTabBar;
/// 行内 Popover 编辑原语：展示行点击编辑与危险操作确认行。
pub use editable::{
    DANGER_ROW_CLASS, DangerActionRow, EDIT_POPOVER_CLASS, EDITABLE_ROW_CLASS, EditableRow,
};
/// 原地编辑原语：可编辑行与无边框输入框 class。
pub use inline_edit::{INLINE_INPUT_CLASS, InlineEdit};
/// 卡片网格分页器与分页纯函数（页大小常量 / 切片 / 页数）。
pub use pager::{CARD_PAGE_SIZE, Pager, page_count, page_slice};
/// 定价模式枚举与分段 toggle（卡片 / 弹窗共用）。
pub use price_mode::{PriceMode, PriceModeToggle};
/// 供管理页展示用户摘要的卡片组件（真实网格卡，操作由页面以 slot 注入）。
pub use users::UserCard;

/// 区段外壳、空态块、卡片外壳等 tab 内容区共享壳（含 class 常量）。
pub use shell::{
    AdminSection, CARD_SHELL_CLASS, CardGrid, CardShell, DangerBlock, GhostButton,
    PlaceholderBlock, SectionHeader,
};
