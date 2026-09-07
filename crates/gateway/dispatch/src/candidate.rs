//! 候选产物 — 选路结果的完整可执行目标。

use contract::records::{ChannelRecord, RouteUnitRecord};

/// 路由单元启用状态 (对齐 ChannelRecord::status: 1 启用 / 2 手动禁用 / 3 自动熔断)。
/// dispatch 只选 status == 1 的单元; 自动熔断由 health 内存表本地判定,
/// 与 center 的汇总熔断 (status=3) 互不干扰。
pub const STATUS_ENABLED: u8 = 1;

/// dispatch 的最终产出: 一个可转发的具体目标。
///
/// 权威定义在 `gateway_pipeline::ctx::SelectedRoute` —— 路由产物要经 `RequestCtx`
/// 跨 stage 传递, 而 pipeline 是全部 stage crate 的公共基座, 反向不依赖 dispatch。
/// 本别名保留 `dispatch::Candidate` 这个语义名字, 两者是同一个类型。
pub type Candidate = gateway_pipeline::ctx::SelectedRoute;

/// 路由单元 + 渠道快照 → 完整候选。
///
/// 凭据取自 `channel.keys` 中 `index == unit.key_index` 的那把 key
/// (new-api `GetNextEnabledKeyForIndex` 的语义: key 按下标解析, 不轮询)。
/// key 缺失或越界 → None, 上层按 NoCandidate 处理。
///
/// `provider_type` 与 `settings` 原样取自 `channel`: 选路时一次解析完,
/// forward 就不必为了拿协议族与渠道覆盖头回头查 catalog 快照。
pub fn resolve_candidate(unit: &RouteUnitRecord, channel: &ChannelRecord) -> Option<Candidate> {
    let key = channel.keys.iter().find(|k| k.index == unit.key_index)?;
    Some(Candidate {
        unit: unit.clone(),
        secret: key.secret.clone(),
        base_url: channel.base_url.clone(),
        upstream_model: unit.upstream_model.clone(),
        provider_type: channel.provider_type.clone(),
        settings: channel.settings.clone(),
    })
}
