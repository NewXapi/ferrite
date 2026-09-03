//! # schema — 版本兼容规则与 fixtures
//!
//! ## 兼容规则 (改契约必读)
//!
//! | 变更 | 是否兼容 | 要求 |
//! |------|----------|------|
//! | 新增字段 | 兼容 | 必须带 `#[serde(default)]`, 且默认值语义正确 |
//! | 新增枚举变体 | 谨慎 | 前端必须对未知变体 fail-soft (显示 "unknown") |
//! | 删除/改名/改类型字段 | 不兼容 | 停写窗口: 旧字段先 `#[serde(skip_serializing)]` 保留一版, 双写后再删 |
//! | bump SCHEMA_VERSION | - | 仅在删字段/改类型时执行, 并登记迁移说明 |
//!
//! ## fixtures
//!
//! 供三处使用: web mock 的字段蓝本、store codec 的往返测试、console 集成测试。
//! `crates/mock` 目前持有手写假数据; 接真后端后 mock 退役, 前端 demo 模式
//! 直接引用这里的 fixtures (形状 100% 对齐契约, 不会再漂移)。

use crate::records::{
    ChannelKey, ChannelRecord, GroupRecord, RouteUnitRecord, SyncMeta, UserRecord,
};
use chrono::Utc;

fn meta(key: &str) -> SyncMeta {
    SyncMeta {
        key: key.into(),
        schema_version: crate::SCHEMA_VERSION,
        logical_version: 1,
        origin: "center".into(),
        updated_at: Utc::now(),
    }
}

/// 测试渠道 (两把 key, openai 协议)。
pub fn fixture_channel() -> ChannelRecord {
    ChannelRecord {
        meta: meta("channel/fixture-1"),
        name: "fixture-openai".into(),
        provider_type: "openai".into(),
        base_url: "https://api.openai.com".into(),
        keys: vec![
            ChannelKey { index: 0, secret: "sk-fixture-a".into(), rpm_limit: 0 },
            ChannelKey { index: 1, secret: "sk-fixture-b".into(), rpm_limit: 300 },
        ],
        max_concurrency: 10,
        status: 1,
        groups: vec!["default".into()],
        settings: serde_json::json!({}),
    }
}

/// 测试分组。
pub fn fixture_group() -> GroupRecord {
    GroupRecord {
        meta: meta("group/default"),
        id: "default".into(),
        display_name: "默认分组".into(),
        rate_multiplier: 1.0,
        allowed_models: vec![],
    }
}

/// 测试用户 (root 角色, svip 组)。
pub fn fixture_user() -> UserRecord {
    UserRecord {
        meta: meta("user/fixture-1"),
        username: "hathaway".into(),
        display_name: "海瑟薇".into(),
        email: "hathaway@wildtoken.com".into(),
        quota: 50_000_000,
        used_quota: 12_400_000,
        request_count: 18_420,
        group: "svip".into(),
        status: 1,
        role: 100,
        created_at: Utc::now(),
    }
}

/// 测试路由单元: gpt-4o → 渠道 fixture-1 / key 0 / gpt-4o-2024-08-06。
pub fn fixture_route_unit() -> RouteUnitRecord {
    RouteUnitRecord {
        meta: meta("route-unit/fixture-1"),
        group: "default".into(),
        public_model: "gpt-4o".into(),
        channel_key: "channel/fixture-1".into(),
        key_index: 0,
        upstream_model: "gpt-4o-2024-08-06".into(),
        priority: 0,
        weight: 10,
        status: 1,
    }
}

// TODO(#240): 为每个 Record 类型补 serde 往返测试 (serde_json → 原类型 → 相等),
//             保证 web/PG/Fjall 三方编解码一致。这是 02-b D0 的验收条件之一。
