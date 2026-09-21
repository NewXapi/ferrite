//! 用户管理 Tab 页面模块。拆分约定:
//! - `page`:页面层——状态 + 拉取 effect + 区段组合(统计 / 筛选 / 卡片网格)
//! - `user_card`:单张用户卡(徽标行 + 额度进度条 + 计数行 + 操作区)
//! - `badge`:分组 / 角色 / 状态共用的胶囊徽标
//! - `modal`:弹窗外壳与输入框样式(表单弹窗与充值弹窗共用)
//! - `user_form`:新建 / 编辑弹窗(含弹窗内三个真实页签)
//! - `group_chips`:生效分组多选 chips
//! - `role_chips`:角色权限单选 chips
//! - `topup_form`:额度充值弹窗
//! - `shared`:该 tab 独占的跨组件共用文案常量(按 LBL_/BTN_/SEC_/FIELD_/MSG_/OPT_ 前缀)

pub mod badge;
pub mod group_chips;
pub mod modal;
pub mod page;
pub mod role_chips;
pub mod shared;
pub mod topup_form;
pub mod user_card;
pub mod user_form;

pub use group_chips::GroupChips;
pub use page::UsersPanel;
pub use role_chips::RoleChips;
pub use user_card::UserCard;
pub use user_form::{FormTab, TAB_LABELS, UserForm};
