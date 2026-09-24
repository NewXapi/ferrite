//! 用户页组件层:只放 `#[component]` 组件(spec 理念 2);跨页复用组件在
//! `ui-components`;文件按业务名/段落边界拆分,命名前缀区分归属。

pub mod badge;
pub mod chip;
pub mod group_chips;
pub mod modal;
pub mod role_chips;
pub mod topup_form;
pub mod user_actions;
pub mod user_card;
pub mod user_form;
pub mod users_filter;
pub mod users_list;
pub mod users_stats;

pub use chip::Chip;
pub use group_chips::GroupChips;
pub use role_chips::RoleChips;
pub use topup_form::TopUpForm;
pub use user_actions::UserActions;
pub use user_card::UserCard;
pub use user_form::{FormTab, TAB_LABELS, UserForm};
pub use users_filter::UsersFilterSection;
pub use users_list::UsersListSection;
pub use users_stats::UsersStatsSection;
