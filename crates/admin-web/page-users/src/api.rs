//! 用户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 现在是 mock 直连(同步返回 `mock` crate 的静态数据);接上真实后端时
//! 只改本文件 —— 换成 `crates/shared/client` 的请求,必要时把签名改成 async,
//! `panel.rs` 与 `data.rs` 的格式化助手都不用动。

pub use mock::users::User;

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

/// 返回运行时当月前缀 ("YYYY-MM"),用 UTC。
/// 原 `THIS_MONTH` 写死 2026-08,9 月时仍显示 8 月数据,现已改运行时。
pub fn current_month_prefix() -> String {
    chrono::Utc::now().format("%Y-%m").to_string()
}

use client::{ApiClient, ApiResult};
use contract::api::admin::{AdminUserPage, ManageUserRequest};
use contract::api::user::UserDto;

/// 真实调用: GET /api/user?search=&page=&size=
pub async fn list_users_api(
    client: &ApiClient,
    search: Option<&str>,
    page: Option<i64>,
    size: Option<i64>,
) -> ApiResult<AdminUserPage> {
    let mut query = Vec::new();
    if let Some(s) = search {
        if !s.is_empty() {
            query.push(format!("search={s}"));
        }
    }
    if let Some(p) = page {
        query.push(format!("page={p}"));
    }
    if let Some(sz) = size {
        query.push(format!("size={sz}"));
    }
    let path = if query.is_empty() {
        "/api/user".to_string()
    } else {
        format!("/api/user?{}", query.join("&"))
    };
    client.get(&path).await
}

/// 真实调用: POST /api/user/manage
pub async fn manage_user_api(
    client: &ApiClient,
    req: &ManageUserRequest,
) -> ApiResult<serde_json::Value> {
    client.post("/api/user/manage", req).await
}

/// 真实调用: GET /api/user/self
pub async fn get_user_self_api(client: &ApiClient) -> ApiResult<UserDto> {
    client.get("/api/user/self").await
}
