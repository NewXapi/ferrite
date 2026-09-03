//! 渠道管理 — CRUD + 校验 + 探活触发。

use store::StoreError;

/// 渠道业务校验 (写前调用)。
///
/// 规则 (对齐 new-api manage_channels.go):
/// - name 非空; base_url 是合法 http(s) URL;
/// - keys 非空且 index 连续唯一; provider_type ∈ 白名单;
/// - groups 里每个 id 必须已存在 (FK 语义在应用层 — groups 是字符串键)。
/// TODO(#420): secret 加密在此层实施 (envelope encryption, 静态 key 先行)。
pub fn validate_channel(_ch: &contract::records::ChannelRecord) -> Result<(), StoreError> {
    todo!("TODO(#420): 校验规则实现")
}

/// 探活任务触发 — 渠道创建/修改后投递 ops::jobs (channel_probe)。
/// TODO(#422): job payload = channel_key + 探活模型; 结果回写 status。
pub fn schedule_probe(_channel_key: &str) {
    todo!("TODO(#422): 走 ops::JobQueue::enqueue")
}
