//! 运行时选项 — one-api Option 表 + new-api 类型化校验的合并。
//!
//! 每个 option key 注册: 默认值 + validator + 变更钩子 (需要下发 edge 的
//! 标记 routing-visible, 变更后经 sync 域发 revision)。

use store::StoreError;

/// 选项定义 (注册表条目)。
pub struct OptionSpec {
    pub key: &'static str,
    pub default: serde_json::Value,
    /// 值域校验 (类型/范围/枚举)。
    pub validate: fn(&serde_json::Value) -> Result<(), String>,
    /// 变更是否影响路由快照 (需要推进 revision 下发 edge)。
    pub routing_visible: bool,
}

/// 首批选项 (TODO(#801) 与 new-api configure_* 对齐后扩表):
/// - gateway.retry.max_attempts / gateway.timeout.first_byte_ms (dispatch/forward)
/// - metering.cooldown_ms / metering.streak_threshold (dispatch::health defaults)
/// - observe.retention.usage_days / observe.retention.health_days
/// - billing.checkin.enabled / billing.checkin.min/max (签到额度区间)
pub fn registry() -> &'static [OptionSpec] {
    &[]
}

/// 读取 (带默认回退)。
pub async fn get<T: serde::de::DeserializeOwned>(_key: &str) -> Result<T, StoreError> {
    todo!("TODO(#801)")
}

/// 写入 (校验 → 落库 → routing_visible 则推进 revision)。
pub async fn set(_key: &str, _value: &serde_json::Value) -> Result<(), StoreError> {
    todo!("TODO(#801)")
}
