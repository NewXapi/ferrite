//! 渠道探活 handler — ops::jobs 的 channel_probe 执行侧。
//!
//! 流程 (对齐 new-api probe_channel.go, 保留 one-api 的简洁形态):
//! 1. 取渠道 + 测试模型 (渠道 settings 里指定, 缺省用其第一个 route_unit);
//! 2. 构造最小请求 (短输出, 限 max_tokens);
//! 3. 走 forward 管道真实调用 (全链路: codec/egress/计量 — 但不入用户账);
//! 4. 结果 → observe::monitor::record_probe;
//! 5. 失败连击 → 渠道自动禁用 + notify (admin)。
//!
//! 探活计费: 系统内部调用 — UsageEvent 的 user_key = 系统 (不计入任何用户)。

use store::StoreError;

/// 单渠道探活 (job payload: {channel_key, model})。
/// TODO(#804): 依赖 forward::Pipeline + observe::monitor — 接线时定 trait 参数。
pub async fn run_probe(_channel_key: &str, _model: &str) -> Result<bool, StoreError> {
    todo!("TODO(#804): 真实调用 + 结果记录 + 连击判定")
}
