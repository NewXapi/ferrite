//! 本地健康表 — EWMA 延迟 + 失败连击 + 熔断冷却 + 慢启动。
//!
//! 参考实现:
//! - new-api `channel_model_health.go` (失败隔离/冷却阶梯) +
//!   `pkg/routestats/quality.go` (EWMA 延迟质量分: target/observed 归一,
//!   MinSamples 门槛前中性 1.0)。
//! - wildtoken `internal/proxy/health.go` (整数健康分 + 定时渐进恢复,
//!   恢复即慢启动; 分数为 0 的节点完全不参与选择)。
//!
//! 全部内存态 (Mutex<HashMap>), 不跨节点同步 — center 的 health_observations
//! 是趋势原料, 不参与热路径决策。冷却窗口内完全排除; 恢复期 slow_start 折扣
//! 权重, 随成功逐步回归 1.0 (wildtoken RecoveryIncrement 的按成功等价物)。
//!
//! 借鉴 wildtoken 的两条教训:
//! - 失败在冷却中继续到账时**延长**冷却窗口 (wildtoken 失败重置恢复计时) —
//!   持续故障的节点不会因为旧冷却到期就立刻满血回归;
//! - 恢复是渐进的, 不是"冷却结束即全量" — slow_start 从 0.5 起步逐次回升。

use std::collections::HashMap;
use std::sync::Mutex;

/// 可重试错误分类 — 决定状态机是否回退换下一个候选。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// 429/5xx/超时/连接失败 → 换候选重试。
    Retryable,
    /// 400/401/403/404/422 → 客户端或凭据问题, 换渠道大概率无效。
    /// 计入失败连击 (坏 key 不该继续被选中), 但重试循环直接停止不换渠道。
    Fatal,
}

/// 单个 RouteUnit 的健康记录。
#[derive(Debug, Clone, Copy)]
pub struct HealthState {
    /// EWMA 延迟 (ms), α = EWMA_ALPHA; 首样本直接采纳。
    pub ewma_latency_ms: f64,
    /// 连续失败计数; 成功清零, 熔断触发后归零。
    pub failure_streak: u32,
    /// 熔断截止时刻 (unix ms); 此前不参与选择。0 = 未熔断。
    pub cooldown_until_ms: u64,
    /// 权重折扣 (0.0-1.0), 双重角色:
    /// - 渐进惩罚: 失败一次 -FAILURE_PENALTY (wildtoken 每次失败 -20 分等价物);
    /// - 慢启动恢复: 熔断后从 SLOW_START_INITIAL 起步, 成功一次 +SLOW_START_STEP。
    ///
    /// Default = 1.0 (从未观测/从未熔断的单元全量参与, 与 wildtoken "无记录
    /// 即满血" 语义一致)。
    pub slow_start: f64,
    /// 观测样本数; 未达 MIN_SAMPLES 前延迟质量分保持中性 1.0。
    pub samples: u32,
    /// 最近一次失败分类 (ocr #0: 保留 Retryable/Fatal 区分供观测;
    /// new-api 亦按 local/upstream 分开计数)。成功清空。
    pub last_failure: Option<FailureClass>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            ewma_latency_ms: 0.0,
            failure_streak: 0,
            cooldown_until_ms: 0,
            slow_start: 1.0,
            samples: 0,
            last_failure: None,
        }
    }
}

/// 健康表 trait — 按候选 key (RouteUnitRecord::meta.key) 读写健康状态。
pub trait HealthTable: Send + Sync {
    /// 读取当前健康状态; 无记录 → Default (完全健康)。
    fn get(&self, unit_key: &str) -> HealthState;

    /// report() 的落地:
    /// - 成功 (Ok(status)) → EWMA 更新 + streak 清零 + slow_start 回升;
    /// - 失败 (Err(class)) → streak +1, 达阈值 → 进入冷却窗口;
    ///   冷却中再失败 → 延长冷却 (重复故障惩罚)。
    fn record(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32);

    /// 门控判定: 候选是否可参与本轮选择 (仅检查冷却窗口)。
    fn is_selectable(&self, unit_key: &str, now_ms: u64) -> bool;
}

/// 默认参数 (TODO(#311) 配置化前的固定值)。
pub mod defaults {
    /// 熔断冷却窗口: 30s。
    pub const COOLDOWN_MS: u64 = 30_000;
    /// 连续失败阈值: 5 次触发熔断。
    pub const STREAK_THRESHOLD: u32 = 5;
    /// EWMA 延迟平滑系数。
    pub const EWMA_ALPHA: f64 = 0.3;
    /// 熔断恢复后的初始权重折扣。
    pub const SLOW_START_INITIAL: f64 = 0.5;
    /// 每次成功恢复的步长。
    pub const SLOW_START_STEP: f64 = 0.25;
    /// 每次失败(未达熔断阈值)的权重折扣步长 — wildtoken 每次失败 -20 分等价物。
    pub const FAILURE_PENALTY: f64 = 0.2;
    /// 渐进惩罚的权重下限 (wildtoken score floor 0 的等价物: 第 4 次失败后 0.2,
    /// 第 5 次触发熔断冷却而非归零)。
    pub const FAILURE_FLOOR: f64 = 0.2;
    /// 延迟质量分生效前的最小样本数 (new-api MinSamples=5)。
    pub const MIN_SAMPLES: u32 = 5;
    /// 延迟基准 (new-api LatencyTargetMs=30000): 达到即 1.0, 更快 >1.0。
    pub const LATENCY_TARGET_MS: f64 = 30_000.0;
    /// 延迟质量分上下限 (new-api ComponentFloor/Ceil)。
    pub const LATENCY_FLOOR: f64 = 0.5;
    pub const LATENCY_CEIL: f64 = 1.5;
}

type StateMap = HashMap<String, HealthState>;

/// 纯内存健康表 — 每 Pod 独立实例, 不跨节点同步。
///
/// `now_ms` 为可注入时钟 (wildtoken AutoWeightManager.now 的等价物):
/// 默认取系统时间, 测试用 `with_clock` 注入固定时间。
pub struct MemoryHealthTable {
    states: Mutex<StateMap>,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl Default for MemoryHealthTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryHealthTable {
    pub fn new() -> Self {
        Self::with_clock(|| chrono::Utc::now().timestamp_millis().max(0) as u64)
    }

    /// 测试/确定性时钟注入。
    pub fn with_clock(now_ms: impl Fn() -> u64 + Send + Sync + 'static) -> Self {
        Self {
            states: Mutex::new(StateMap::new()),
            now_ms: Box::new(now_ms),
        }
    }
}

impl HealthTable for MemoryHealthTable {
    fn get(&self, unit_key: &str) -> HealthState {
        self.states
            .lock()
            .unwrap()
            .get(unit_key)
            .copied()
            .unwrap_or_default()
    }

    fn record(&self, unit_key: &str, outcome: Result<u16, FailureClass>, latency_ms: u32) {
        let now = (self.now_ms)();
        let mut states = self.states.lock().unwrap();
        let mut st = states.get(unit_key).copied().unwrap_or_default();
        match outcome {
            Ok(_) => {
                update_ewma(&mut st, latency_ms as f64);
                st.failure_streak = 0;
                st.slow_start = (st.slow_start + defaults::SLOW_START_STEP).min(1.0);
                st.samples = st.samples.saturating_add(1);
                st.last_failure = None;
            }
            Err(class) => {
                st.last_failure = Some(class);
                st.failure_streak += 1;
                let cooling = st.cooldown_until_ms > now;
                if cooling {
                    // 冷却期内失败: 只顺延窗口 (持续故障不因旧冷却到期而满血回归,
                    // wildtoken 同款)。单元已被排除, 不再重复惩罚权重。
                    st.cooldown_until_ms = st.cooldown_until_ms.max(now) + defaults::COOLDOWN_MS;
                } else {
                    let tripped = st.failure_streak >= defaults::STREAK_THRESHOLD;
                    if tripped {
                        st.failure_streak = 0;
                        st.slow_start = defaults::SLOW_START_INITIAL;
                        st.cooldown_until_ms =
                            st.cooldown_until_ms.max(now) + defaults::COOLDOWN_MS;
                    } else {
                        // 渐进惩罚 (wildtoken: 每次失败 -20 分, 权重立即缩水):
                        // 失败 1-4 次 slow_start 递减, 而不是第 5 次才突然掉线。
                        st.slow_start = (st.slow_start - defaults::FAILURE_PENALTY)
                            .max(defaults::FAILURE_FLOOR);
                    }
                }
            }
        }
        states.insert(unit_key.to_string(), st);
    }

    fn is_selectable(&self, unit_key: &str, now_ms: u64) -> bool {
        self.get(unit_key).cooldown_until_ms <= now_ms
    }
}

fn update_ewma(st: &mut HealthState, latency_ms: f64) {
    if st.samples == 0 {
        st.ewma_latency_ms = latency_ms;
    } else {
        st.ewma_latency_ms =
            defaults::EWMA_ALPHA * latency_ms + (1.0 - defaults::EWMA_ALPHA) * st.ewma_latency_ms;
    }
}

/// 延迟质量分: target/observed 归一, 夹在 [FLOOR, CEIL]。
/// 样本不足或缺少观测 → 中性 1.0 (new-api MinSamples 门槛语义)。
pub fn latency_quality(st: &HealthState) -> f64 {
    if st.samples < defaults::MIN_SAMPLES || st.ewma_latency_ms <= 0.0 {
        return 1.0;
    }
    let q = defaults::LATENCY_TARGET_MS / st.ewma_latency_ms;
    q.clamp(defaults::LATENCY_FLOOR, defaults::LATENCY_CEIL)
}

/// 权重乘数 — selector 的最终打分 (new-api routingBaseWeight 语义: +1 保证
/// weight=0 仍以最低份额参与; wildtoken "恢复渐进" 语义: slow_start 折扣)。
/// 冷却中 → 0 (完全排除)。
pub fn routing_weight(weight: u32, st: &HealthState, now_ms: u64) -> f64 {
    if st.cooldown_until_ms > now_ms {
        return 0.0;
    }
    (f64::from(weight) + 1.0) * st.slow_start * latency_quality(st)
}
