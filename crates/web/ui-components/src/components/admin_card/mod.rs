//! 管理页新卡片原型及其共享基元。
//!
//! 本模块提供只读内容页签与统一的卡片外壳，供各实体卡复用；它不会替换旧卡片。

/// 别名实体卡原型。
pub mod aliases;
/// 管理卡片外壳。
pub mod card;
/// 渠道实体卡原型。
pub mod channels;
/// 圆点页签切换器。
pub mod dot_tab;
/// 分组实体卡原型。
pub mod groups;
/// 兑换码实体卡原型。
pub mod redemptions;
/// 用户实体卡原型。
pub mod users;

/// 供管理页展示别名摘要的卡片组件。
pub use aliases::AliasCard;
/// 提供标题与只读圆点页签的共享卡片组件。
pub use card::AdminCard;
/// 供管理页展示渠道摘要的卡片组件。
pub use channels::ChannelCard;
/// 用普通按钮呈现只读内容页签的圆点切换组件。
pub use dot_tab::DotTabBar;
/// 供管理页展示分组摘要的卡片组件。
pub use groups::GroupCard;
/// 供管理页展示兑换码摘要的卡片组件。
pub use redemptions::RedemptionCard;
/// 供管理页展示用户摘要的卡片组件。
pub use users::UserCard;

/// 区段外壳、空态块、卡片外壳等 tab 内容区共享壳（含 class 常量）。
pub mod shell;
pub use shell::{
    AdminSection, CARD_SHELL_CLASS, CardGrid, CardShell, DangerBlock, GhostButton,
    PlaceholderBlock, SectionHeader,
};
