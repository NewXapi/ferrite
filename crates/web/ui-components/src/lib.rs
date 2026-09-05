//! # ui-components — 全栈通用的 Web 组件库
//!
//! 整合原 admin-ui 与 tavern-ui，按文件清晰拆分：
//! - auth_modal — 通用认证弹窗与用户状态微标 (AuthModal, UserBadge)
//! - session — 会话凭证管理与登录注册客户端
//! - form — 表单基元 (Field, CodeField, SubmitButton, SliderField)
//! - feedback — 头像、图标按钮、空态、加载指示器 (Avatar, IconButton, EmptyState, Loading)
//! - bubble — 对话气泡与分支切换器 (MessageBubble, SwipePicker)
//! - card — 状态与行动决策卡片 (StatusCard, ChoiceCard, ChoiceOption)
//! - dialog — 确认弹窗 (Dialog)
//! - scroll_spy — 滚动监听导航 (ScrollSpyNav)
//! - segmented — 分段胶囊选择器 (SegmentedCapsule)

pub mod auth_modal;
pub mod bubble;
pub mod card;
pub mod dialog;
pub mod feedback;
pub mod form;
pub mod scroll_spy;
pub mod segmented;
pub mod session;

pub use auth_modal::{AuthModal, UserBadge};
pub use bubble::{MessageBubble, SwipePicker};
pub use card::{ChoiceCard, ChoiceOption, StatusCard};
pub use dialog::Dialog;
pub use feedback::{Avatar, EmptyState, IconButton, Loading};
pub use form::{CodeField, Field, FormField, SliderField, SubmitButton};
pub use scroll_spy::ScrollSpyNav;
pub use segmented::SegmentedCapsule;
pub use session::{
    api_login, api_register, clear_cached_session, get_cached_token, get_cached_user,
    set_cached_session,
};
