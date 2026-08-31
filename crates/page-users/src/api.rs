//! 用户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 现在是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! `panel.rs` 与 `data.rs` 的格式化助手都不用动。

pub use mock::users::{THIS_MONTH, User};

pub fn fetch_users() -> &'static [User] {
    mock::users::USERS
}

pub fn fetch_user(id: u32) -> Option<&'static User> {
    fetch_users().iter().find(|u| u.id == id)
}

/// 分组筛选项:(标签, group 值);空值表示不过滤
pub fn fetch_groups() -> &'static [(&'static str, &'static str)] {
    mock::users::GROUPS
}

/// 状态筛选项:(标签, status 值);0 表示不过滤
pub fn fetch_statuses() -> &'static [(&'static str, u8)] {
    mock::users::STATUSES
}

/// 角色筛选项:(标签, role 值);0 表示不过滤
pub fn fetch_roles() -> &'static [(&'static str, u16)] {
    mock::users::ROLES
}
