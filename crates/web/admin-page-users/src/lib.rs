//! Users page: 用户管理面板(纯 UI)。数据经 `api` 取用。
//! 字段与样式 token 对标 new-api features/users 与账户线(page-account)。
//!
//! crate 形状(spec 理念 1):`tab-page/` 每 tab 一个页面文件(编排层,薄 rsx),
//! `components/` 本 crate 独有组件;`format`/`shared` 为根级共享模块(非组件)。
//! 跨页复用组件在 `ui-components`,跨页 wire 在 `admin-client`。

pub mod api;
pub mod components;
pub mod format;
pub mod shared;
// Rust 模块名不带连字符:目录保持 `tab-page/`,此处一行 #[path] 声明
#[path = "tab-page/mod.rs"]
pub mod tab_page;

pub use tab_page::users::UsersPanel;
