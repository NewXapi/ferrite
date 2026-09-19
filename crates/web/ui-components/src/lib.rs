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
//! - action_buttons — 卡片底部操作按钮组 (ActionButtonGroup, ActionSpec, ActionTone)
//! - i18n — 跨 crate 文案抽象 (Locale, LOCALE, t, t_in, plural)

pub mod action_buttons;
pub mod auth_modal;
pub mod bubble;
pub mod card;
pub mod components;
pub mod dialog;
pub mod feedback;
pub mod i18n;
pub mod form;
pub mod icons;
pub mod scroll_spy;
pub mod segmented;
pub mod session;

pub use action_buttons::{ActionButtonGroup, ActionSpec, ActionTone};
pub use auth_modal::{AuthModal, UserBadge};
pub use bubble::{MessageBubble, SwipePicker};
pub use card::{ChoiceCard, ChoiceOption, StatusCard};
pub use components::admin_card::{
    AdminCard, AliasCard, ChannelCard, DotTabBar, GroupCard, RedemptionCard, UserCard,
};
pub use components::layout::{AppShell, SectionRail, StatusBar, StatusItem, TopNavBar};
pub use components::showcase::{PosterCard, RadarFlipCard, StatTabsCard};
pub use components::stat_card::{StatCard, StatSize};
pub use i18n::{LOCALE, Locale, plural, t, t_in};
pub use form::{CodeField, Field, FormField, PasswordField, SliderField, SubmitButton};
pub use icons::{IconChartBar, IconLogOut, IconUser};
pub use scroll_spy::ScrollSpyNav;
pub use segmented::SegmentedCapsule;
pub use session::{
    api_login, api_register, clear_cached_session, copy_text_to_clipboard, get_cached_token,
    get_cached_user, get_storage_item, refresh_access_token, remove_storage_item,
    set_cached_session, set_storage_item, set_storage_scoped, token_is_persistent,
};
