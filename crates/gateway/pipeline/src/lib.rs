//! `gateway-pipeline` —— gateway 聚合目录的装配入口
//!
//! 负责定义 Stage trait / RequestCtx / Pipeline / GatewayShared / axum 集成。
//! 其它四个 gateway crate（admission / dispatch / forward / protocol-bridge）
//! 依赖本 crate 拿到这些共享类型，但反过来不依赖它们。
//!
//! ## 文件分工
//!
//! - [`ctx`] —— 请求上下文: `RequestCtx` / `RequestMeta` / `BodySource` / `PipeStream` / `ProtocolKind`
//! - [`stage`] —— `Stage` trait + `StageError` + `StageOutcome`
//! - [`pipeline`] —— 链式 `Pipeline` 编排器
//! - [`router`] —— axum 集成 (`build_router` + `error_to_response`)
//!
//! 本文件 (`lib.rs`) 装载所有跨请求共享的占位类型 + `GatewayShared` 总入口。
//! 详细设计见 [`README.md`](https://github.com/)。

pub mod ctx;
pub mod stage;

pub use ctx::{RequestCtx, RequestMeta, StageOutcome, PipeStream, BodySource, ProtocolKind};
pub use stage::{Stage, StageError, UpstreamError};
pub use pipeline::Pipeline;
pub use router::{build_router, error_to_response};

// ============================================================================
// ArcSwap 跨请求只读快照（来自 service::sync）
// ============================================================================

/// Token 快照（来自 `service::sync`）
#[derive(Default)]
pub struct TokenSnapshot;

impl TokenSnapshot {
    /// 查找 token（按 sha256 哈希，O(1)）
    pub fn lookup(&self, _hash: &[u8; 32]) -> Option<TokenInfo> {
        // TODO: hash index lookup
        unimplemented!("TokenSnapshot::lookup")
    }

    /// 解析 `sk-xxx-<channelId>` 后缀钉渠道
    pub fn resolve_channel_pin(&self, _raw_key: &str) -> Option<i64> {
        // TODO: 解析 sk-xxx-<channelId> 后缀
        unimplemented!("TokenSnapshot::resolve_channel_pin")
    }
}

/// Token 鉴权后产物（Admission 写入 RequestCtx.token）
#[derive(Debug, Clone)]
pub struct TokenInfo {
    pub id: i64,
    pub group: String,
    pub enabled: bool,
    pub allowed_models: Option<Vec<String>>,
    pub auth_version: u64,
}

/// 路由能力快照（`(group, model) -> Vec<RouteUnit>`）
#[derive(Default)]
pub struct AbilitiesSnapshot;

impl AbilitiesSnapshot {
    /// 按 (group, model) 匹配候选 RouteUnit
    pub fn match_route(&self, _group: &str, _model: &str) -> Vec<RouteUnit> {
        // TODO: 二级索引查询
        unimplemented!("AbilitiesSnapshot::match_route")
    }
}

/// 路由单元（候选最小单位）
#[derive(Debug, Clone)]
pub struct RouteUnit {
    pub channel_id: i64,
    pub priority: i32,
    pub weight: u32,
    pub status: RouteStatus,
    pub api_type: u32,
    pub base_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteStatus {
    Enabled,
    Disabled,
}

/// 价格快照
#[derive(Default)]
pub struct PricingSnapshot;

impl PricingSnapshot {
    /// 按 (model, group) 查价格
    pub fn lookup(&self, _model: &str, _group: &str) -> Option<PriceRow> {
        // TODO: 价格表查询
        unimplemented!("PricingSnapshot::lookup")
    }
}

#[derive(Debug, Clone)]
pub struct PriceRow {
    pub input_per_m: f64,
    pub output_per_m: f64,
    pub cache_per_m: f64,
}

/// IP 白名单
#[derive(Default)]
pub struct IpPolicy;

impl IpPolicy {
    /// 检查 IP 是否被允许
    pub fn allows(&self, _ip: &std::net::IpAddr) -> bool {
        // TODO: CIDR 匹配
        unimplemented!("IpPolicy::allows")
    }
}

/// 配额快照
#[derive(Default)]
pub struct QuotaSnapshot;

impl QuotaSnapshot {
    /// 按 token_id 查剩余配额
    pub fn remaining(&self, _token_id: i64) -> i64 {
        // TODO: 配额余额查询
        unimplemented!("QuotaSnapshot::remaining")
    }
}

/// 敏感词库
#[derive(Default)]
pub struct SensitiveWords;

impl SensitiveWords {
    /// Aho-Corasick 扫描
    pub fn scan(&self, _input: &[u8]) -> Vec<MatchHit> {
        // TODO: AC 自动机扫描
        unimplemented!("SensitiveWords::scan")
    }
}

#[derive(Debug, Clone)]
pub struct MatchHit {
    pub start: usize,
    pub end: usize,
    pub word: String,
}

// ============================================================================
// 进程内高频写结构
// ============================================================================

/// 渠道健康度（进程内 EWMA + 冷却）
pub struct ChannelHealth {
    pub ewma_ms: std::sync::atomic::AtomicU64,
    pub consecutive_fails: std::sync::atomic::AtomicU32,
    pub cooled_until: std::sync::atomic::AtomicU64,
}

impl Default for ChannelHealth {
    fn default() -> Self {
        Self {
            ewma_ms: std::sync::atomic::AtomicU64::new(0),
            consecutive_fails: std::sync::atomic::AtomicU32::new(0),
            cooled_until: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

/// 健康表
pub struct HealthTable {
    inner: dashmap::DashMap<i64, ChannelHealth>,
}

impl HealthTable {
    pub fn new() -> Self {
        Self { inner: dashmap::DashMap::new() }
    }

    /// 记录一次执行结果（更新 EWMA / 失败连击 / 冷却）
    pub fn record(&self, _channel_id: i64, _latency: std::time::Duration, _success: bool) {
        // TODO: EWMA α=0.3 + 失败连击 5 触发冷却 30s
        unimplemented!("HealthTable::record")
    }

    /// 渠道是否可选（不在冷却中且 status 启用）
    pub fn is_selectable(&self, _channel_id: i64) -> bool {
        // TODO: 读 cooled_until + consecutive_fails
        unimplemented!("HealthTable::is_selectable")
    }
}

impl Default for HealthTable {
    fn default() -> Self { Self::new() }
}

/// 限流器（`N/[N]s|m|h|d` 表达式）
#[derive(Default)]
pub struct RateLimiter;

impl RateLimiter {
    pub fn new() -> Self { Self }

    /// 解析限流表达式（如 `100/1m`）
    pub fn parse(&self, _expr: &str) -> Result<LimitSpec, ParseError> {
        // TODO: 解析 `100/1m` 等表达式
        unimplemented!("RateLimiter::parse")
    }

    /// 检查是否允许通过
    pub fn try_acquire(&self, _scope: LimitScope, _key: i64) -> bool {
        // TODO: 滑动窗口检查
        unimplemented!("RateLimiter::try_acquire")
    }
}

pub struct LimitSpec {
    pub count: u32,
    pub window: std::time::Duration,
}

pub enum LimitScope {
    PerChannel,
    PerKey,
    PerGroup,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("invalid rate limit expression: {0}")]
    Invalid(String),
}

/// 计量表（进程内预扣 / 结算 / 退还）
#[derive(Default)]
pub struct MeteringTable;

impl MeteringTable {
    pub fn new() -> Self { Self }

    /// 预扣额度，返回 hold_id（原子操作）
    pub fn prehold(&self, _key: i64, _cost: i64) -> u64 {
        // TODO: parking_lot::Mutex + u64 hold_id 原子操作
        unimplemented!("MeteringTable::prehold")
    }

    /// 结算（差额回补）
    pub fn settle(&self, _hold_id: u64, _actual_cost: i64) {
        // TODO: 差额计算
        unimplemented!("MeteringTable::settle")
    }

    /// 退还（请求失败）
    pub fn release(&self, _hold_id: u64) {
        // TODO: 退还已预扣
        unimplemented!("MeteringTable::release")
    }
}

/// Hyper 客户端（连接池 + 代理池 + 三段超时）
pub struct HyperClient;

impl HyperClient {
    pub fn new() -> Self { Self }

    /// 构造并应用超时配置
    pub fn with_config(_config: HyperConfig) -> Self {
        // TODO: 连接池 + 三段超时
        unimplemented!("HyperClient::with_config")
    }

    /// 发起上游请求
    pub fn request(&self, _req: http::Request<bytes::Bytes>) -> Result<http::Response<UpstreamBody>, UpstreamError> {
        // TODO: 拨号 + 写 + 读头
        unimplemented!("HyperClient::request")
    }
}

impl Default for HyperClient {
    fn default() -> Self { Self::new() }
}

pub enum UpstreamBody {
    Full(bytes::Bytes),
    Streaming(futures_util::stream::BoxStream<'static, Result<bytes::Bytes, UpstreamError>>),
}

#[derive(Clone, Debug)]
pub struct HyperConfig {
    pub connect_timeout: std::time::Duration,
    pub first_byte_timeout: std::time::Duration,
    pub total_timeout: std::time::Duration,
}

impl Default for HyperConfig {
    fn default() -> Self {
        Self {
            connect_timeout: std::time::Duration::from_secs(5),
            first_byte_timeout: std::time::Duration::from_secs(30),
            total_timeout: std::time::Duration::from_secs(300),
        }
    }
}

/// Body 存储（内存 / 临时文件双模式）
pub struct BodyStorage;

impl BodyStorage {
    pub fn new() -> Self { Self }

    /// 存储请求体（自动选择内存/磁盘）
    pub fn store(&self, _body: bytes::Bytes) -> crate::ctx::BodySource {
        // TODO: memory_threshold 决策
        unimplemented!("BodyStorage::store")
    }
}

impl Default for BodyStorage {
    fn default() -> Self { Self::new() }
}

/// SSE 扫描器
pub struct SseScanner;

impl SseScanner {
    pub fn new() -> Self { Self }

    /// 三站管道：帧扫描 → 抽 usage → 客户端
    pub fn scan(
        &self,
        _stream: futures_util::stream::BoxStream<'static, Result<bytes::Bytes, UpstreamError>>,
    ) -> crate::ctx::PipeStream {
        // TODO: SSE 帧扫描 + CtxTail 跨 chunk
        unimplemented!("SseScanner::scan")
    }
}

impl Default for SseScanner {
    fn default() -> Self { Self::new() }
}

/// 协议 Codec 注册表
pub struct CodecRegistry;

impl CodecRegistry {
    pub fn with_defaults() -> Self { Self }

    /// 按协议查找 codec
    pub fn get(&self, _kind: crate::ctx::ProtocolKind) -> Option<std::sync::Arc<dyn std::any::Any>> {
        // TODO: codec map 查找
        unimplemented!("CodecRegistry::get")
    }
}

impl Default for CodecRegistry {
    fn default() -> Self { Self::with_defaults() }
}

// ============================================================================
// GatewayShared —— 跨请求共享状态总入口
// ============================================================================

/// 跨请求共享状态总入口
/// User 快照（来自 `service::sync`）
#[derive(Default)]
pub struct UserSnapshot;

impl UserSnapshot {
    /// 查 user
    pub fn lookup(&self, _user_id: i64) -> Option<UserRecord> {
        // TODO: DashMap<i64, UserRecord> 查询
        unimplemented!("UserSnapshot::lookup")
    }
}

#[derive(Debug, Clone)]
pub struct UserRecord {
    pub id: i64,
    pub enabled: bool,
    pub group: String,
    pub auth_version: u64,
}

/// 跨请求共享状态总入口
///
/// 在 `apps/gateway` 启动时构造一次，clone 到各 stage 中。
pub struct GatewayShared {
    // ---- ArcSwap 跨请求只读快照（来自 service::sync）----
    pub tokens:         std::sync::Arc<arc_swap::ArcSwap<TokenSnapshot>>,
    pub users:          std::sync::Arc<arc_swap::ArcSwap<UserSnapshot>>,
    pub pricing:        std::sync::Arc<arc_swap::ArcSwap<PricingSnapshot>>,
    pub allow_ips:      std::sync::Arc<arc_swap::ArcSwap<IpPolicy>>,
    pub quota:          std::sync::Arc<arc_swap::ArcSwap<QuotaSnapshot>>,
    pub sensitive_words: std::sync::Arc<arc_swap::ArcSwap<SensitiveWords>>,

    // ---- 进程内高频写结构 ----
    pub health:         std::sync::Arc<HealthTable>,
    pub rate_limiter:   std::sync::Arc<RateLimiter>,
    pub metering_table: std::sync::Arc<MeteringTable>,
    pub hyper_client:   std::sync::Arc<HyperClient>,
    pub body_storage:   std::sync::Arc<BodyStorage>,
    pub sse_scanner:    std::sync::Arc<SseScanner>,
    pub proxy_pool:     std::sync::Arc<ProxyPool>,
    pub codecs:         std::sync::Arc<CodecRegistry>,
}

/// 出口代理池（按 channel 索引）
///
/// 实际实现在 `gateway-proxy` crate；本类型作为引用占位，编译时由
/// `apps/gateway` 把 `gateway_proxy::ProxyPool` 通过 newtype 注入。
pub struct ProxyPool;
