//! # dispatch — 调度状态机 (热路径第 2 步)
//!
//! 从 catalog 快照中选出本次请求的 [`Candidate`]。选择流程四阶段:
//!
//! ```text
//! ① 候选过滤   group + public_model 匹配的 RouteUnit 全集
//! ② 健康门控   剔除: 熔断冷却中 / 并发已满 / status != 启用
//! ③ 权重打分   priority 分层 → 层内按 weight 加权随机 (EWMA 延迟微调)
//! ④ 失败回退   转发失败且错误可重试 → 从剩余候选重选 (退避)
//! ```
//!
//! 健康数据全部是**本节点内存观测** (DashMap), 不跨节点同步 — 这是"没有
//! 全局证据不做全局 lease"原则的落地: 每 Pod 独立熔断, 中心只做趋势汇总。

use contract::records::RouteUnitRecord;

/// dispatch 的最终产出: 一个可转发的具体目标。
#[derive(Debug, Clone)]
pub struct Candidate {
    pub unit: RouteUnitRecord,
    /// 解析后的上游凭据 (来自 channel keys[key_index])。
    pub secret: String,
    pub base_url: String,
    /// 上游真名 (unit.upstream_model)。
    pub upstream_model: String,
}

/// 可重试错误分类 — 决定状态机是否回退换下一个候选。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// 429/5xx/超时/连接失败 → 换候选重试。
    Retryable,
    /// 400/401/403/404/422 → 客户端或凭据问题, 换渠道大概率无效,
    /// 但 401 (key 失效) 例外: TODO(#310) 401 是否熔断该 key? 暂按不重试。
    Fatal,
}

/// 调度器 trait。实现持有 catalog 快照 (ArcSwap) 与健康表 (DashMap)。
pub trait Dispatch: Send + Sync {
    /// 为一次请求选出候选。
    ///
    /// `exclude`: 已失败的候选 key 集合 (failover 回退时排除它们)。
    /// 全部候选被排除/耗尽 → [`DispatchError::NoCandidate`]。
    fn select(
        &self,
        group: &str,
        public_model: &str,
        exclude: &[String],
    ) -> Result<Candidate, DispatchError>;

    /// 转发结束后回报结果, 驱动健康状态演化 (EWMA 更新 / 熔断开关)。
    /// TODO(#311): EWMA 衰减系数与熔断阈值配置化 (config.toml), 先硬编码合理默认。
    fn report(
        &self,
        unit_key: &str,
        outcome: Result<u16, FailureClass>,
        latency_ms: u32,
    );
}

#[derive(Debug, thiserror::Error)]
pub enum DispatchError {
    /// 该分组下没有此模型的路由 (或全部被健康门控剔除)。
    #[error("no candidate for {group}/{model}")]
    NoCandidate { group: String, model: String },
    /// catalog 快照尚未就绪 (启动初期 sync 未完成首次拉取)。
    /// TODO(#312): 启动期策略 — 快照未就绪时 fail-closed (拒绝) 还是阻塞等待?
    /// 倾向 fail-closed + 短暂 503, 与"安全配置 fail-closed"原则一致。
    #[error("catalog snapshot not ready")]
    SnapshotNotReady,
}
