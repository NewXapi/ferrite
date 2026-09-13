//! 网关渠道健康端点 — `/api/gateway/health`。
//!
//! 数据面（#158 outcome-driven cooldown）在运行期实时更新
//! [`dispatch::MemoryHealthTable`] 的冷却 / 慢启动 / 失败分类，但此前零查询面。
//! 本模块把该内存状态暴露成 admin 只读 JSON，作为 web 网关面板（PR-9）的数据源。
//!
//! 语义约定（与数据面严格区分）：
//! - **响应只含有记录的 unit**：健康表从未 record 过的渠道（或已自然恢复、
//!   被惰性结算清出冷却的）默认不出现，避免几百条 "ok" 噪音；
//! - 读路径**不结算**过期冷却（`entries()` 无副作用）—— 到期的冷却在面板上
//!   仍显示为剩余 0ms，下一次热路径选择才把它结算进 slow-start ramp。
//!   这是刻意的：查询面不产生数据面副作用；
//! - 快照 join 失败（unit 已从渠道目录删除）的条目保留，但展示名 / 模型名降级为 null。

use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::HeaderMap;
use dispatch::Snapshot;
use dispatch::health::{ChannelOutcome, HealthState};
use serde::Serialize;

use auth::error::AuthError;
use auth::routes::{ADMIN_ROLE_THRESHOLD, bearer_user};
use auth::service::AuthService;

/// 健康状态视图 — 三态枚举，序列化按 snake_case 转字符串（cooling / slow_start / ok）。
///
/// 用类型而非 `&'static str`：调用方拼写错误编译期即暴露，API 自文档化。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStateView {
    /// 冷却窗口内。
    Cooling,
    /// 冷却刚结束、ramp 渐进期。
    SlowStart,
    /// 当前健康。
    Ok,
}

impl HealthStateView {
    /// 序列化后的字符串形态（测试断言用；与 `#[serde(rename_all = "snake_case")]` 同源）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Cooling => "cooling",
            Self::SlowStart => "slow_start",
            Self::Ok => "ok",
        }
    }
}

/// 单条渠道健康视图 — join 后的面板数据行。
///
/// 字段为 Option 时：unit 的 channel_key 已不在当前快照 channels 中（渠道
/// 被删 / 目录未同步），不造数，前端渲染占位。
#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthView {
    /// 候选 key（RouteUnitRecord::meta.key，形如 `{channel_key}/{key_index}:{public_model}`）。
    pub unit_key: String,
    /// 渠道 UUID（join 自快照 units；unit 已从目录移除时为 None）。
    pub channel_key: Option<String>,
    /// 渠道展示名（join 自快照 channels；同上可缺省）。
    pub channel_name: Option<String>,
    /// 公开模型别名（join 自快照 units；同上可缺省）。
    pub public_model: Option<String>,
    /// 三态，取值见 [`HealthStateView`]（序列化为 snake_case 字符串）。
    pub state: HealthStateView,
    /// 最近一次触发冷却的 outcome（P1-A 分档依据）；从未冷却 = None。
    pub last_cooling_outcome: Option<String>,
    /// 剩余冷却毫秒；未冷却 = 0。读路径惰性结算，到期待结算时为 0 而非负数。
    pub remaining_cooldown_ms: u64,
    /// 当前慢启动因子（0.0..=1.0）。
    pub slow_start_factor: f64,
}

/// 响应体。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayHealthView {
    pub items: Vec<HealthView>,
}

/// handler state — 与 `apps/api` 的 `ReloadState` 同款 Clone 纪律：
/// 每个 Arc 都是进程级单例句柄，Clone 仅复制引用计数，绝不深拷贝数据。
/// `dispatcher` / `health` 与数据面（ForwardStage 重试循环 + reload 路由）
/// 共享同一批 `Arc`，reload / 运行期更新后本端点读到的永远是最新状态。
#[derive(Clone)]
pub struct GatewayHealthState {
    /// 渠道目录快照句柄（`Dispatcher` 内部 ArcSwap，`load_full` 零拷贝）。
    pub dispatcher: Arc<dispatch::Dispatcher>,
    /// 运行期实时健康表（#158）。
    pub health: Arc<dispatch::MemoryHealthTable>,
    /// admin 鉴权（bearer_user + 角色白名单，与 monitor 一致）。
    pub auth: Arc<AuthService>,
}

/// GET /api/gateway/health — admin 角色白名单（照抄 monitor 的 require_admin 模式）。
pub fn router(state: GatewayHealthState) -> axum::Router {
    use axum::routing::get;
    axum::Router::new()
        .route("/api/gateway/health", get(gateway_health_handler))
        .with_state(state)
}

fn err_json(e: AuthError) -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        e.status(),
        Json(serde_json::json!({ "code": e.code(), "message": e.to_string() })),
    )
}

async fn require_admin(auth: &AuthService, headers: &HeaderMap) -> Result<(), AuthError> {
    let user = bearer_user(auth, headers).await?;
    if user.role >= ADMIN_ROLE_THRESHOLD {
        Ok(())
    } else {
        Err(AuthError::Forbidden)
    }
}

/// handler 主体：纯函数 [`build_health_view`] 做 join，鉴权照抄 monitor。
async fn gateway_health_handler(
    State(state): State<GatewayHealthState>,
    headers: HeaderMap,
) -> Result<Json<GatewayHealthView>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    require_admin(&state.auth, &headers)
        .await
        .map_err(err_json)?;

    let now_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let entries = state.health.entries();
    let snapshot = state.dispatcher.snapshot();
    let items = build_health_view(&entries, snapshot.as_deref(), now_ms);
    Ok(Json(GatewayHealthView { items }))
}

/// 纯函数：健康表 entries join 目录快照，产出面板数据行。
///
/// 规则：
/// - 只保留"有记录"的 unit（见模块文档）；
/// - `state` 判定：`is_cooling(now)` → `cooling`；`ramp_pending` →
///   `slow_start`；其余 → `ok`（历史 404/401 等 Neutral 记录但当前健康）；
/// - join 不到的字段（unit 已不在目录）降为 None，条目本身保留。
pub fn build_health_view(
    entries: &[(String, HealthState)],
    snapshot: Option<&Snapshot>,
    now_ms: u64,
) -> Vec<HealthView> {
    let mut items = Vec::with_capacity(entries.len());
    for (unit_key, st) in entries {
        // join：从快照 units 里按 meta.key 反查渠道 / 模型（O(n) 一次，
        // 面板低频调用可接受；若将来 entries 上量再上 HashMap 索引）。
        let (channel_key, channel_name, public_model) = match snapshot {
            Some(s) => {
                let unit = s
                    .units
                    .iter()
                    .find(|u| u.meta.key == *unit_key)
                    .map(|u| u.channel_key.clone())
                    .and_then(|ck| {
                        s.channels
                            .get(&ck)
                            .map(|c| (Some(ck), Some(c.name.clone())))
                    })
                    .unwrap_or((None, None));
                let model = s
                    .units
                    .iter()
                    .find(|u| u.meta.key == *unit_key)
                    .map(|u| u.public_model.clone());
                (unit.0, unit.1, model)
            }
            None => (None, None, None),
        };

        let state = if st.is_cooling(now_ms) {
            HealthStateView::Cooling
        } else if st.ramp_pending {
            HealthStateView::SlowStart
        } else {
            HealthStateView::Ok
        };

        let remaining = if st.is_cooling(now_ms) {
            st.cooldown_until_ms.saturating_sub(now_ms)
        } else {
            0
        };

        items.push(HealthView {
            unit_key: unit_key.clone(),
            channel_key,
            channel_name,
            public_model,
            state,
            last_cooling_outcome: st
                .last_cooling_outcome
                .map(|o| outcome_label(o).to_string()),
            remaining_cooldown_ms: remaining,
            // 读路径无 cfg 注入，min_requests 取 0 → 因子恒 1.0（面板只展示，
            // 不重算渐进权重；真实 ramp 因子由数据面按 cfg.min_requests 计算）。
            slow_start_factor: st.slow_start_factor(0),
        });
    }
    items
}

/// outcome → 面板短标签（返回 `&'static str`，序列化方按需用 `.to_string()` 转堆）。
fn outcome_label(o: ChannelOutcome) -> &'static str {
    match o {
        ChannelOutcome::Success => "success",
        ChannelOutcome::Fatal => "fatal",
        ChannelOutcome::Throttled => "throttled",
        ChannelOutcome::Neutral => "neutral",
    }
}
