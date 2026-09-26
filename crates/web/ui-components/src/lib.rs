//! # ui-components — 全栈通用的 Web 组件库
//!
//! 按组件类型聚合：一个目录一个类型族（目录内按组件一文件拆分）；
//! 单一组件直接平铺为 `src/<类型>.rs`，不设单文件目录。
//! - button/ — 按钮族：Button、IconButton、SubmitButton、ActionButton(Group)、GhostButton、CloseButton
//! - card/ — 卡片族：shadcn Card 基元 + StatusCard、ChoiceCard、StatCard
//! - form/ — 表单基元：Field、FormField、CodeField、SliderField、PasswordField
//! - feedback/ — 反馈：兼容包装 Avatar（36px 默认档）、EmptyState、Loading
//! - auth/ — 认证：AuthModal、UserBadge
//! - bubble/ — 对话：MessageBubble、SwipePicker
//! - admin_card/ — 管理区卡牌族（AdminCard、实体卡、Pager、EditableRow）
//! - layout/ — 布局原语（AppShell、SectionRail、TopNavBar、StatusBar、AvatarMenu）
//! - showcase/ — 展示卡牌（PosterCard、RadarFlipCard、StatTabsCard）
//! - icons/ — 单色 stroke 图标集（lucide 风格，一图标一文件）
//! - dialog — 确认弹窗 (Dialog)；panel — 选择器面板外壳与模态样式常量 (FieldPanel, MODAL_*)
//! - nav — 滚动监听导航 (ScrollSpyNav)；segmented — 分段胶囊选择器 (SegmentedCapsule)
//! - sheet / sidebar / dropdown_menu / toast / rank_board / badge / avatar / input / skeleton / switch — 单文件类型组件
//! - components/ — rust-ui registry 过渡层（rui_*，待替换后整目录删除）
//! - session — 会话凭证管理与登录注册客户端
//! - i18n — 跨 crate 文案抽象 (Locale, LOCALE, t, t_in, plural)
//! - styles / wheel_tab — 样式 token 与滚轮 tab 切换

pub mod admin_card;
pub mod auth;
pub mod avatar;
pub mod badge;
pub mod bubble;
pub mod button;
pub mod card;
pub mod components;
pub mod dialog;
pub mod dropdown_menu;
pub mod feedback;
pub mod form;
pub mod i18n;
pub mod icons;
pub mod input;
pub mod layout;
pub mod nav;
pub mod panel;
pub mod rank_board;
pub mod segmented;
pub mod select;
pub mod session;
pub mod sheet;
pub mod showcase;
pub mod sidebar;
pub mod skeleton;
pub mod styles;
pub mod switch;
pub mod toast;
pub mod wheel_tab;

pub use admin_card::{
    AdminCard, AliasCard, AliasEditField, ChannelCard, ChannelEditField, DotTabBar, UserCard,
};
pub use admin_card::{
    AdminSection, CARD_SHELL_CLASS, CardGrid, CardShell, DangerBlock, PlaceholderBlock,
    SectionHeader,
};
pub use admin_card::{CARD_PAGE_SIZE, Pager, page_count, page_slice};
pub use admin_card::{DangerActionRow, EditableRow};
pub use admin_card::{PriceMode, PriceModeToggle};
pub use auth::{AuthModal, UserBadge};
pub use bubble::{MessageBubble, SwipePicker};
pub use button::{ActionButton, ActionButtonGroup, ActionSpec, ActionTone};
pub use button::{Button, CloseButton, GhostButton, IconButton, SubmitButton};
pub use card::{ChoiceCard, ChoiceOption, StatCard, StatSize, StatusCard};
pub use dialog::Dialog;
pub use feedback::{Avatar, EmptyState, Loading};
pub use form::{CodeField, Field, FormField, PasswordField, SliderField};
pub use i18n::{LOCALE, Locale, plural, t, t_in};
pub use icons::{IconChartBar, IconLogOut, IconUser};
pub use layout::{AppShell, SectionRail, StatusBar, StatusItem, TopNavBar};
pub use nav::ScrollSpyNav;
pub use panel::{CLOSE_BTN, FIELD_PANEL, FieldPanel, MODAL_BACKDROP, MODAL_CARD, MODAL_HEADER};
pub use segmented::SegmentedCapsule;
pub use session::{
    api_login, api_register, clear_cached_session, copy_text_to_clipboard, get_cached_token,
    get_cached_user, get_storage_item, refresh_access_token, remove_storage_item,
    set_cached_session, set_storage_item, set_storage_scoped, token_is_persistent,
};
pub use showcase::{PosterCard, RadarFlipCard, StatTabsCard};
pub use styles::*;
pub use wheel_tab::{cycle_index, on_tab_wheel};
