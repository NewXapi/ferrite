//! 本地健康表 — phase0 `track_health.go` 模型的完整移植。
//!
//! 参考: new-api phase0 `internal/catalog/track_health.go` +
//! `bridge_channel_health.go` (outcome 分类 / EWMA 连续分 / 递增冷却 /
//! slow-start ramp / max-ejection)。
//!
//! 模型要点 (与 phase0 逐项对齐):
//! - **outcome 五分类**: Success / Fatal / Throttled(429) / Neutral(其它 4xx) /
//!   UnauthorizedRun(401/403 连续升级, 3 次 → Fatal);
//! - **EWMA 连续分**: `score = α×obs + (1-α)×score`, obs ∈ {1.0, 0.0, 0.7(429)},
//!   MinRequests 前不更新, MinScore 下限;
//! - **递增冷却**: 触发阈值 CooldownThreshold; 5xx/传输层 streak 时长按
//!   `base + (max-base)×(1-α^streak)` 滑向 max, 每次激活 streak+1;
//!   P1-A 分档: 401/403-run 升级的 Fatal → max 档, 429 → base 短冷却;
//! - **slow-start ramp**: 冷却结束进入 ramp (RampPending), 权重按
//!   `request_count/min_requests` 渐进, 真实失败立即 RampExited;
//! - **max-ejection**: 同层冷却渠道占比超 CooldownMaxEjectionPercent 时,
//!   超出的冷却渠道以降权方式保留 (bypassCooldown) 而不是全弹射。
//!
//! 全部内存态 (Mutex<HashMap>), 不跨节点同步 — center 的 health_observations
//! 是趋势原料, 不参与热路径决策。

use std::collections::HashMap;
use std::sync::Mutex;

/// 可重试错误分类 — 决定状态机是否回退换下一个候选。
/// 健康分类独立于重试决策: 健康看上游状态码, 重试看客户端可恢复性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// 传输层失败 (超时/连接) → 换候选重试。
    Retryable,
    /// 客户端或凭据问题 (4xx) → 不重试。
    Fatal,
}

/// 健康 outcome — phase0 `ChannelOutcome` 枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelOutcome {
    /// 2xx — 全信任, EWMA obs = 1.0。
    Success,
    /// 5xx / 传输层 / 坏 body — 记 streak, obs = 0.0, 触发冷却。
    Fatal,
    /// 429 — 渠道健康但被限流, 轻度降权, obs = 0.7。
    Throttled,
    /// 其它 4xx (400/404/422/孤立 401/403) — 渠道无责, 不改分不记 streak。
    Neutral,
}

impl ChannelOutcome {
    /// 是否影响 EWMA 分数 (Success/Fatal/Throttled; Neutral 不参与)。
    pub fn affects_health(self) -> bool {
        !matches!(self, ChannelOutcome::Neutral)
    }
}

/// 单个 RouteUnit 的健康记录 — phase0 `ChannelHealthState`。
#[derive(Debug, Clone, Copy)]
pub struct HealthState {
    /// EWMA 连续健康分, 范围 [MinScore, 1.0]; 无历史 = 1.0。
    pub ewma_score: f64,
    /// 已观测请求数; 未达 MinRequests 前 EWMA 不更新 (信任新渠道)。
    pub request_count: u32,
    /// 连续 401/403 (凭据类) 次数; 达 UnauthorizedEscalationThreshold → Fatal。
    pub unauthorized_run: u32,
    /// 真实失败后退出 slow-start ramp (不再渐进)。
    pub ramp_exited: bool,
    /// 冷却刚结束, 首次选择从 ramp 地板起步。
    pub ramp_pending: bool,
    /// 连续 fatal/throttled 次数; 达 CooldownThreshold → 冷却。
    pub failure_streak: u32,
    /// 连续冷却激活次数; 决定冷却时长 (递增)。
    pub cooldown_streak: u32,
    /// 冷却截止时刻 (unix ms); 0 = 未在冷却。
    pub cooldown_until_ms: u64,
    /// 退火起点 (unix ms): 最近一次冷却结束时刻; 0 = 无空闲退火锚点。
    /// 空闲期间 cooldown_streak 以此为起点按 base 窗口衰减 (见 anneal_streak)。
    pub anneal_since_ms: u64,
    /// 最近一次触发冷却时的 outcome（P1-A 差异化冷却依据：401-run 升级的 Fatal
    /// → max 档；5xx streak → 现行曲线；429 → base 短冷却）。None = 尚未冷却过。
    pub last_cooling_outcome: Option<ChannelOutcome>,
}

impl Default for HealthState {
    fn default() -> Self {
        Self {
            ewma_score: DEFAULT_SCORE,
            request_count: 0,
            unauthorized_run: 0,
            ramp_exited: false,
            ramp_pending: false,
            failure_streak: 0,
            cooldown_streak: 0,
            cooldown_until_ms: 0,
            anneal_since_ms: 0,
            last_cooling_outcome: None,
        }
    }
}

/// 健康配置 — phase0 `ChannelHealthSetting` 默认值。
#[derive(Debug, Clone, Copy)]
pub struct HealthSetting {
    pub enabled: bool,
    /// EWMA 平滑系数。
    pub alpha: f64,
    /// 健康分下限。
    pub min_score: f64,
    /// EWMA 可信前的最小请求数。
    pub min_requests: u32,
    /// 连续 fatal/throttled 触发冷却的阈值。
    pub cooldown_threshold: u32,
    /// 冷却基础时长 (s)。
    pub cooldown_base_seconds: u64,
    /// 冷却最大时长 (s)。
    pub cooldown_max_seconds: u64,
    /// 同层冷却渠道最大弹射占比 (%)。
    pub cooldown_max_ejection_percent: u8,
    /// 冷却时长递增因子 (独立于 EWMA alpha)。
    pub cooldown_alpha: f64,
    /// 冷却激活达此数 → 该 model 在渠道上禁用。
    pub cooldown_disable_streak: u32,
}

impl Default for HealthSetting {
    fn default() -> Self {
        Self {
            enabled: true,
            alpha: 0.3,
            min_score: 0.05,
            min_requests: 5,
            cooldown_threshold: 5,
            cooldown_base_seconds: 10,
            cooldown_max_seconds: 60,
            cooldown_max_ejection_percent: 50,
            cooldown_alpha: 0.3,
            cooldown_disable_streak: 3,
        }
    }
}

/// 429 的 EWMA 观测值 (轻度降权, 非致命)。
pub const THROTTLED_OBSERVATION: f64 = 0.7;
/// 连续 401/403 达此数升级为 Fatal。
pub const UNAUTHORIZED_ESCALATION_THRESHOLD: u32 = 3;
/// 无历史渠道的默认分。
pub const DEFAULT_SCORE: f64 = 1.0;

/// 健康表 trait — 按候选 key (RouteUnitRecord::meta.key) 读写健康状态。
pub trait HealthTable: Send + Sync {
    /// 读取当前健康状态; 无记录 → Default (满健康)。
    fn get(&self, unit_key: &str) -> HealthState;

    /// 上报一次尝试结果 (带上游状态码), 内部按 phase0 规则分类并记账。
    ///
    /// - `Ok(status)`: 按状态码分类 (2xx→Success, 429→Throttled, 5xx→Fatal,
    ///   401→升级计数, 其它 4xx→Neutral);
    /// - `Err(Retryable)`: 传输层失败 → Fatal (渠道当前不可用);
    /// - `Err(Fatal)`: 客户端问题 → Neutral (渠道无责)。
    fn record(&self, unit_key: &str, outcome: Result<u16, FailureClass>);

    /// 门控判定: 候选是否可参与本轮选择 (冷却中 → 否)。
    fn is_selectable(&self, unit_key: &str, now_ms: u64) -> bool;

    /// 剩余冷却时间 — `Some(ms)` 表示该渠道正在冷却 (未到期),
    /// `None` 表示未冷却或冷却已到期。selector 在池枯竭时用它找出
    /// 最接近恢复的渠道做紧急召回。
    fn cooling_remaining_ms(&self, unit_key: &str, now_ms: u64) -> Option<u64>;

    /// 紧急召回 — 强制结束该渠道的冷却 (复用 finish_cooldown 语义:
    /// 清冷却、重置请求计数、武装 slow-start ramp)。
    /// **保留 cooldown_streak**: 召回不是宽恕, 下一次失败仍按原档位爬升冷却时长。
    fn force_recall(&self, unit_key: &str);

    /// 路由权重 — phase0 `RoutingWeight` 语义:
    /// 冷却中 → 0 (除非 max-ejection 豁免, V1 未实现 bypass);
    /// 否则 `base_weight × ewma_score × slow_start_factor`。
    fn routing_weight(&self, unit_key: &str, base_weight: u32, now_ms: u64) -> f64;
}

type StateMap = HashMap<String, HealthState>;

/// 纯内存健康表 — phase0 `HealthStore` 的 Rust 移植。
/// 配置在构造时固化 (TODO(#311) 运行时热更新用 ArcSwap)。
pub struct MemoryHealthTable {
    states: Mutex<StateMap>,
    cfg: HealthSetting,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl Default for MemoryHealthTable {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryHealthTable {
    pub fn new() -> Self {
        Self::with_config(HealthSetting::default())
    }

    pub fn with_config(cfg: HealthSetting) -> Self {
        Self::with_config_and_clock(cfg, || chrono::Utc::now().timestamp_millis().max(0) as u64)
    }

    /// 测试/确定性时钟注入。
    pub fn with_config_and_clock(
        cfg: HealthSetting,
        now_ms: impl Fn() -> u64 + Send + Sync + 'static,
    ) -> Self {
        Self {
            states: Mutex::new(StateMap::new()),
            cfg,
            now_ms: Box::new(now_ms),
        }
    }

    /// 取锁, 毒化不 panic (一个线程 panic 不该拖垮全部请求)。
    fn lock(&self) -> std::sync::MutexGuard<'_, StateMap> {
        self.states.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 全表只读口 — 返回当前所有健康记录的 (unit_key, 状态) 快照列表。
    ///
    /// 语义：锁内克隆整张表后释放锁（`HealthState` 是 `Copy`，克隆无堆分配
    /// 风险；调用方拿到的是数据副本，后续再读不受并发 record 影响）。
    /// 只读、不结算过期冷却 — 查询面不产生副作用（与 `is_selectable` /
    /// `routing_weight` 的惰性结算语义区分）。
    /// 未 `record` 过的 unit 不在表里，不会出现在结果中。
    /// 面向 admin 查询面（`/api/gateway/health`），不在热路径调用
    /// （锁内整表克隆，规模随 record 过的渠道数增长）。
    pub fn entries(&self) -> Vec<(String, HealthState)> {
        let states = self.lock();
        states.iter().map(|(key, st)| (key.clone(), *st)).collect()
    }
}

impl HealthTable for MemoryHealthTable {
    fn get(&self, unit_key: &str) -> HealthState {
        self.lock().get(unit_key).copied().unwrap_or_default()
    }

    fn record(&self, unit_key: &str, outcome: Result<u16, FailureClass>) {
        let now = (self.now_ms)();
        let mut states = self.lock();
        let mut st = states.get(unit_key).copied().unwrap_or_default();
        apply_outcome(&mut st, outcome, &self.cfg, now);
        states.insert(unit_key.to_string(), st);
    }

    fn is_selectable(&self, unit_key: &str, now_ms: u64) -> bool {
        // 惰性结算过期冷却 (phase0 FilterCoolingChannels 语义) — 冷却到期的
        // 渠道在此次门控即重入 slow-start ramp。
        let mut states = self.lock();
        let Some(st) = states.get_mut(unit_key) else {
            return true; // 无历史 = 未冷却
        };
        if st.cooldown_until_ms != 0 && !st.is_cooling(now_ms) {
            finish_cooldown(st, now_ms);
        }
        // 空闲退火: 无活跃流量的渠道冷却连击按 base 窗口衰减。
        anneal_streak(st, &self.cfg, now_ms);
        !st.is_cooling(now_ms)
    }

    fn routing_weight(&self, unit_key: &str, base_weight: u32, now_ms: u64) -> f64 {
        if !self.cfg.enabled {
            return f64::from(base_weight);
        }
        let mut states = self.lock();
        let st = states.get_mut(unit_key);
        let Some(st) = st else {
            return f64::from(base_weight); // 无历史 = 满健康
        };
        // 惰性结算过期冷却 (phase0 RoutingWeight 语义): 恢复渠道在此次选择
        // 就重入 slow-start ramp, 而不是等下一次 record。
        if st.cooldown_until_ms != 0 && !st.is_cooling(now_ms) {
            finish_cooldown(st, now_ms);
        }
        anneal_streak(st, &self.cfg, now_ms);
        if st.is_cooling(now_ms) {
            return 0.0;
        }
        f64::from(base_weight) * st.ewma_score * st.slow_start_factor(self.cfg.min_requests)
    }

    fn cooling_remaining_ms(&self, unit_key: &str, now_ms: u64) -> Option<u64> {
        // 冷却截止时刻严格大于当前时刻才算冷却中 (与 is_cooling 同一口径);
        // 未记录 / 从未冷却 (cooldown_until_ms == 0) → None。
        self.lock()
            .get(unit_key)
            .and_then(|st| st.cooldown_until_ms.checked_sub(now_ms))
    }

    fn force_recall(&self, unit_key: &str) {
        // 用表内注入的时钟: 召回即"冷却此刻结束", 退火锚点也从此刻起算。
        let now = (self.now_ms)();
        if let Some(st) = self.lock().get_mut(unit_key) {
            finish_cooldown(st, now);
        }
    }
}

impl HealthState {
    /// 是否正处于冷却窗口 (且未过期)。
    pub fn is_cooling(&self, now_ms: u64) -> bool {
        self.cooldown_until_ms > now_ms
    }

    /// 慢启动因子 — phase0 `slowStartFactor`:
    /// - RampExited → 1.0 (真实失败, 不再渐进);
    /// - RampPending → 1/min_requests (冷却后首选的 ramp 地板);
    /// - 未达 min_requests → count/min_requests (渐进);
    /// - 达阈值 → 1.0。
    pub fn slow_start_factor(&self, min_requests: u32) -> f64 {
        if min_requests == 0 || self.ramp_exited {
            return 1.0;
        }
        if self.ramp_pending {
            return 1.0 / f64::from(min_requests);
        }
        if self.request_count == 0 || self.request_count >= min_requests {
            return 1.0;
        }
        f64::from(self.request_count) / f64::from(min_requests)
    }
}

/// 按结果分类并记账 — phase0 `recordChannelOutcome` 的逻辑主体。
fn apply_outcome(
    st: &mut HealthState,
    outcome: Result<u16, FailureClass>,
    cfg: &HealthSetting,
    now_ms: u64,
) {
    if !cfg.enabled {
        return;
    }

    // 先结算过期冷却 (post-expiry 结果干净地重入 slow-start ramp)。
    if st.cooldown_until_ms != 0 && !st.is_cooling(now_ms) {
        finish_cooldown(st, now_ms);
    }

    let outcome = classify(st, outcome);

    match outcome {
        ChannelOutcome::Success => {
            st.failure_streak = 0;
            // 活跃事件重新计期: 退火锚点清零, 空闲窗口从头算。
            st.anneal_since_ms = 0;
            // 冷却过期后的干净成功递减冷却连击 (下次失败从更短时长起步)。
            if st.cooldown_streak > 0 && st.cooldown_until_ms == 0 {
                st.cooldown_streak = st.cooldown_streak.saturating_sub(1);
            }
        }
        // Neutral (400/404/孤立 401): 渠道无责 —— 不改分、不计请求,
        // 但失败连击保留: 错误码出现即计入连击, 否则 500/404 交替会让
        // streak 永远在 1↔0 跳, 冷却阈值永不触达。
        ChannelOutcome::Neutral => return,
        ChannelOutcome::Fatal | ChannelOutcome::Throttled => {
            st.failure_streak += 1;
        }
    }

    // EWMA 观测值。
    let observation = match outcome {
        ChannelOutcome::Success => 1.0,
        ChannelOutcome::Fatal => 0.0,
        ChannelOutcome::Throttled => THROTTLED_OBSERVATION,
        ChannelOutcome::Neutral => unreachable!("Neutral returned above"),
    };

    st.request_count += 1;
    st.ramp_pending = false;

    // 真实失败立即退出 ramp。
    if outcome == ChannelOutcome::Fatal {
        st.ramp_exited = true;
    }

    // MinRequests 前不更新 EWMA (信任新渠道)。
    if st.request_count > cfg.min_requests {
        st.ewma_score = cfg.alpha * observation + (1.0 - cfg.alpha) * st.ewma_score;
        if st.ewma_score < cfg.min_score {
            st.ewma_score = cfg.min_score;
        }
    }

    // 冷却触发在 MinRequests 门控之外 (新渠道也可能被立即弹射)。
    if matches!(outcome, ChannelOutcome::Fatal | ChannelOutcome::Throttled)
        && st.failure_streak >= cfg.cooldown_threshold
    {
        start_cooldown(st, cfg, now_ms, outcome);
    }
}

/// 分类 — phase0 `classifyChannelOutcomeUnlocked` + UnauthorizedRun 升级。
/// 401/403 独立分支 (P1-A): 孤立凭据错误保持 Neutral 语义 (渠道无责不改分),
/// 但计入 unauthorized_run; 连续达阈值 → Fatal。成功/429/5xx/其它 4xx 清零。
fn classify(st: &mut HealthState, outcome: Result<u16, FailureClass>) -> ChannelOutcome {
    match outcome {
        Ok(status) => match status {
            200..=299 => {
                st.unauthorized_run = 0;
                ChannelOutcome::Success
            }
            429 => {
                st.unauthorized_run = 0;
                ChannelOutcome::Throttled
            }
            401 | 403 => {
                if st.unauthorized_run < UNAUTHORIZED_ESCALATION_THRESHOLD {
                    st.unauthorized_run += 1;
                }
                if st.unauthorized_run >= UNAUTHORIZED_ESCALATION_THRESHOLD {
                    ChannelOutcome::Fatal
                } else {
                    ChannelOutcome::Neutral
                }
            }
            500..=599 => {
                st.unauthorized_run = 0;
                ChannelOutcome::Fatal
            }
            // 其它 4xx (400/404/422 等): 渠道无责, 不改分。
            _ => {
                st.unauthorized_run = 0;
                ChannelOutcome::Neutral
            }
        },
        Err(FailureClass::Retryable) => {
            // 传输层失败 (超时/连接) → 渠道当前不可用。
            st.unauthorized_run = 0;
            ChannelOutcome::Fatal
        }
        Err(FailureClass::Fatal) => {
            // 客户端问题 → 渠道无责。
            st.unauthorized_run = 0;
            ChannelOutcome::Neutral
        }
    }
}

/// 冷却时长 (5xx/传输层曲线) — phase0 `CooldownDuration`:
/// `base + (max-base) × (1 - cooldown_alpha^prior_activations)`。
/// 签名保持纯 (cfg, streak): outcome 分档在 `start_cooldown` 调用点做。
fn cooldown_duration_ms(cfg: &HealthSetting, prior_activations: u32) -> u64 {
    let base = cfg.cooldown_base_seconds;
    let max = cfg.cooldown_max_seconds.max(base);
    if base == 0 || max == 0 {
        return 0;
    }
    let factor = 1.0 - cfg.cooldown_alpha.powi(prior_activations as i32);
    let secs = base as f64 + (max - base) as f64 * factor;
    let secs = secs.clamp(base as f64, max as f64);
    (secs * 1000.0) as u64
}

/// 进入冷却 — phase0 `startCooldownLocked` + P1-A outcome 分档:
/// - 401/403-run 升级的 Fatal → max 档 (凭据坏是持续态, 短冷却只会空转);
/// - 429 (Throttled) → base 短冷却 (限流是暂态, 不随 streak 爬向 max);
/// - 5xx/传输层 streak → phase0 递增曲线。
///
/// 时长由当前 cooldown_streak 决定, 激活后 streak+1, 重置请求计数进入 ramp;
/// 成功进入冷却时把触发 outcome 记入 last_cooling_outcome。
fn start_cooldown(st: &mut HealthState, cfg: &HealthSetting, now_ms: u64, outcome: ChannelOutcome) {
    let d = match outcome {
        ChannelOutcome::Fatal if st.unauthorized_run >= UNAUTHORIZED_ESCALATION_THRESHOLD => {
            cfg.cooldown_max_seconds.max(cfg.cooldown_base_seconds) * 1000
        }
        ChannelOutcome::Throttled => cfg.cooldown_base_seconds * 1000,
        _ => cooldown_duration_ms(cfg, st.cooldown_streak),
    };
    if d == 0 {
        return;
    }
    st.last_cooling_outcome = Some(outcome);
    st.cooldown_streak += 1;
    st.failure_streak = 0;
    st.request_count = 0;
    st.ramp_exited = false;
    st.cooldown_until_ms = now_ms + d;
}

/// 结束冷却 — phase0 `finishCooldownLocked`: 清除冷却, 武装 slow-start ramp。
/// `now_ms` 记为退火起点: 之后若持续空闲, cooldown_streak 按 base 窗口衰减。
fn finish_cooldown(st: &mut HealthState, now_ms: u64) {
    st.cooldown_until_ms = 0;
    st.request_count = 0;
    st.ramp_exited = false;
    st.ramp_pending = true;
    st.anneal_since_ms = now_ms;
}

/// 空闲退火 — 冷却到期后若持续无流量, cooldown_streak 随时间线性回落。
///
/// 退火窗口复用 `cooldown_base_seconds` (不加新配置): 每过一个完整窗口
/// streak -1 (saturating 到 0), 零头留给下一次结算。活跃流量本身会经
/// apply_outcome 的 Success 分支递减 streak 并清零锚点, 所以退火只管"静默"
/// 的渠道; 锚点为 0 (成功过) 时休眠, 直到下一次冷却结束重新设锚。
///
/// - `st`: 待结算状态, 原地衰减;
/// - `cfg`: 提供退火窗口时长;
/// - `now_ms`: 当前时钟。
///
/// 仅在读取路径 (is_selectable / routing_weight) 惰性调用 —— record 本身
/// 就是活跃事件, 不需要退火。
fn anneal_streak(st: &mut HealthState, cfg: &HealthSetting, now_ms: u64) {
    if st.cooldown_until_ms != 0 || st.cooldown_streak == 0 || st.anneal_since_ms == 0 {
        return;
    }
    let window_ms = cfg.cooldown_base_seconds.saturating_mul(1000);
    if window_ms == 0 {
        return;
    }
    let steps = now_ms.saturating_sub(st.anneal_since_ms) / window_ms;
    if steps == 0 {
        return;
    }
    st.cooldown_streak = st.cooldown_streak.saturating_sub(steps as u32);
    st.anneal_since_ms += steps * window_ms;
    if st.cooldown_streak == 0 {
        st.anneal_since_ms = 0;
    }
}
