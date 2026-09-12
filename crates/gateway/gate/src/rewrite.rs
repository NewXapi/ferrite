//! P4 渠道级 body 改写 — inject 补默认 / drop 摘字段（骨架，PR #158）。
//!
//! 对标 api-hub `_inject_body`（:257-287）的语义：**client 显式传的字段优先，
//! 注入只补默认**；`drop_fields` 是该渠道不支持、发出前摘除的字段清单（最后执行）。
//!
//! 与 api-hub 的形态差异：中转站里「渠道 A 需要注入 thinking 参数」是**渠道属性**
//! （随 [`gateway_pipeline::ctx::SelectedRoute`] 走到 forward 挂点
//! `stage.rs::build_task`），不是 api-hub 的全局 `[inject]` 默认——后者是个人脚本
//! 语义，见 todo/gateway-resilience.md「不做清单」。本模块只提供纯函数改写步，
//! 配置面（`ChannelConfig.inject` / `drop_fields`）属 contract/apps 侧，另行落地。

use std::collections::HashMap;

use bytes::Bytes;

/// 渠道级 body 改写规格（纯数据，随候选透传）。
#[derive(Debug, Clone, Default)]
pub struct InjectSpec {
    /// 注入字段：仅补 client 未显式传的键（client 值永远优先）。
    pub fields: HashMap<String, serde_json::Value>,
    /// 渠道不支持的字段名，发出前摘除；在注入之后执行。
    pub drop_fields: Vec<String>,
}

/// 对上游请求体应用 [`InjectSpec`]。
///
/// body 非 JSON object（空体 / 坏 JSON / 数组）时原样返回——改写是尽力而为的
/// 渠道适配，不该把不可解析的请求体变成 500。
pub fn rewrite_body(body: &Bytes, _spec: &InjectSpec) -> Bytes {
    // TODO(#158): 注入未显式键 + 摘除 drop_fields；非 JSON 原样返回（P4 主体实现）。
    // 骨架期语义：原样返回，不改写（尚未接线，消除 todo! 运行时 panic 坑）。
    body.clone()
}
