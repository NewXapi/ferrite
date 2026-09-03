//! 存储错误统一形状 — 不暴露底层 SQL/KV 细节。

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("record not found: {0}")]
    NotFound(String),

    /// 违反唯一约束 / 域不匹配 / 业务校验失败。
    #[error("conflict: {0}")]
    Conflict(String),

    /// 底层失败 (连接/磁盘/编码), 已含上下文但不含敏感串。
    #[error("backend failure: {0}")]
    Backend(String),
}
