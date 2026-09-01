//! # admission — 网关准入闸 (热路径第 1 步)
//!
//! 职责: 在 `/v1/*` 请求进入 dispatch 之前完成**全部本地检查**, 零跨网络调用:
//!
//! 1. **鉴权**: sk-xxx → SHA-256 → 在本地 token 快照 (ArcSwap) 中查找;
//! 2. **状态门**: token/user/channel 未禁用、未过期;
//! 3. **配额闸**: 余额足够覆盖预估成本 (预扣由 metering 执行, 这里只判断);
//! 4. **并发闸**: 该渠道/该 key 的本地 Semaphore 尚有空位。
//!
//! 数据来源: `service/sync` 拉取的 identity/catalog 快照 → 由 apps/gateway
//! 组装成 [`TokenSnapshot`] 后 `ArcSwap::store`, 本模块只读。
//!
//! ```text
//! 请求 → admission.authenticate → (metering.prehold) → dispatch.select
//!              ↑本地内存                ↑本地内存            ↑本地内存
//! ```

use contract::records::{TokenRecord, UserRecord};

/// admission 判定的最终产出: 放行 + 关联实体, 供后续模块免查快照直接使用。
#[derive(Debug, Clone)]
pub struct Admitted {
    pub token: TokenRecord,
    pub user: UserRecord,
    /// token 所属分组 (计费倍率/可见模型以此为准)。
    pub group: String,
    /// 预扣成功的凭据 (metering 层发放), 转发结束后必须 settle 或 release。
    pub hold_id: u64,
}

/// 拒绝原因 — 会映射到 OpenAI 风格的 4xx 错误体 (由 apps/gateway 完成)。
#[derive(Debug, Clone, thiserror::Error)]
pub enum Rejection {
    /// 无效 key (快照中不存在此 hash)。
    #[error("invalid api key")]
    InvalidKey,
    /// key 存在但被禁用/过期。携带人类可读原因。
    #[error("token unavailable: {0}")]
    TokenUnavailable(String),
    /// 余额不足。预估成本 (内部单位) 放在字段里供错误信息展示。
    #[error("insufficient quota: need {estimated}, have {available}")]
    InsufficientQuota { estimated: i64, available: i64 },
    /// 该 key 的分组无权访问请求的模型。
    #[error("model {model} not allowed for group {group}")]
    ModelForbidden { model: String, group: String },
    /// 并发已满 — 返回 429, 客户端应退避重试。
    #[error("channel busy")]
    Busy,
}

/// 准入闸 trait。apps/gateway 里用本地快照实现; 测试用内存假实现。
///
/// TODO(#300): trait 拆分粒度 — authenticate 与 acquire 是否拆开? 当前合并,
/// 因为热路径希望一次调用完成全部检查, 减少 ArcSwap 读放大。
pub trait Admit: Send + Sync {
    /// 完整准入流程。`raw_key` 是请求头里的明文 sk-key。
    ///
    /// # 错误
    /// 任何 [`Rejection`] 都应直接终止请求, 不重试 (4xx 语义)。
    fn admit(
        &self,
        raw_key: &str,
        // 请求的公开模型别名, 用于分组白名单校验。
        model: &str,
    ) -> impl Future<Output = Result<Admitted, Rejection>> + Send;

    /// 请求结束 (成功或失败) 后归还并发槽位。幂等。
    fn release(&self, hold_id: u64) -> impl Future<Output = ()> + Send;
}
