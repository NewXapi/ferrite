//! # admission — 网关准入闸 (热路径第 1 步)
//!
//! 职责: 在 `/v1/*` 请求进入 dispatch 之前完成**全部本地检查**, 零跨网络调用。
//!
//! ## 模块地图 (四道闸, 一次调用全部过完)
//!
//! | 模块 | 闸门 | 失败码 |
//! |------|------|--------|
//! | [`auth`]          | ① key 哈希查找 + token/user 状态 | INVALID_API_KEY / TOKEN_EXPIRED |
//! | [`quota`]         | ② 余额 vs 预估成本 | INSUFFICIENT_QUOTA |
//! | [`concurrency`]   | ③ 渠道/单 key 本地并发槽 | BUSY |
//! | [`snapshot`]      | (数据源) identity 快照的持有与原子替换 | CATALOG_NOT_READY |
//!
//! 模型白名单校验在 ①② 之间: token 组 / 用户组的 allowed_models。
//! (V2: 子网/CIDR 闸 — one-api token.Subnet)
//!
//! ```text
//! 请求 → auth → 模型白名单 → quota → concurrency → (hold 交给 metering) → dispatch
//!          ↑ 全部查本地内存快照, 零跨网络调用
//! ```

pub mod auth;
pub mod concurrency;
pub mod error;
pub mod quota;
pub mod snapshot;

pub use error::Rejection;
pub use snapshot::TokenSnapshot;

use contract::records::{TokenRecord, UserRecord};

/// admission 判定的最终产出: 放行 + 关联实体, 供后续模块免查快照直接使用。
#[derive(Debug, Clone)]
pub struct Admitted {
    pub token: TokenRecord,
    pub user: UserRecord,
    /// 生效分组 = token.group.unwrap_or(user.group)。
    pub group: String,
    /// 并发槽凭据 (concurrency 模块发放), 请求结束必须 release (幂等)。
    pub hold_id: u64,
}

/// 准入闸 trait — apps/gateway 用本地快照实现; 测试用内存假实现。
///
/// TODO(#300): authenticate 与 acquire 合并为一次调用的取舍已定 (减少 ArcSwap
/// 读放大); 若未来配额检查需要远程数据, 再拆两段。
pub trait Admit: Send + Sync {
    /// 完整准入流程。`raw_key` = 请求头明文 sk-key; `model` = 公开模型别名。
    fn admit(
        &self,
        raw_key: &str,
        model: &str,
    ) -> impl Future<Output = Result<Admitted, Rejection>> + Send;

    /// 请求结束 (成功或失败) 后归还并发槽位。幂等。
    fn release(&self, hold_id: u64) -> impl Future<Output = ()> + Send;
}
