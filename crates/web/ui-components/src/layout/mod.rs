//! Console layout primitives（Linear 风格三段式 shell）。
//!
//! - [`app_shell::AppShell`]      — 骨架：左 rail + 顶导航 + 滚动内容 + 底状态栏
//! - [`section_rail::SectionRail`] — 左侧 icon-only rail（总览/账户/管理 + account footer）
//! - [`top_nav::TopNavBar`]       — 顶部悬浮 page-tab 胶囊
//! - [`status_bar::StatusBar`]    — 底部悬浮状态栏（占位命名 + 主题 + 账户）
//!
//! 组件间解耦：AppShell 只收 slot（Element），业务接线（signal/hash/登录态）
//! 全部留在调用方页面（apps/admin-web HomePage），保证布局原语可被 tavern-web 复用。

mod app_shell;
mod avatar_menu;
mod section_rail;
mod status_bar;
mod top_nav;

pub use app_shell::AppShell;
pub use avatar_menu::AvatarMenu;
pub use section_rail::SectionRail;
pub use status_bar::{StatusBar, StatusItem};
pub use top_nav::TopNavBar;
