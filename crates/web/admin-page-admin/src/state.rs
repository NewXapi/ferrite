//! 管理面板的共享实体 store：分组 / 模型别名 / 渠道 / 订阅套餐 / 兑换码。
//! 拓扑图、抽屉与「设置」tab 读写同一份数据，任一侧修改立即同步。
//! 目前全是 mock；将来 API crate 加载后替换初始值即可替换来源。
//!
//! 索引必须与图的 SEED_EDGES 对齐（见 network/mod.rs）：
//! 分组顺序 default/claude/gpt-5/vip，别名 gpt-4o/gpt-5/claude-sonnet-4/gemini-2.5-pro。

use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct GroupRow {
    pub name: String,
    pub display: String,
    /// 分组倍率(≥0)
    pub multiplier: f64,
}

#[derive(Clone, PartialEq)]
pub struct AliasRow {
    pub alias: String,
    pub display: String,
    /// 输入单价(¥/1k tokens,对应服务端 ModelPricing.input_per_1k)
    pub input_per_1k: f64,
    /// 输出单价(¥/1k tokens,对应服务端 ModelPricing.output_per_1k)
    pub output_per_1k: f64,
    /// 计费倍率(≥0,0 = 免费)
    pub multiplier: f64,
}

#[derive(Clone, PartialEq)]
pub struct ChannelRow {
    pub name: String,
    /// 渠道类型,取值见 CHANNEL_TYPES(镜像 api 的 CHANNEL_TYPES)
    pub ctype: String,
    pub url: String,
    pub keys: String,
    /// 状态:1 启用 / 0 手动停用 / 2 自动停用(测速失败)
    pub status: u8,
    /// 所属分组(默认 default)
    pub group: String,
    /// 最近一次测速毫秒数;None = 未测
    pub latency_ms: Option<u32>,
    /// 拉取回来、尚未进入拓扑的候补：(模型名, 是否勾选)
    pub candidates: Vec<(String, bool)>,
    /// 已加入拓扑的调度模型；名字来自上游，不可改
    pub dispatch: Vec<String>,
}

/// 渠道类型选项,镜像 `apps/api/src/gateway.rs` 的 CHANNEL_TYPES。
pub const CHANNEL_TYPES: &[&str] = &["openai", "openai-compat", "claude", "gemini"];

/// 订阅套餐(对齐 new-api subscriptions 字段)
#[derive(Clone, PartialEq)]
pub struct PlanRow {
    pub id: u32,
    pub title: String,
    pub subtitle: String,
    pub price: f64,
    pub quota: f64,
    pub currency_price: f64,
    pub payment_method: String,
    pub group: String,
    pub downgrade_group: String,
    pub period_val: u32,
    pub period_unit: String,
    pub reset_cycle: String,
    pub priority: i32,
    pub enabled: bool,
    pub allow_redeem: bool,
    pub allow_wallet: bool,
    pub max_per_user: u32,
    pub sort_order: i32,
    pub stripe_price_id: String,
    pub creem_product_id: String,
    pub waffo_product_id: String,
}
pub const PLAN_PERIODS: &[&str] = &["month", "quarter", "year"];

/// 兑换码(对齐 new-api redemption 的字段子集)
/// 状态:1 未用 / 2 停用 / 3 已用
#[derive(Clone, PartialEq)]
pub struct RedRow {
    pub name: String,
    pub key: String,
    /// 额度(¥ 等值)
    pub quota: f64,
    pub status: u8,
    pub created: String,
    pub expired: String,
}

#[derive(Clone, Copy)]
pub struct EntityStore {
    pub groups: Signal<Vec<GroupRow>>,
    pub aliases: Signal<Vec<AliasRow>>,
    pub channels: Signal<Vec<ChannelRow>>,
    pub plans: Signal<Vec<PlanRow>>,
    pub redemptions: Signal<Vec<RedRow>>,
}

impl EntityStore {
    pub fn seed() -> Self {
        Self {
            groups: Signal::new(vec![
                GroupRow {
                    name: "default".into(),
                    display: "默认分组".into(),
                    multiplier: 1.0,
                },
                GroupRow {
                    name: "claude".into(),
                    display: "Claude 专用".into(),
                    multiplier: 1.2,
                },
                GroupRow {
                    name: "gpt-5".into(),
                    display: "GPT-5".into(),
                    multiplier: 1.5,
                },
                GroupRow {
                    name: "vip".into(),
                    display: "VIP".into(),
                    multiplier: 0.8,
                },
            ]),
            aliases: Signal::new(vec![
                AliasRow {
                    alias: "gpt-4o".into(),
                    display: "GPT-4o".into(),
                    input_per_1k: 0.0175,
                    output_per_1k: 0.07,
                    multiplier: 1.0,
                },
                AliasRow {
                    alias: "gpt-5".into(),
                    display: "GPT-5".into(),
                    input_per_1k: 0.035,
                    output_per_1k: 0.28,
                    multiplier: 1.0,
                },
                AliasRow {
                    alias: "claude-sonnet-4".into(),
                    display: "Claude Sonnet 4".into(),
                    input_per_1k: 0.021,
                    output_per_1k: 0.105,
                    multiplier: 1.0,
                },
                AliasRow {
                    alias: "gemini-2.5-pro".into(),
                    display: "Gemini 2.5 Pro".into(),
                    input_per_1k: 0.00875,
                    output_per_1k: 0.07,
                    multiplier: 1.0,
                },
            ]),
            channels: Signal::new(vec![
                ChannelRow {
                    name: "OpenAI 官方".into(),
                    ctype: "openai".into(),
                    url: "https://api.openai.com/v1".into(),
                    keys: "sk-**************************".into(),
                    status: 1,
                    group: "default".into(),
                    latency_ms: Some(186),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into(), "gpt-5".into()],
                },
                ChannelRow {
                    name: "Azure East".into(),
                    ctype: "openai-compat".into(),
                    url: "https://east.azure.example/openai".into(),
                    keys: "az-****".into(),
                    status: 1,
                    group: "default".into(),
                    latency_ms: Some(243),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into()],
                },
                ChannelRow {
                    name: "OneAPI 上游".into(),
                    ctype: "openai-compat".into(),
                    url: "https://oneapi.example/v1".into(),
                    keys: "oa-****".into(),
                    status: 1,
                    group: "default".into(),
                    latency_ms: Some(312),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into(), "gpt-5".into(), "claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "Claude 官网".into(),
                    ctype: "claude".into(),
                    url: "https://api.anthropic.com".into(),
                    keys: "ak-****".into(),
                    status: 1,
                    group: "claude".into(),
                    latency_ms: Some(298),
                    candidates: vec![],
                    dispatch: vec!["claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "AWS Bedrock".into(),
                    ctype: "openai-compat".into(),
                    url: "https://bedrock.us-east-1.amazonaws.com".into(),
                    keys: "aws-****".into(),
                    status: 2,
                    group: "claude".into(),
                    latency_ms: None,
                    candidates: vec![],
                    dispatch: vec!["claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "Gemini".into(),
                    ctype: "gemini".into(),
                    url: "https://generativelanguage.googleapis.com".into(),
                    keys: "gm-****".into(),
                    status: 1,
                    group: "default".into(),
                    latency_ms: Some(156),
                    candidates: vec![],
                    dispatch: vec!["gemini-2.5-pro".into()],
                },
            ]),
            plans: Signal::new(vec![
                PlanRow {
                    id: 5,
                    title: "开拓的封赏".into(),
                    subtitle: "向你们致敬，向外开拓的勇士们！".into(),
                    price: 0.0,
                    quota: 0.0,
                    currency_price: 0.0,
                    payment_method: "无限制".into(),
                    group: "不升级".into(),
                    downgrade_group: "降级到购买前分组".into(),
                    period_val: 6,
                    period_unit: "小时".into(),
                    reset_cycle: "不重置".into(),
                    priority: 0,
                    enabled: true,
                    allow_redeem: true,
                    allow_wallet: true,
                    max_per_user: 0,
                    sort_order: 0,
                    stripe_price_id: "".into(),
                    creem_product_id: "".into(),
                    waffo_product_id: "".into(),
                },
                PlanRow {
                    id: 4,
                    title: "重置分组".into(),
                    subtitle: "直接重置回原始分组，避免由于模型分组原因导致不可使用".into(),
                    price: 0.0,
                    quota: 0.01,
                    currency_price: 0.01,
                    payment_method: "仅扣菌种".into(),
                    group: "default".into(),
                    downgrade_group: "降级到购买前分组".into(),
                    period_val: 1,
                    period_unit: "秒".into(),
                    reset_cycle: "不重置".into(),
                    priority: 0,
                    enabled: true,
                    allow_redeem: true,
                    allow_wallet: true,
                    max_per_user: 0,
                    sort_order: 0,
                    stripe_price_id: "".into(),
                    creem_product_id: "".into(),
                    waffo_product_id: "".into(),
                },
                PlanRow {
                    id: 3,
                    title: "“杰瑞”的牛奶".into(),
                    subtitle: "我的牛奶！不会有下次了！！！".into(),
                    price: 1.0,
                    quota: 10.0,
                    currency_price: 10.0,
                    payment_method: "仅扣菌种".into(),
                    group: "vip".into(),
                    downgrade_group: "降级到购买前分组".into(),
                    period_val: 1,
                    period_unit: "小时".into(),
                    reset_cycle: "不重置".into(),
                    priority: 0,
                    enabled: true,
                    allow_redeem: true,
                    allow_wallet: true,
                    max_per_user: 0,
                    sort_order: 0,
                    stripe_price_id: "".into(),
                    creem_product_id: "".into(),
                    waffo_product_id: "".into(),
                },
                PlanRow {
                    id: 2,
                    title: "大老鼠".into(),
                    subtitle: "享受一折优惠！".into(),
                    price: 100.0,
                    quota: 150.0,
                    currency_price: 150.0,
                    payment_method: "仅扣菌种".into(),
                    group: "svip".into(),
                    downgrade_group: "降级到购买前分组".into(),
                    period_val: 1,
                    period_unit: "个月".into(),
                    reset_cycle: "不重置".into(),
                    priority: 0,
                    enabled: true,
                    allow_redeem: true,
                    allow_wallet: true,
                    max_per_user: 0,
                    sort_order: 0,
                    stripe_price_id: "price_1OvXx8...".into(),
                    creem_product_id: "prod_creem_lar...".into(),
                    waffo_product_id: "".into(),
                },
                PlanRow {
                    id: 1,
                    title: "小老鼠".into(),
                    subtitle: "享受半价优惠".into(),
                    price: 50.0,
                    quota: 50.0,
                    currency_price: 50.0,
                    payment_method: "仅扣菌种".into(),
                    group: "vip".into(),
                    downgrade_group: "降级到购买前分组".into(),
                    period_val: 14,
                    period_unit: "天".into(),
                    reset_cycle: "不重置".into(),
                    priority: 0,
                    enabled: false,
                    allow_redeem: true,
                    allow_wallet: true,
                    max_per_user: 0,
                    sort_order: 0,
                    stripe_price_id: "price_1OvYy2...".into(),
                    creem_product_id: "".into(),
                    waffo_product_id: "".into(),
                },
            ]),
            redemptions: Signal::new(vec![
                RedRow {
                    name: "内测福利".into(),
                    key: "BETA-3F2A".into(),
                    quota: 50.0,
                    status: 1,
                    created: "2026-08-30".into(),
                    expired: "永不过期".into(),
                },
                RedRow {
                    name: "内测福利".into(),
                    key: "BETA-9C14".into(),
                    quota: 50.0,
                    status: 3,
                    created: "2026-08-30".into(),
                    expired: "永不过期".into(),
                },
                RedRow {
                    name: "活动码".into(),
                    key: "ACTV-77D1".into(),
                    quota: 10.0,
                    status: 2,
                    created: "2026-08-25".into(),
                    expired: "2026-09-15".into(),
                },
                RedRow {
                    name: "活动码".into(),
                    key: "ACTV-08E2".into(),
                    quota: 10.0,
                    status: 1,
                    created: "2026-08-25".into(),
                    expired: "2026-09-15".into(),
                },
            ]),
        }
    }
}
