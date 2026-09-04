//! 幂等护栏 — 所有资金写操作的通用前置。
//!
//! 语义 (对齐 sub2api idempotency.go + repository/idempotency_repo.go):
//! ```text
//! begin(scope, key, fingerprint):
//!   INSERT processing ... ON CONFLICT (scope, key_hash) DO NOTHING
//!   - 插入成功       → Proceed (本调用获得执行权)
//!   - 已存在同名同指纹:
//!       succeeded    → Replay(缓存响应)   ← 幂等回放
//!       processing   → InFlight(冲突, 附 Retry-After)
//!       failed_retryable 且租约过期 → CAS 重领 → Proceed
//!   - 同名不同指纹   → FingerprintConflict (客户端 bug, 4xx)
//! complete(scope, key, response): processing → succeeded + 存响应
//! fail(scope, key): processing → failed_retryable
//! ```
//!
//! TODO(#609): 状态机 SQL + 过期清理挂 ops::jobs (对齐 sub2api 清理任务)。

pub enum Begin {
    /// 获得执行权。
    Proceed,
    /// 已成功过 — 直接回放存储的响应。
    Replay(serde_json::Value),
    /// 别的实例正在处理 — 冲突 (425/409 + Retry-After)。
    InFlight,
    /// 同 key 不同请求体 — 客户端 bug。
    FingerprintConflict,
}

pub async fn begin(
    _store: &(impl store::UsageStore + Sync),
    _scope: &str,
    _key: &str,
    _fingerprint: &str,
) -> Result<Begin, store::StoreError> {
    todo!("TODO(#609)")
}

pub async fn complete(
    _store: &(impl store::UsageStore + Sync),
    _scope: &str,
    _key: &str,
    _response: serde_json::Value,
) -> Result<(), store::StoreError> {
    todo!("TODO(#609)")
}
