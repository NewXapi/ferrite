//! 用户页的数据来源。面板只从这里取数,不认识数据是怎么来的。
//!
//! 真实数据经 `list_users_api` / `manage_user_api` / `create_user_api` /
//! `list_groups_api` 走 `client::ApiClient` 取;状态 / 角色筛选项(纯 UI
//! 常量,后端无对应枚举端点)值内联在本文件。分组列表走真实 `GET /api/group`,
//! 静态兜底仅「全部」(写死的 default/vip/svip/internal 与后端实际分组不符)。

use client::{ApiClient, ApiResult};
use contract::api::admin::{AdminUserPage, GroupDto};

pub use contract::api::admin::ManageUserRequest;
use contract::api::user::UserDto;

/// 分组筛选项的静态兜底:仅「全部」。真实分组由 `list_groups_api`
/// 异步注入(2026-09 起不再依赖写死常量)。
pub fn fetch_groups() -> &'static [(&'static str, &'static str)] {
    &[("全部", "")]
}

/// 状态筛选项:(标签, status 值);0 表示不过滤
pub const STATUSES: &[(&str, u8)] = &[("全部", 0), ("启用", 1), ("禁用", 2)];

/// 角色筛选项:(标签, role 值);0 表示不过滤
pub const ROLES: &[(&str, u16)] = &[("全部", 0), ("普通用户", 1), ("管理员", 10), ("Root", 100)];

/// 状态筛选项:(标签, status 值);0 表示不过滤
pub fn fetch_statuses() -> &'static [(&'static str, u8)] {
    STATUSES
}

/// 角色筛选项:(标签, role 值);0 表示不过滤
pub fn fetch_roles() -> &'static [(&'static str, u16)] {
    ROLES
}

/// 返回运行时当月前缀 ("YYYY-MM"),用 UTC。
/// 原 `THIS_MONTH` 写死 2026-08,9 月时仍显示 8 月数据,现已改运行时。
pub fn current_month_prefix() -> String {
    chrono::Utc::now().format("%Y-%m").to_string()
}

/// 分组选择项:(展示标签, 分组名),来自真实 `GET /api/group`(items 信封,
/// 与 admin-page-admin `list_groups_api` 同型)。
///
/// 展示标签取 `remark`(分组管理页的主标题口径),remark 为空才回落到
/// `name` —— 否则弹窗 chips 显示裸 name、分组页显示 remark,两边对不上。
/// 第二元永远是分组名,提交/匹配都用它。
pub async fn list_groups_api(client: &ApiClient) -> ApiResult<Vec<(String, String)>> {
    #[derive(serde::Deserialize, Default)]
    struct GroupItems {
        #[serde(default)]
        items: Vec<GroupDto>,
    }
    let r: GroupItems = client.get("/api/group").await?;
    Ok(r.items
        .into_iter()
        .map(|g| {
            let label = if g.remark.trim().is_empty() {
                g.name.clone()
            } else {
                g.remark.clone()
            };
            (label, g.name)
        })
        .collect())
}

/// 真实调用: GET /api/user/users?search=&page=&size=
///
/// 路径含两段 `user(s)`：auth 子路由自身是 `/users`，被 admin-router
/// 整体 nest 到 `/api/user` 之下，因此完整路径是 `/api/user/users`。
pub async fn list_users_api(
    client: &ApiClient,
    search: Option<&str>,
    page: Option<i64>,
    size: Option<i64>,
) -> ApiResult<AdminUserPage> {
    let mut query = Vec::new();
    if let Some(s) = search
        && !s.is_empty()
    {
        query.push(format!("search={s}"));
    }
    if let Some(p) = page {
        query.push(format!("page={p}"));
    }
    if let Some(sz) = size {
        query.push(format!("size={sz}"));
    }
    let path = if query.is_empty() {
        "/api/user/users".to_string()
    } else {
        format!("/api/user/users?{}", query.join("&"))
    };
    client.get(&path).await
}

/// 真实调用: POST /api/user/users/manage
///
/// action ∈ enable | disable | set_role | adjust_quota | set_group |
/// reset_password；`set_group` 的 value 为分组名（后端落 `auth_users.group_id`）。
pub async fn manage_user_api(
    client: &ApiClient,
    req: &ManageUserRequest,
) -> ApiResult<serde_json::Value> {
    client.post("/api/user/users/manage", req).await
}

/// 真实调用: POST /api/user/users — admin 创建用户（指定 role/quota/分组）。
///
/// 响应为 `UserView`（camelCase）：新建的 key 在响应体里直接可取。
pub async fn create_user_api(
    client: &ApiClient,
    req: &CreateUserRequest,
) -> ApiResult<contract::api::user::UserDto> {
    client.post("/api/user/users", req).await
}

/// POST /api/user/users 请求体（camelCase wire，对齐后端 `CreateUserRequest`）。
///
/// `role` 是 wire 位值 1 | 10 | 100；`quota` 是内部额度单位
/// （`500_000 = $1`，展示换算见 `data::fmt_cny`）；`groups` 是生效分组
/// 数组（`groups[1]` 为生效分组，缺省 `["default"]`）。
#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    pub role: u16,
    pub quota: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<String>,
}

/// 真实调用: GET /api/user/self
pub async fn get_user_self_api(client: &ApiClient) -> ApiResult<UserDto> {
    client.get("/api/user/self").await
}
