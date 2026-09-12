//! P1-B 渠道相关性分类场景登记（#158 骨架，实现落地后去掉 #[ignore] 并填断言）。
//!
//! 预期：401/403（凭据）、404（该渠道无此模型）→ [`FailureScope::Channel`]——
//! 换渠道可能成立；400/413/422（请求本身坏）→ [`FailureScope::Request`]——
//! 换谁都是失败。5xx/429 不归 `classify_channel_scope`：可重试分类已由
//! `forward::egress::classify_status` 判定，本函数只吃 4xx。

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn channel_scoped_4xx_maps_to_channel() {
    todo!("TODO(#158): classify_channel_scope(401|403|404) == FailureScope::Channel")
}

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn request_scoped_4xx_maps_to_request() {
    todo!("TODO(#158): classify_channel_scope(400|413|422) == FailureScope::Request")
}

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn fatal_but_switchable_keeps_health_neutral_and_switches_candidate() {
    todo!("TODO(#158): 首候选 FatalButSwitchable → mark_tried 换第二候选；健康记 Neutral")
}

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn last_candidate_fatal_but_switchable_passes_through() {
    todo!("TODO(#158): 最后一个候选 FatalButSwitchable → 透传给客户端（P1-B 实现后补断言）")
}

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn policy_for_hits_per_model_then_default() {
    todo!("TODO(#158): ModelRetryPolicies::policy_for 精确命中 per_model，否则回 default")
}
