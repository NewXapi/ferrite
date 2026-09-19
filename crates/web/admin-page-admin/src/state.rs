//! 管理面板的共享实体 store：分组 / 模型别名 / 渠道 / 订阅套餐。
//! 拓扑图、抽屉与「设置」tab 读写同一份数据，任一侧修改立即同步。
//! 数据在应用启动时由 `hydrate()` 从真实后端灌入
//! (/api/group + /api/channel + /api/models + /api/subscriptions);
//! 订阅页的增删改由 SubscriptionsPage 直接调 `crate::api` 的订阅端点,
//! 写回成功后用返回的视图就地刷新本 store 的 plans signal。
//! 兑换码不进本 store — RedemptionsPage 直接消费 /api/redemption。
//!
//! 网络拓扑(NetworkPanel)另走自己的实时拉取路径(network.rs
//! `load_network_data`),store 侧 hydrate 结果只作为启动布局的兜底快照。

use crate::api::{SubscriptionView, list_subscriptions_api};
use client::ApiClient;
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct GroupRow {
    pub name: String,
    pub display: String,
    /// 分组倍率(≥0)
    pub multiplier: f64,
    /// 服务端 key；seed 演示行为空串（写路径此时回退 find-by-name）
    pub key: String,
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
    /// 服务端 key；seed 演示行为空串（写路径此时回退 find-by-name）
    pub key: String,
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

/// 渠道类型选项(与后端 `POST /api/admin/channels` 接受的 `channel_type` 取值一致)。
pub const CHANNEL_TYPES: &[&str] = &["openai", "openai-compat", "claude", "gemini"];

/// 订阅套餐行(对齐 new-api subscriptions 字段;各字段语义见
/// [`map_subscription_view`] 的映射注释)。
#[derive(Clone, PartialEq)]
pub struct PlanRow {
    /// 行 key(UUID)——`DELETE /api/subscriptions/{key}` 的定位符,
    /// 也是页面写回时按 key 就地刷新 signal 的依据。
    pub key: String,
    /// new-api 口径的数字 id;后端该列恒为 None,ferrite 侧无对应列。
    pub id: Option<u32>,
    /// 套餐名称(后端 name;upsert 按 name 定位,改名 = 新建一行)。
    pub title: String,
    pub subtitle: String,
    /// 售价(后端 price,NUMERIC 语义;展示用)。
    pub price: f64,
    /// 套餐额度——**展示口径**:后端入库侧 ×500_000、读回侧 ÷500_000,
    /// 本字段拿到的已是展示值,任何地方都不得再换算。
    pub quota: f64,
    /// 计价货币("CNY" | "USD")——决定价格符号 ¥/$;重建 upsert
    /// 请求体(如启停切换)时必须回传,后端拒绝空 currency。
    pub currency: String,
    pub currency_price: f64,
    pub payment_method: String,
    /// 升级分组(后端 upgrade_group;空串 = 不升级)。
    pub group: String,
    pub downgrade_group: String,
    /// 有效期数值(后端 duration_days)。
    pub period_val: u32,
    /// 有效期单位——后端恒为 "days"。
    pub period_unit: String,
    pub reset_cycle: String,
    pub priority: i32,
    pub enabled: bool,
    pub allow_redeem: bool,
    pub allow_wallet: bool,
    /// 限购(后端 max_purchases);0 = 不限。
    pub max_per_user: u32,
    pub sort_order: i32,
    pub stripe_price_id: String,
    pub creem_product_id: String,
    pub waffo_product_id: String,
}
pub const PLAN_PERIODS: &[&str] = &["month", "quarter", "year"];

/// `SubscriptionView` → [`PlanRow`] 的纯映射:hydrate 拉取与页面写回成功后
/// 共用同一份字段对齐逻辑(单测钉在 `tests/subscriptions_wire.rs`)。
///
/// 关键口径:
/// - `key` 透传(UUID),写回与删除都靠它定位行;
/// - `price` 取 f64;后端 price 是 NUMERIC 语义字符串,JSON 输出为数字;
/// - `quota` **直接取 DTO 的 f64 展示值**——后端入库侧 ×500_000、读回侧
///   ÷500_000 已对称还原,前端再乘一次会得到双倍换算的错误额度;
/// - DTO 里后端不填的列(None)统一落到展示默认值(空串 / 0 / false),
///   绝不用本地假数据回填(数据全部来自真实后端)。
pub fn map_subscription_view(v: SubscriptionView) -> PlanRow {
    PlanRow {
        key: v.key,
        id: v.id,
        title: v.name,
        subtitle: v.description.unwrap_or_default(),
        price: v.price.unwrap_or(0.0),
        quota: v.quota.unwrap_or(0.0),
        currency: if v.currency.is_empty() {
            "CNY".to_string()
        } else {
            v.currency
        },
        currency_price: v.currency_price.unwrap_or(0.0),
        payment_method: v.payment_method.unwrap_or_default(),
        group: v.group.unwrap_or_default(),
        downgrade_group: v.downgrade_group.unwrap_or_default(),
        period_val: v.period_val.unwrap_or(0),
        period_unit: v.period_unit.unwrap_or_default(),
        reset_cycle: v.reset_cycle.unwrap_or_default(),
        priority: v.priority.map(|p| p as i32).unwrap_or(0),
        enabled: v.enabled.unwrap_or(false),
        allow_redeem: v.allow_redeem.unwrap_or(false),
        allow_wallet: v.allow_wallet.unwrap_or(false),
        max_per_user: v.max_per_user.unwrap_or(0),
        sort_order: v.sort_order.map(|s| s as i32).unwrap_or(0),
        stripe_price_id: v.stripe_price_id.unwrap_or_default(),
        creem_product_id: v.creem_product_id.unwrap_or_default(),
        waffo_product_id: v.waffo_product_id.unwrap_or_default(),
    }
}

// 兑换码已从本 store 移除:RedemptionsPage 直接消费 `crate::api` 的
// /api/redemption 真实端点,不再经过 EntityStore。

#[derive(Clone, Copy)]
pub struct EntityStore {
    pub groups: Signal<Vec<GroupRow>>,
    pub aliases: Signal<Vec<AliasRow>>,
    pub channels: Signal<Vec<ChannelRow>>,
    pub plans: Signal<Vec<PlanRow>>,
}

impl EntityStore {
    pub fn seed() -> Self {
        Self {
            groups: Signal::new(vec![
                GroupRow {
                    name: "default".into(),
                    display: "默认分组".into(),
                    multiplier: 1.0,
                    key: String::new(),
                },
                GroupRow {
                    name: "claude".into(),
                    display: "Claude 专用".into(),
                    multiplier: 1.2,
                    key: String::new(),
                },
                GroupRow {
                    name: "gpt-5".into(),
                    display: "GPT-5".into(),
                    multiplier: 1.5,
                    key: String::new(),
                },
                GroupRow {
                    name: "vip".into(),
                    display: "VIP".into(),
                    multiplier: 0.8,
                    key: String::new(),
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
                    key: String::new(),
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
                    key: String::new(),
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
                    key: String::new(),
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
                    key: String::new(),
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
                    key: String::new(),
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
                    key: String::new(),
                    status: 1,
                    group: "default".into(),
                    latency_ms: Some(156),
                    candidates: vec![],
                    dispatch: vec!["gemini-2.5-pro".into()],
                },
            ]),
            // 订阅套餐数据来自真实后端:由 hydrate() 从
            // /api/subscriptions 灌入,seed 只给空列表兜底。
            plans: Signal::new(Vec::new()),
        }
    }
}

impl EntityStore {
    /// 空店：分组/别名/渠道/套餐全空,等 hydrate 灌入真实数据。
    pub fn empty() -> Self {
        Self {
            groups: Signal::new(Vec::new()),
            aliases: Signal::new(Vec::new()),
            channels: Signal::new(Vec::new()),
            plans: Signal::new(Vec::new()),
        }
    }

    /// 从真实后端灌水：分组(/api/group)、渠道(/api/channel)、
    /// 模型别名(/api/models)。渠道的 dispatch 模型直接展开自渠道
    /// 自身的 `models` JSONB,不再打已删除的 /api/route_unit。
    /// 未登录(401)时静默保持空,登录后 HomePage 重挂载会再次 hydrate。
    pub async fn hydrate(mut store: EntityStore) {
        #[derive(Debug, Default, serde::Deserialize)]
        struct Items<T> {
            #[serde(default)]
            items: Vec<T>,
        }

        let client = ApiClient::shared().clone();

        // 分组
        #[derive(Default, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct GroupDto {
            key: String,
            name: String,
            ratio: f64,
            #[serde(default)]
            remark: String,
        }
        let r: Items<GroupDto> = match client.get("/api/group").await {
            Ok(r) => r,
            Err(e) => {
                log_hydrate_error("group", &e);
                return; // 未登录/后端不可达:保持空
            }
        };
        let groups = r.items;
        store.groups.write().extend(groups.into_iter().map(|g| {
            let display = if g.remark.is_empty() {
                g.name.clone()
            } else {
                g.remark
            };
            GroupRow {
                name: g.name,
                display,
                multiplier: g.ratio,
                key: g.key,
            }
        }));

        // 渠道
        #[derive(Default, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ChannelDto {
            key: String,
            name: String,
            #[serde(default)]
            channel_type: String,
            #[serde(default)]
            base_url: String,
            status: i16,
            #[serde(default)]
            groups: Vec<String>,
            #[serde(default)]
            models: serde_json::Value,
        }
        let r: Items<ChannelDto> = match client.get("/api/channel").await {
            Ok(r) => r,
            Err(e) => {
                log_hydrate_error("channel", &e);
                return;
            }
        };
        let channels = r.items;

        let rows: Vec<ChannelRow> = channels
            .into_iter()
            .map(|c| {
                // 对外模型名来自渠道自身的 models JSONB（字符串或 {alias,upstream}），
                // 不再打已删除的 /api/route_unit。
                let models: Vec<String> = c
                    .models
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| {
                                m.as_str().map(|s| s.to_string()).or_else(|| {
                                    m.get("alias")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string())
                                })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                ChannelRow {
                    name: c.name,
                    key: c.key,
                    ctype: if c.channel_type.is_empty() {
                        "openai".into()
                    } else {
                        c.channel_type
                    },
                    url: c.base_url,
                    keys: String::new(),
                    status: if c.status == 1 { 1 } else { 0 },
                    group: if c.groups.is_empty() {
                        "default".into()
                    } else {
                        c.groups.join(",")
                    },
                    latency_ms: None,
                    candidates: models.iter().map(|m| (m.clone(), false)).collect(),
                    dispatch: models,
                }
            })
            .collect();
        store.channels.write().extend(rows);

        // 模型别名(定价三字段对齐后端 0018 列;缺省容错见 ModelDto 字段注释)
        #[derive(Default, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ModelDto {
            name: String,
            /// 输入单价(每 1k tokens);后端 0018 起恒返回,未定价 = 0。
            #[serde(default)]
            input_per_1k: f64,
            /// 输出单价(每 1k tokens);后端 0018 起恒返回,未定价 = 0。
            #[serde(default)]
            output_per_1k: f64,
            /// 价格倍率;后端 0018 起恒返回,无加价 = 1.0。
            #[serde(default = "default_hydrate_multiplier")]
            multiplier: f64,
        }
        let r: Items<ModelDto> = match client.get("/api/models?size=100").await {
            Ok(r) => r,
            Err(e) => {
                log_hydrate_error("models", &e);
                Items { items: Vec::new() }
            }
        };
        let models = r.items;
        let mut aliases: Vec<AliasRow> = models
            .into_iter()
            .map(|m| AliasRow {
                alias: m.name,
                display: String::new(),
                input_per_1k: m.input_per_1k,
                output_per_1k: m.output_per_1k,
                multiplier: m.multiplier,
            })
            .collect();
        aliases.sort_by(|a, b| a.alias.cmp(&b.alias));
        store.aliases.write().extend(aliases);

        // 订阅套餐(/api/subscriptions)。失败容忍同 models:记日志 + 保持空,
        // 不让订阅端点暂时不可用拖垮分组/渠道/别名的启动加载。
        let plans = match list_subscriptions_api(&client).await {
            Ok(items) => items.into_iter().map(map_subscription_view).collect(),
            Err(e) => {
                log_hydrate_error("subscriptions", &e);
                Vec::new()
            }
        };
        store.plans.write().extend(plans);
    }
}

/// `ModelDto.multiplier` 的反序列化缺省值 — 与后端 0018 迁移的列 DEFAULT 1.0
/// 一致(无加价);仅在字段缺席时用,正常载荷恒带真实值。
fn default_hydrate_multiplier() -> f64 {
    1.0
}

/// hydrate 失败的可见化：浏览器 console.warn（wasm 下 std eprintln 不可见）。
fn log_hydrate_error(what: &str, e: &client::ApiError) {
    #[cfg(target_arch = "wasm32")]
    web_sys::console::warn_1(&format!("EntityStore hydrate {what} failed: {e}").into());
    #[cfg(not(target_arch = "wasm32"))]
    eprintln!("EntityStore hydrate {what} failed: {e}");
    let _ = (what, e);
}
