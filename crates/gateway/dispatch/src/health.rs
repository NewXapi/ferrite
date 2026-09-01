//! 本地健康表 — EWMA 延迟 + 失败连击 + 熔断冷却。
//!
//! 参考: new-api catalog/track_health.go (EWMA/失败连击/冷却/慢启动),
//! wildtoken auto-weight (惩罚/恢复/静态权重兜底)。
//! 全部内存态 (DashMap), 不同步 — center 的 health_observations 是趋势原料,
//! 不参与热路径决策。

/// 可重试错误分类 — 决定状态机是否回退换下一个候选。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// 429/5xx/超时/连接失败 → 换候选重试。
    Retryable,
    /// 400/401/403/404/422 → 客户端或凭据问题, 换渠道大概率无效。
    /// TODO(#310): 401 (key 失效) 是否熔断该 key? 暂按不重试。
    Fatal,
}

/// 单个 RouteUnit 的健康记录。
#[derive(Debug, Clone, Copy, Default)]
pub struct HealthState {
    /// EWMA 延迟 (ms)。α = 0.3 起步 (TODO(#311): 配置化)。
    pub ewma_latency_ms: f64,
    /// 连续失败计数; 成功清零。
    pub failure_streak: u32,
    /// 熔断截止时刻 (unix ms); 此前不参与选择。0 = 未熔断。
    pub cooldown_until_ms: u64,
    /// 慢启动: 熔断恢复后的初始权重折扣 (0.0-1.0, 逐步恢复)。
    pub slow_start: f64,
}

/// 健康表 trait — 按候选 key 读写健康状态。
pub trait HealthTable: Send + Sync {
    fn get(&self, unit_key: &str) -> HealthState;
    /// report() 的落地: 成功 → EWMA 更新 + streak 清零 + slow_start 恢复;
    /// Retryable 失败 → streak +1, 达阈值 → cooldown_until = now + 冷却窗口。
    fn record(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32);
    /// 门控判定: 候选是否可参与本轮选择。
    fn is_selectable(&self, unit_key: &str, now_ms: u64) -> bool;
}

/// 默认参数 (TODO(#311) 配置化前): 冷却 30s / 阈值 5 连击 / EWMA α=0.3。
pub mod defaults {
    pub const COOLDOWN_MS: u64 = 30_000;
    pub const STREAK_THRESHOLD: u32 = 5;
    pub const EWMA_ALPHA: f64 = 0.3;
}
