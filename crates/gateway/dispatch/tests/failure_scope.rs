//! P1-B 渠道相关性分类场景登记（#158 骨架，P1-B 落地后已填断言）。
//!
//! 预期：401/403（凭据）、404（该渠道无此模型）→ [`FailureScope::Channel`]——
//! 换渠道可能成立；400/413/422（请求本身坏）→ [`FailureScope::Request`]——
//! 换谁都是失败。5xx/429 不归 `classify_channel_scope`：可重试分类已由
//! `forward::egress::classify_status` 判定，本函数只吃 4xx。
//!
//! 降层在重试循环里的行为（switch / 最后一个候选才透传 / Neutral 回报）见
//! `channel_downgrade.rs`；端到端双渠道降层见
//! `apps/gateway/tests/route_resolution.rs` P0-2。

use dispatch::{FailureScope, classify_channel_scope};

#[test]
fn channel_scoped_4xx_maps_to_channel() {
    for status in [401u16, 403, 404] {
        assert_eq!(
            classify_channel_scope(status),
            FailureScope::Channel,
            "{status} 换渠道可能成立，应判 Channel"
        );
    }
}

#[test]
fn request_scoped_4xx_maps_to_request() {
    for status in [400u16, 413, 422] {
        assert_eq!(
            classify_channel_scope(status),
            FailureScope::Request,
            "{status} 请求本身坏，换渠道无效，应判 Request"
        );
    }
}

#[test]
#[ignore = "TODO(#158): 骨架未实现"]
fn policy_for_hits_per_model_then_default() {
    todo!("TODO(#158): ModelRetryPolicies::policy_for 精确命中 per_model，否则回 default")
}
