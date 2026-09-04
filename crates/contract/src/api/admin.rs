//! 管理面端点 DTO — console admin 面板 (page-admin / page-users) 专用。
//!
//! 鉴权: role >= 10 (admin)。端点登记表见本文件尾部。
//! 参考: new-api /api/channel|user|group 管理路由 + mock::users 过滤枚举。

use crate::records::{ChannelRecord, GroupRecord, RouteUnitRecord, UserRecord};
use serde::{Deserialize, Serialize};

/// POST/PUT /api/channel — 渠道创建/更新 (admin)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminChannelUpsert {
    /// None = 创建。
    pub key: Option<String>,
    pub name: String,
    pub provider_type: String,
    pub base_url: String,
    /// 明文 keys (服务端加密入库); 响应永远不回明文。
    pub keys: Vec<String>,
    pub max_concurrency: u32,
    pub groups: Vec<String>,
    pub settings: serde_json::Value,
}

impl From<&ChannelRecord> for AdminChannelUpsert {
    fn from(r: &ChannelRecord) -> Self {
        Self {
            key: Some(r.meta.key.clone()),
            name: r.name.clone(),
            provider_type: r.provider_type.clone(),
            base_url: r.base_url.clone(),
            // 密文不出库 — 编辑时前端拿到掩码, 保存时留空 = 不变 (TODO(#209))
            keys: vec![],
            max_concurrency: r.max_concurrency,
            groups: r.groups.clone(),
            settings: r.settings.clone(),
        }
    }
}

/// POST/PUT /api/route-unit — 路由单元 (admin)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminRouteUnitUpsert {
    pub key: Option<String>,
    pub group: String,
    pub public_model: String,
    pub channel_key: String,
    pub key_index: u32,
    pub upstream_model: String,
    pub priority: i32,
    pub weight: u32,
}

impl From<&RouteUnitRecord> for AdminRouteUnitUpsert {
    fn from(r: &RouteUnitRecord) -> Self {
        Self {
            key: Some(r.meta.key.clone()),
            group: r.group.clone(),
            public_model: r.public_model.clone(),
            channel_key: r.channel_key.clone(),
            key_index: r.key_index,
            upstream_model: r.upstream_model.clone(),
            priority: r.priority,
            weight: r.weight,
        }
    }
}

/// GET /api/user → data: AdminUserPage (admin 用户列表, 对齐 mock::users)。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserPage {
    pub items: Vec<AdminUserDto>,
    pub total: u64,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDto {
    pub key: String,
    pub username: String,
    pub display_name: String,
    pub email: String,
    pub quota: i64,
    pub used_quota: i64,
    pub request_count: u64,
    pub group: String,
    pub status: u8,
    pub role: u16,
    pub created_at: String,
}

impl From<&UserRecord> for AdminUserDto {
    fn from(r: &UserRecord) -> Self {
        Self {
            key: r.meta.key.clone(),
            username: r.username.clone(),
            display_name: r.display_name.clone(),
            email: r.email.clone(),
            quota: r.quota,
            used_quota: r.used_quota,
            request_count: r.request_count,
            group: r.group.clone(),
            status: r.status,
            role: r.role,
            created_at: r.created_at.format("%Y-%m-%d").to_string(),
        }
    }
}

/// POST /api/user/manage — admin 管理用户 (enable/disable/set_role/adjust_quota/reset_password)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManageUserRequest {
    pub key: String,
    pub action: String,
    #[serde(default)]
    pub value: Option<String>,
}

/// 渠道视图 DTO — 对标 admin-api /api/channel 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelDto {
    pub key: String,
    pub name: String,
    pub channel_type: String,
    pub base_url: String,
    pub key_count: i64,
    #[serde(default)]
    pub keys: Option<Vec<String>>,
    pub models: serde_json::Value,
    pub group_name: String,
    pub priority: i32,
    pub weight: i32,
    pub status: i16,
    #[serde(default)]
    pub test_model: Option<String>,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// 渠道创建/更新请求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChannelUpsertRequest {
    pub name: String,
    pub channel_type: String,
    pub base_url: String,
    #[serde(default)]
    pub keys: Vec<String>,
    pub models: serde_json::Value,
    pub group_name: String,
    pub priority: i32,
    pub weight: i32,
    #[serde(default)]
    pub test_model: Option<String>,
    #[serde(default)]
    pub remark: String,
}

/// 用户组 DTO — 对标 admin-api /api/group 响应
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupDto {
    pub key: String,
    pub name: String,
    pub ratio: f64,
    pub model_whitelist: serde_json::Value,
    #[serde(default)]
    pub remark: String,
    pub status: i16,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

/// 用户组创建/更新请求
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupUpsertRequest {
    pub name: String,
    pub ratio: f64,
    pub model_whitelist: serde_json::Value,
    #[serde(default)]
    pub remark: String,
}

/// PUT /api/group — 分组编辑 (admin)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminGroupUpsert {
    pub id: String,
    pub display_name: String,
    pub rate_multiplier: f64,
    pub allowed_models: Vec<String>,
}

impl From<&GroupRecord> for AdminGroupUpsert {
    fn from(r: &GroupRecord) -> Self {
        Self {
            id: r.id.clone(),
            display_name: r.display_name.clone(),
            rate_multiplier: r.rate_multiplier,
            allowed_models: r.allowed_models.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// 管理面端点登记表 (console 实现对照; 鉴权 role >= 10)
// ---------------------------------------------------------------------------
//
// | 方法 | 路径                     | 请求体                | data 类型        |
// |------|--------------------------|-----------------------|------------------|
// | GET  | /api/channel/            | -                     | Vec<ChannelRecord> 投影 |
// | POST | /api/channel/            | AdminChannelUpsert    | ()               |
// | DEL  | /api/channel/{key}       | -                     | ()               |
// | GET  | /api/route-unit/         | -                     | Vec<AdminRouteUnitUpsert> |
// | POST | /api/route-unit/         | AdminRouteUnitUpsert  | ()               |
// | GET  | /api/user/               | ?page=&group=&status= | AdminUserPage    |
// | PUT  | /api/user/{key}/quota    | { delta: i64 }        | { balance }      |
// | GET  | /api/group/              | -                     | Vec<AdminGroupUpsert> |
// | PUT  | /api/group/              | AdminGroupUpsert      | ()               |
//
// TODO(#209): 渠道编辑时的 key 保留语义 — 前端 keys 留空 = 保持原 keys 不变。
