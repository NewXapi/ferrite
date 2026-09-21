//! Users page: 用户管理面板(纯 UI)。数据经 `api` 取用。
//! 字段与样式 token 对标 new-api features/users 与账户线(page-account)。
//!
//! 一个 tab = 一个目录(目录名带 `tab-page-` 前缀,Rust 模块名不带连字符,
//! 故用 `#[path]` 声明);目录内文件不带前缀,见各 `mod.rs` 的拆分约定。

pub mod api;
pub mod data;

#[path = "tab-page-users/mod.rs"]
pub mod tab_page_users;

pub use tab_page_users::UsersPanel;
