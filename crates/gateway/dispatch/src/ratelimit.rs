//! 纯内存滑动窗口限流 — 按单元 (RouteUnit) 维度的请求频率保护。
//!
//! 参考实现:
//! - wildtoken `internal/ratelimit/{limiter.go,parser.go}`:
//!   按 key 维护秒级桶计数 (`BTreeMap<unix_sec, count>`), 窗口起点向下取整
//!   到整秒, 桶内计数 >= 上限即拒绝; 兜底永远在严格侧 (宁可多拒一次, 绝不
//!   漏放应该被拒的请求, 与 limiter.go 注释一致)。
//!
//! 设计取舍:
//! - 不引入后台清理线程; admits 路径顺手清掉早于 `now - 2h` 的旧桶 (避免
//!   长尾 key 残留内存, 也避免任何定时器故障牵连热路径)。
//! - 时钟与 health.rs 同模式: 默认 `chrono::Utc::now()`, 测试用 `with_clock`
//!   注入确定性毫秒。
//! - 空 spec / `None` 视作不限流; 配置层不填即无开销 (select 路径会先判
//!   limits 是否为空再走限流判定)。

use std::collections::{BTreeMap, HashMap};
use std::sync::Mutex;

/// 解析后的限流规格 — "requests 个请求 / window_ms 毫秒"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitSpec {
    pub requests: u64,
    pub window_ms: u64,
}

impl RateLimitSpec {
    /// 语法 `^(\d+)/(\d+)?([smhd])$`:
    ///   - `100/s`  → 100 req / 1s
    ///   - `200/5m` → 200 req / 5min
    ///   - `10000/d`→ 10000 req / 24h
    ///
    /// 空串与非法串都返回 `None` (ocr #3): 调用方无法区分"不限流"与
    /// "配置写错"。ponytail: parse 不区分, 错误由配置层在写入时拦截
    /// (wildtoken 同款—写时 NormalizeRateLimit 校验); 热路径只关心有没有。
    /// `requests == 0` 或 `mult == 0` 同样拒绝。
    pub fn parse(s: &str) -> Option<Self> {
        if s.is_empty() {
            return None;
        }
        // ponytail: 手写扫描而不是正则 — 模式简单, 避免拖一个 regex 依赖。
        let bytes = s.as_bytes();
        let mut i = 0;
        // 请求数: \d+
        if i >= bytes.len() || !bytes[i].is_ascii_digit() {
            return None;
        }
        let req_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let requests: u64 = s[req_start..i].parse().ok()?;
        if requests == 0 {
            return None;
        }
        // '/'
        if i >= bytes.len() || bytes[i] != b'/' {
            return None;
        }
        i += 1;
        // 可选倍数: \d*
        let mult_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        let mult: u64 = if i > mult_start {
            s[mult_start..i].parse().ok()?
        } else {
            1
        };
        if mult == 0 {
            return None;
        }
        // 单位: 单字符 [smhd]
        if i >= bytes.len() {
            return None;
        }
        let unit = bytes[i];
        if i + 1 != bytes.len() {
            return None; // 必须正好在单位处收尾
        }
        let base_ms: u64 = match unit {
            b's' => 1_000,
            b'm' => 60_000,
            b'h' => 3_600_000,
            b'd' => 86_400_000,
            _ => return None,
        };
        Some(RateLimitSpec {
            requests,
            window_ms: base_ms.checked_mul(mult)?,
        })
    }
}

/// 单元内桶: unix 秒 → 该秒内的累计计数。`BTreeMap` 便于按起点裁剪旧桶
/// (`split_off`/`range`), 也保证按时间顺序遍历做和。
type Buckets = BTreeMap<i64, u64>;

type LimitMap = HashMap<String, Buckets>;

/// 纯内存滑动窗口限流器 — 每 Pod 独立, 不跨节点同步。
///
/// `admits` 接受显式 `now_ms` 以便测试注入; 生产路径可调 `SlidingWindow::new()`
/// (默认取系统时钟), 也可走 `with_clock` 注入确定性时钟。
pub struct SlidingWindow {
    limits: Mutex<LimitMap>,
    now_ms: Box<dyn Fn() -> u64 + Send + Sync>,
}

impl Default for SlidingWindow {
    fn default() -> Self {
        Self::new()
    }
}

impl SlidingWindow {
    pub fn new() -> Self {
        Self::with_clock(|| chrono::Utc::now().timestamp_millis().max(0) as u64)
    }

    /// 时钟注入 (测试确定性)。
    pub fn with_clock(now_ms: impl Fn() -> u64 + Send + Sync + 'static) -> Self {
        Self {
            limits: Mutex::new(LimitMap::new()),
            now_ms: Box::new(now_ms),
        }
    }

    /// 取锁, 毒化不 panic (ocr #2: 热路径一个线程 panic 不该拖垮所有请求)。
    fn lock(&self) -> std::sync::MutexGuard<'_, LimitMap> {
        self.limits.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 判定 + 记账: 限额内放行并 +1, 超限返回 false 且不递增。
    ///
    /// `None` spec 永远放行 (配置层短路, 零开销)。
    pub fn admits(&self, key: &str, spec: Option<&RateLimitSpec>, now_ms: u64) -> bool {
        let spec = match spec {
            Some(s) if s.requests > 0 && s.window_ms > 0 => s,
            _ => return true,
        };

        let now_s = (now_ms / 1000) as i64;
        let window_s = (spec.window_ms / 1000).max(1) as i64;
        // 窗口起点: 包含当前秒, 向后覆盖 window_s 个完整秒桶。
        // 起点 = now_s - window_s + 1; 严格侧判定: 落在该区间的桶累计 < requests 放行。
        let start_s = now_s - window_s + 1;

        let mut limits = self.lock();
        let buckets = limits.entry(key.to_string()).or_default();

        // 顺手清理 < now_s - 7200 (2h) 的桶 — 防止长尾 key 内存膨胀。
        // ponytail: 每次 admits 都清, 但 retain 只在有旧桶时才实际分配;
        // 后台清理线程省掉, 热路径上这点成本换来零定时器牵连。
        let cutoff = now_s - 7200;
        if buckets
            .keys()
            .next()
            .copied()
            .is_some_and(|first| first < cutoff)
        {
            buckets.retain(|bucket, _| *bucket >= cutoff);
        }

        let current_count: u64 = buckets.range(start_s..=now_s).map(|(_, c)| *c).sum();

        if current_count >= spec.requests {
            return false;
        }

        *buckets.entry(now_s).or_insert(0) += 1;
        true
    }

    /// 测试辅助: 用内部时钟取 now_ms 后再调 admits。生产路径通常直接传
    /// `chrono::Utc::now()` 避免一次闭包调用, 注入时钟主要用于测试。
    pub fn admits_now(&self, key: &str, spec: Option<&RateLimitSpec>) -> bool {
        self.admits(key, spec, (self.now_ms)())
    }
}
