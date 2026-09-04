//! `adapt` —— contract record → gate 视图的扩展 trait
//!
//! 把 contract 的字段语义（`status: u8`、`meta.key: String`、`expires_at: DateTime<Utc>`）
//! 转成 gate 直接要的布尔 / 数值 / 字符串。gate 不应该再关心 schema 细节。

use chrono::{DateTime, Utc};
use contract::records::{TokenRecord, UserRecord};

/// `UserRecord` → gate 视角的字段
pub trait UserView {
    /// 用户是否启用（status == 1）
    fn is_enabled(&self) -> bool;
    /// auth_version 单调性 —— 用于"密码/2FA 变更后旧 token 立即失效"。
    /// 这里取 `meta.schema_version` 作为粗略代理；真实 auth_version 由 sync 层维护。
    fn auth_version(&self) -> u64;
    /// 用户组字符串
    fn group(&self) -> &str;
}

impl UserView for UserRecord {
    fn is_enabled(&self) -> bool {
        self.status == 1
    }
    fn auth_version(&self) -> u64 {
        self.meta.schema_version as u64
    }
    fn group(&self) -> &str {
        &self.group
    }
}

/// `TokenRecord` → gate 视角的字段
pub trait TokenView {
    fn is_enabled(&self) -> bool;
    fn auth_version(&self) -> u64;
    fn group(&self) -> Option<&str>;
    /// unix 时间戳；None = 永不过期。
    fn expires_at_unix(&self) -> Option<i64>;
    fn user_key(&self) -> &str;
    /// token 自身的剩余额度（unlimited_quota 时为 i64::MAX）。
    fn quota_remaining(&self) -> i64;
}

impl TokenView for TokenRecord {
    fn is_enabled(&self) -> bool {
        self.status == 1
    }
    fn auth_version(&self) -> u64 {
        self.meta.schema_version as u64
    }
    fn group(&self) -> Option<&str> {
        self.group.as_deref()
    }
    fn expires_at_unix(&self) -> Option<i64> {
        self.expires_at.as_ref().map(|t| t.timestamp())
    }
    fn user_key(&self) -> &str {
        &self.user_key
    }
    fn quota_remaining(&self) -> i64 {
        if self.unlimited_quota {
            i64::MAX
        } else {
            self.quota - self.used_quota
        }
    }
}

/// 当前 unix 时间戳。
pub fn now_unix() -> i64 {
    Utc::now().timestamp()
}

/// `DateTime<Utc>` → unix（保持公开，方便测试 / 边界调用）。
pub fn to_unix(t: &DateTime<Utc>) -> i64 {
    t.timestamp()
}
