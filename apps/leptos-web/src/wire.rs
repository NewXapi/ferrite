//! 页面内联的网关健康 wire DTO 与三态语义色常量。
//!
//! DTO 与 `crates/web/admin-client/src/wire.rs` 同源，本 app 内联一份，
//! 后续接真实 API 时再统一；语义色常量抄自
//! `crates/web/admin-page-admin/src/tab-page-gateway/shared.rs`，
//! 保证与管理端颜色语义一致（cooling=红系 / slow_start=黄系 / ok=绿系）。
//! 本模块只放纯数据与常量：不引入 `ApiClient`、fetch 函数与 gloo-net，
//! 演示数据的构造与消费都在 `crate::pages::gateway`。

/// 渠道健康三态 — 与后端 `HealthStateView` 同源
/// （`#[serde(rename_all = "snake_case")]`：cooling / slow_start / ok）。
///
/// 前端轮询判定用它：存在 cooling / slow_start 条目才继续轮询。
#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthItemState {
    /// 冷却窗口内。
    Cooling,
    /// 冷却刚结束、ramp 渐进期。
    SlowStart,
    /// 当前健康（含历史 404/401 等 Neutral 记录但当前健康的渠道）。
    /// `#[serde(other)]`：未知 state 串（后端加枚举值）降级为健康态，
    /// 前端不 panic、不误报异常。
    #[serde(other)]
    Ok,
}

impl Default for HealthItemState {
    /// `data: null` 的信封兜底走 `Default`（未知 state 串走 `#[serde(other)]`
    /// 落到 `Ok`），两条降级路径都不误报异常态。
    fn default() -> Self {
        Self::Ok
    }
}

impl HealthItemState {
    /// 是否处于需要继续轮询的异常态（cooling / slow_start）。
    ///
    /// 演示页当前只渲染三态徽标，尚未接轮询；先保留实现供接线使用。
    #[allow(dead_code)] // 演示期无调用方，接真实 API 后由页面轮询判定调用。
    pub fn is_abnormal(self) -> bool {
        !matches!(self, Self::Ok)
    }

    /// 中文标签（面板徽标文案）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Cooling => "冷却中",
            Self::SlowStart => "慢启动",
            Self::Ok => "正常",
        }
    }
}

/// 单条渠道健康视图 — 对齐后端 `HealthView`（camelCase）。
///
/// `channel_key` / `channel_name` / `public_model` 为 `None` 时：
/// unit 的 channel_key 已不在当前渠道目录（被删 / 目录未同步），
/// 前端渲染占位而非造数。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthItem {
    /// 候选 key（`{channel_key}/{key_index}:{public_model}`），始终存在。
    pub unit_key: String,
    /// 渠道 UUID；join 失败为 None。
    pub channel_key: Option<String>,
    /// 渠道展示名；join 失败为 None，前端回退显示 channelKey 前 8 位。
    pub channel_name: Option<String>,
    /// 公开模型别名；join 失败为 None。
    pub public_model: Option<String>,
    /// 三态（cooling / slow_start / ok）。
    pub state: HealthItemState,
    /// 最近一次触发冷却的 outcome（success/fatal/throttled/neutral）；从未冷却 = None。
    pub last_cooling_outcome: Option<String>,
    /// 剩余冷却毫秒；未冷却 = 0。
    pub remaining_cooldown_ms: u64,
    /// 当前慢启动因子（0.0..=1.0）。
    ///
    /// 演示页暂未展示该指标，字段保留以对齐 wire 契约。
    #[allow(dead_code)] // 页面尚未读取；接真实 API 后由行渲染展示 ramp 进度。
    pub slow_start_factor: f64,
}

/// 响应体 — 对齐后端 `GatewayHealthView`。
///
/// `items` 为空 = 所有渠道健康（正常态），面板显示虚线占位卡。
#[derive(Debug, Clone, PartialEq, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthView {
    /// 有记录的渠道健康行（只含有记录，正常态为空数组）。
    pub items: Vec<GatewayHealthItem>,
}

// ---- 三态语义色（抄自 admin-page-admin 的 tab-page-gateway/shared.rs）----

/// cooling=红系。
pub const TONE_COOLING: &str = "border-red-500/30 bg-red-500/15 text-red-300";
/// slow_start=黄系。
pub const TONE_SLOW_START: &str = "border-amber-500/30 bg-amber-500/15 text-amber-300";
/// ok=绿系。
pub const TONE_OK: &str = "border-emerald-500/30 bg-emerald-500/15 text-emerald-400";
