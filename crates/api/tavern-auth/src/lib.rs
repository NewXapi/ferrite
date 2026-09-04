//! tavern-auth — 请求身份到用户目录的唯一入口
//!
//! 存在的意义是留一个唯一身份入口，让 characters / chats / settings
//! 不各自硬编码 `default-user`。以后加账号只改这里。

use tavern_storage::{DataRoot, UserDirs};

/// MVP 单用户 handle。
pub const DEFAULT_USER: &str = "default-user";

/// 请求身份。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub handle: String,
}

impl Identity {
    pub fn default_user() -> Self {
        Self { handle: DEFAULT_USER.to_string() }
    }

    /// 该身份的用户目录。
    pub fn dirs(&self, root: &DataRoot) -> UserDirs {
        root.user(&self.handle)
    }
}

/// 从请求头解析身份。MVP 恒为默认用户。
///
/// 要实现：多用户会话解析。
pub fn resolve(_headers: &http::HeaderMap) -> Identity {
    Identity::default_user()
}
