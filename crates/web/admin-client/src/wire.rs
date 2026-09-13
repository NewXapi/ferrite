//! 网关渠道健康 — 本地 wire DTO 与客户端封装。
//!
//! 对应后端 `crates/api/admin-observe/src/gateway_health.rs` 的
//! `GET /api/gateway/health` (admin 角色白名单,响应无外层 Envelope
//! 包装——端点直接返回 `GatewayHealthView` 裸 JSON):
//!
//! ```json
//! { "items": [ { "unitKey", "channelKey", "channelName", "publicModel",
//!   "state": "cooling"|"slow_start"|"ok", "lastCoolingOutcome",
//!   "remainingCooldownMs", "slowStartFactor" } ] }
//! ```
//!
//! 语义:items 只含有记录的渠道 (健康表从未 record 过 / 已自然恢复的
//! 渠道不出现,正常态是空数组),因此 **空 items 不是错误**。
//!
//! DTO 按任务约定定义在本 crate (web 域私有),不进 `crates/contract`,
//! 避免跨域共享点变更。

use crate::{ApiClient, ApiResult};

/// 渠道健康三态 — 与后端 `HealthStateView` 同源
/// (`#[serde(rename_all = "snake_case")]`:cooling / slow_start / ok)。
///
/// 前端轮询判定用它:存在 cooling / slow_start 条目才继续轮询。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthItemState {
    /// 冷却窗口内。
    Cooling,
    /// 冷却刚结束、ramp 渐进期。
    SlowStart,
    /// 当前健康 (含历史 404/401 等 Neutral 记录但当前健康的渠道)。
    /// `#[serde(other)]`:未知 state 串 (后端加枚举值) 降级为健康态,
    /// 前端不 panic、不误报异常。
    #[serde(other)]
    Ok,
}

impl Default for HealthItemState {
    /// `data: null` 的信封兜底走 `Default` (未知 state 串走 `#[serde(other)]`
    /// 落到 `Ok`),两条降级路径都不误报异常态。
    fn default() -> Self {
        Self::Ok
    }
}

impl HealthItemState {
    /// 是否处于需要继续轮询的异常态 (cooling / slow_start)。
    pub fn is_abnormal(self) -> bool {
        !matches!(self, Self::Ok)
    }

    /// 中文标签 (面板徽标文案)。
    pub fn label(self) -> &'static str {
        match self {
            Self::Cooling => "冷却中",
            Self::SlowStart => "慢启动",
            Self::Ok => "正常",
        }
    }
}

/// 单条渠道健康视图 — 对齐后端 `HealthView` (camelCase)。
///
/// `channel_key` / `channel_name` / `public_model` 为 `None` 时:
/// unit 的 channel_key 已不在当前渠道目录 (被删 / 目录未同步),
/// 前端渲染占位而非造数。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthItem {
    /// 候选 key (`{channel_key}/{key_index}:{public_model}`),始终存在。
    pub unit_key: String,
    /// 渠道 UUID;join 失败为 None。
    pub channel_key: Option<String>,
    /// 渠道展示名;join 失败为 None,前端回退显示 channelKey 前 8 位。
    pub channel_name: Option<String>,
    /// 公开模型别名;join 失败为 None。
    pub public_model: Option<String>,
    /// 三态 (cooling / slow_start / ok)。
    pub state: HealthItemState,
    /// 最近一次触发冷却的 outcome (success/fatal/throttled/neutral);从未冷却 = None。
    pub last_cooling_outcome: Option<String>,
    /// 剩余冷却毫秒;未冷却 = 0。
    pub remaining_cooldown_ms: u64,
    /// 当前慢启动因子 (0.0..=1.0)。
    pub slow_start_factor: f64,
}

/// 响应体 — 对齐后端 `GatewayHealthView`。
///
/// `items` 为空 = 所有渠道健康 (正常态),面板显示虚线占位卡。
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthView {
    /// 有记录的渠道健康行 (只含有记录,正常态为空数组)。
    pub items: Vec<GatewayHealthItem>,
}

/// 拉取网关渠道健康快照:GET `/api/gateway/health` (admin 白名单)。
///
/// 复用 [`ApiClient::get`] 的信封解码:该端点返回裸
/// `{"items":[...]}` (无 `success` 外层),命中裸 JSON 分支直接解
/// `GatewayHealthView`。401 走标准刷新重试路径。
pub async fn fetch_gateway_health(client: &ApiClient) -> ApiResult<GatewayHealthView> {
    client.get("/api/gateway/health").await
}
