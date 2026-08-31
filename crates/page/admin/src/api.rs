//! 管理面板的数据来源：DTO 定义 + mock 后端适配器。
//!
//! `fetch_admin_data()` 现在同步返回内置 mock；接上真实后端后只改本文件，
//! `state::EntityStore` 与各面板都不用动。
//!
//! 图边按名字派生：别名与调度模型同名即连接，分组→别名见 `group_alias_links`。

#[derive(Clone, PartialEq)]
pub struct GroupRow {
    pub name: String,
    pub display: String,
    /// 禁用的分组不参与路由
    pub enabled: bool,
    pub description: String,
}

#[derive(Clone, PartialEq)]
pub struct AliasRow {
    pub alias: String,
    pub display: String,
}

#[derive(Clone, PartialEq)]
pub struct ChannelRow {
    pub name: String,
    pub url: String,
    pub keys: String,
    /// 拉取回来、尚未进入拓扑的候补:(模型名, 是否勾选)
    pub candidates: Vec<(String, bool)>,
    /// 已加入拓扑的调度模型;名字来自上游,不可改
    pub dispatch: Vec<String>,
    /// 禁用的渠道不参与调度
    pub enabled: bool,
    /// 分组归属(引用 groups.name)
    pub groups: Vec<String>,
    pub remark: String,
}

/// 模型对外展示元数据。alias 是主键,来自渠道 dispatch 并集,不由本存储创建。
#[derive(Clone, PartialEq)]
pub struct PresentationRow {
    pub alias: String,
    /// 展示名,空则回落到 alias
    pub display_name: String,
    pub description: String,
    /// 封面/图标:URL 或内置图标 id,空为无
    pub icon: String,
    /// 角标:"" / "new" / "beta" / "推荐" 等
    pub badge: String,
    pub tags: Vec<String>,
    pub sort_order: i32,
    /// 隐藏的模型仍可路由,只是不在前台展示
    pub visible: bool,
}

/// API key(控制台密钥的 mock),一个 key 归属一个分组。
#[derive(Clone, PartialEq)]
pub struct KeyRow {
    pub name: String,
    pub masked: String,
    /// 归属分组(引用 groups.name)
    pub group: String,
    pub enabled: bool,
}

/// 分组倍率:每个分组一个系数。
#[derive(Clone, PartialEq)]
pub struct GroupRatioRow {
    pub group: String,
    pub ratio: f64,
}

/// 用户分组 × 使用分组 交叉倍率。只存已配置的交叉项,不渲染完整矩阵。
#[derive(Clone, PartialEq)]
pub struct CrossRatioRow {
    pub user_group: String,
    pub use_group: String,
    pub ratio: f64,
}

/// 模型定价,美元 / 1M tokens。
#[derive(Clone, PartialEq)]
pub struct ModelPriceRow {
    pub alias: String,
    pub input_per_m: f64,
    pub output_per_m: f64,
}

/// 速率限制规则。
#[derive(Clone, PartialEq)]
pub struct RateRuleRow {
    pub name: String,
    /// 作用域描述,如 "所有分组" / "vip"
    pub scope: String,
    pub limit: u32,
    /// 窗口描述,如 "每分钟"
    pub window: String,
    pub enabled: bool,
}

/// 一次拉取回来的全部管理数据。
#[derive(Clone, PartialEq)]
pub struct AdminData {
    pub groups: Vec<GroupRow>,
    pub aliases: Vec<AliasRow>,
    pub channels: Vec<ChannelRow>,
    pub presentations: Vec<PresentationRow>,
    pub api_keys: Vec<KeyRow>,
    pub group_ratios: Vec<GroupRatioRow>,
    pub cross_ratios: Vec<CrossRatioRow>,
    pub model_prices: Vec<ModelPriceRow>,
    pub rate_rules: Vec<RateRuleRow>,
    pub banned_words: Vec<String>,
    pub announcement: String,
    pub system_facts: Vec<(String, String)>,
    /// 分组→模型别名关系；图层边由此和渠道 dispatch 派生。
    pub group_alias_links: Vec<(String, String)>,
}

/// mock 后端：同步返回整份管理数据。
// ponytail: 同步 mock，接真实 API 时改成 async fetch 即可
pub fn fetch_admin_data() -> AdminData {
    AdminData {
        groups: vec![
            GroupRow {
                name: "default".into(),
                display: "默认分组".into(),
                enabled: true,
                description: String::new(),
            },
            GroupRow {
                name: "claude".into(),
                display: "Claude 专用".into(),
                enabled: true,
                description: String::new(),
            },
            GroupRow {
                name: "gpt-5".into(),
                display: "GPT-5".into(),
                enabled: true,
                description: String::new(),
            },
            GroupRow {
                name: "vip".into(),
                display: "VIP".into(),
                enabled: true,
                description: String::new(),
            },
            GroupRow {
                name: "free".into(),
                display: "免费池".into(),
                enabled: true,
                description: String::new(),
            },
            GroupRow {
                name: "test".into(),
                display: "灰度测试".into(),
                enabled: true,
                description: String::new(),
            },
        ],
        aliases: vec![
            AliasRow {
                alias: "gpt-4o".into(),
                display: "GPT-4o".into(),
            },
            AliasRow {
                alias: "gpt-4o-mini".into(),
                display: "GPT-4o mini".into(),
            },
            AliasRow {
                alias: "gpt-5".into(),
                display: "GPT-5".into(),
            },
            AliasRow {
                alias: "gpt-5-codex".into(),
                display: "GPT-5 Codex".into(),
            },
            AliasRow {
                alias: "claude-sonnet-4".into(),
                display: "Claude Sonnet 4".into(),
            },
            AliasRow {
                alias: "claude-opus-4.1".into(),
                display: "Claude Opus 4.1".into(),
            },
            AliasRow {
                alias: "claude-haiku-3.5".into(),
                display: "Claude Haiku 3.5".into(),
            },
            AliasRow {
                alias: "gemini-2.5-pro".into(),
                display: "Gemini 2.5 Pro".into(),
            },
            AliasRow {
                alias: "gemini-2.5-flash".into(),
                display: "Gemini 2.5 Flash".into(),
            },
            AliasRow {
                alias: "gemini-3-pro".into(),
                display: "Gemini 3 Pro".into(),
            },
            AliasRow {
                alias: "deepseek-v3.2".into(),
                display: "DeepSeek V3.2".into(),
            },
            AliasRow {
                alias: "deepseek-r1".into(),
                display: "DeepSeek R1".into(),
            },
            AliasRow {
                alias: "qwen3-max".into(),
                display: "Qwen3 Max".into(),
            },
            AliasRow {
                alias: "qwen3-coder-plus".into(),
                display: "Qwen3 Coder Plus".into(),
            },
            AliasRow {
                alias: "grok-4".into(),
                display: "Grok 4".into(),
            },
            AliasRow {
                alias: "kimi-k2".into(),
                display: "Kimi K2".into(),
            },
            AliasRow {
                alias: "glm-4.6".into(),
                display: "GLM 4.6".into(),
            },
        ],
        channels: vec![
            ChannelRow {
                name: "OpenAI 官方".into(),
                url: "https://api.openai.com/v1".into(),
                keys: "sk-**************************".into(),
                candidates: vec![("o3".into(), true), ("gpt-5-pro".into(), false)],
                dispatch: vec![
                    "gpt-4o".into(),
                    "gpt-4o-mini".into(),
                    "gpt-5".into(),
                    "gpt-5-codex".into(),
                ],
                enabled: true,
                groups: vec!["default".into(), "gpt-5".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "Azure East".into(),
                url: "https://east.azure.example/openai".into(),
                keys: "az-****".into(),
                candidates: vec![],
                dispatch: vec!["gpt-4o".into(), "gpt-4o-mini".into(), "gpt-5".into()],
                enabled: true,
                groups: vec!["default".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "OneAPI 上游".into(),
                url: "https://oneapi.example/v1".into(),
                keys: "oa-****".into(),
                candidates: vec![],
                dispatch: vec![
                    "gpt-4o".into(),
                    "gpt-5".into(),
                    "claude-sonnet-4".into(),
                    "deepseek-v3.2".into(),
                    "qwen3-max".into(),
                ],
                enabled: true,
                groups: vec!["default".into(), "vip".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "Claude 官网".into(),
                url: "https://api.anthropic.com".into(),
                keys: "ak-****".into(),
                candidates: vec![("claude-opus-4.5".into(), false)],
                dispatch: vec![
                    "claude-sonnet-4".into(),
                    "claude-opus-4.1".into(),
                    "claude-haiku-3.5".into(),
                ],
                enabled: true,
                groups: vec!["claude".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "AWS Bedrock".into(),
                url: "https://bedrock.us-east-1.amazonaws.com".into(),
                keys: "aws-****".into(),
                candidates: vec![],
                dispatch: vec!["claude-sonnet-4".into(), "claude-haiku-3.5".into()],
                enabled: false,
                groups: vec!["claude".into()],
                remark: "账单未结算".into(),
            },
            ChannelRow {
                name: "Gemini".into(),
                url: "https://generativelanguage.googleapis.com".into(),
                keys: "gm-****".into(),
                candidates: vec![],
                dispatch: vec![
                    "gemini-2.5-pro".into(),
                    "gemini-2.5-flash".into(),
                    "gemini-3-pro".into(),
                ],
                enabled: true,
                groups: vec!["default".into(), "vip".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "DeepSeek 官方".into(),
                url: "https://api.deepseek.com/v1".into(),
                keys: "ds-****".into(),
                candidates: vec![("deepseek-v4".into(), false)],
                dispatch: vec!["deepseek-v3.2".into(), "deepseek-r1".into()],
                enabled: true,
                groups: vec!["default".into(), "free".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "阿里百炼".into(),
                url: "https://dashscope.aliyuncs.com/compatible-mode/v1".into(),
                keys: "ali-****".into(),
                candidates: vec![],
                dispatch: vec![
                    "qwen3-max".into(),
                    "qwen3-coder-plus".into(),
                    "deepseek-v3.2".into(),
                    "glm-4.6".into(),
                ],
                enabled: true,
                groups: vec!["default".into(), "free".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "xAI 官方".into(),
                url: "https://api.x.ai/v1".into(),
                keys: "xai-****".into(),
                candidates: vec![("grok-4-fast".into(), true)],
                dispatch: vec!["grok-4".into()],
                enabled: true,
                groups: vec!["vip".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "聚合中转 A".into(),
                url: "https://relay-a.example/v1".into(),
                keys: "zz-****".into(),
                candidates: vec![("llama-4-maverick".into(), false)],
                dispatch: vec![
                    "gpt-4o".into(),
                    "gpt-4o-mini".into(),
                    "claude-sonnet-4".into(),
                    "gemini-2.5-flash".into(),
                    "deepseek-v3.2".into(),
                    "qwen3-max".into(),
                    "kimi-k2".into(),
                    "glm-4.6".into(),
                ],
                enabled: true,
                groups: vec!["default".into(), "free".into()],
                remark: String::new(),
            },
            ChannelRow {
                name: "聚合中转 B".into(),
                url: "https://relay-b.example/v1".into(),
                keys: "zz2-****".into(),
                candidates: vec![],
                dispatch: vec![
                    "gpt-4o".into(),
                    "gpt-5".into(),
                    "claude-opus-4.1".into(),
                    "gemini-2.5-pro".into(),
                    "deepseek-r1".into(),
                    "qwen3-coder-plus".into(),
                    "grok-4".into(),
                    "kimi-k2".into(),
                ],
                enabled: true,
                groups: vec!["vip".into()],
                remark: "按量计费,价格浮动".into(),
            },
            ChannelRow {
                name: "SiliconFlow".into(),
                url: "https://api.siliconflow.cn/v1".into(),
                keys: "sf-****".into(),
                candidates: vec![],
                dispatch: vec![
                    "deepseek-v3.2".into(),
                    "deepseek-r1".into(),
                    "qwen3-max".into(),
                    "qwen3-coder-plus".into(),
                    "glm-4.6".into(),
                    "kimi-k2".into(),
                ],
                enabled: true,
                groups: vec!["free".into(), "default".into()],
                remark: String::new(),
            },
        ],
        presentations: vec![
            PresentationRow {
                alias: "gpt-4o".into(),
                display_name: "GPT-4o".into(),
                description: "OpenAI 多模态主力".into(),
                icon: String::new(),
                badge: String::new(),
                tags: vec!["openai".into(), "多模态".into()],
                sort_order: 10,
                visible: true,
            },
            PresentationRow {
                alias: "gpt-5".into(),
                display_name: "GPT-5".into(),
                description: "新一代旗舰".into(),
                icon: String::new(),
                badge: "new".into(),
                tags: vec!["openai".into(), "旗舰".into()],
                sort_order: 0,
                visible: true,
            },
            PresentationRow {
                alias: "claude-sonnet-4".into(),
                display_name: "Claude Sonnet 4".into(),
                description: "编码与长文写作".into(),
                icon: String::new(),
                badge: "推荐".into(),
                tags: vec!["anthropic".into(), "编码".into()],
                sort_order: 5,
                visible: true,
            },
            PresentationRow {
                alias: "gemini-2.5-pro".into(),
                display_name: "Gemini 2.5 Pro".into(),
                description: "百万级上下文".into(),
                icon: String::new(),
                badge: String::new(),
                tags: vec!["google".into(), "长上下文".into()],
                sort_order: 20,
                visible: true,
            },
        ],
        api_keys: vec![
            KeyRow {
                name: "主力 key".into(),
                masked: "sk-....9f2a".into(),
                group: "default".into(),
                enabled: true,
            },
            KeyRow {
                name: "测试 key".into(),
                masked: "sk-....c41d".into(),
                group: "vip".into(),
                enabled: true,
            },
            KeyRow {
                name: "Claude 专用".into(),
                masked: "sk-....77b0".into(),
                group: "claude".into(),
                enabled: true,
            },
            KeyRow {
                name: "旧 key".into(),
                masked: "sk-....03e9".into(),
                group: "default".into(),
                enabled: false,
            },
        ],
        group_ratios: vec![
            GroupRatioRow {
                group: "default".into(),
                ratio: 1.0,
            },
            GroupRatioRow {
                group: "claude".into(),
                ratio: 1.0,
            },
            GroupRatioRow {
                group: "gpt-5".into(),
                ratio: 1.2,
            },
            GroupRatioRow {
                group: "vip".into(),
                ratio: 0.8,
            },
        ],
        cross_ratios: vec![CrossRatioRow {
            user_group: "default".into(),
            use_group: "vip".into(),
            ratio: 1.5,
        }],
        model_prices: vec![
            ModelPriceRow {
                alias: "gpt-4o".into(),
                input_per_m: 2.5,
                output_per_m: 10.0,
            },
            ModelPriceRow {
                alias: "gpt-5".into(),
                input_per_m: 5.0,
                output_per_m: 20.0,
            },
            ModelPriceRow {
                alias: "claude-sonnet-4".into(),
                input_per_m: 3.0,
                output_per_m: 15.0,
            },
            ModelPriceRow {
                alias: "gemini-2.5-pro".into(),
                input_per_m: 1.25,
                output_per_m: 10.0,
            },
        ],
        rate_rules: vec![
            RateRuleRow {
                name: "全局默认".into(),
                scope: "所有分组".into(),
                limit: 60,
                window: "每分钟".into(),
                enabled: true,
            },
            RateRuleRow {
                name: "VIP 宽松".into(),
                scope: "vip".into(),
                limit: 300,
                window: "每分钟".into(),
                enabled: true,
            },
        ],
        banned_words: vec!["赌博".into(), "诈骗".into(), "枪支".into()],
        announcement: "Ferrite 网关试运行中,遇到问题请反馈。".into(),
        system_facts: vec![
            ("版本".into(), "0.1.0-dev".into()),
            ("构建".into(), "debug · wasm32".into()),
            ("提交".into(), "cb06302".into()),
            ("运行时长".into(), "3 天 4 小时".into()),
        ],
        group_alias_links: vec![
            ("default".into(), "gpt-4o".into()),
            ("default".into(), "gpt-4o-mini".into()),
            ("default".into(), "gemini-2.5-pro".into()),
            ("default".into(), "gemini-2.5-flash".into()),
            ("default".into(), "deepseek-v3.2".into()),
            ("default".into(), "qwen3-max".into()),
            ("claude".into(), "claude-sonnet-4".into()),
            ("claude".into(), "claude-opus-4.1".into()),
            ("claude".into(), "claude-haiku-3.5".into()),
            ("gpt-5".into(), "gpt-5".into()),
            ("gpt-5".into(), "gpt-5-codex".into()),
            ("vip".into(), "gpt-4o".into()),
            ("vip".into(), "gpt-5".into()),
            ("vip".into(), "claude-sonnet-4".into()),
            ("vip".into(), "claude-opus-4.1".into()),
            ("vip".into(), "gemini-2.5-pro".into()),
            ("vip".into(), "deepseek-r1".into()),
            ("vip".into(), "grok-4".into()),
            ("vip".into(), "kimi-k2".into()),
            ("free".into(), "gpt-4o-mini".into()),
            ("free".into(), "gemini-2.5-flash".into()),
            ("free".into(), "deepseek-v3.2".into()),
            ("free".into(), "qwen3-coder-plus".into()),
            ("free".into(), "glm-4.6".into()),
            ("test".into(), "gpt-5-codex".into()),
            ("test".into(), "gemini-3-pro".into()),
            ("test".into(), "deepseek-r1".into()),
            ("test".into(), "qwen3-max".into()),
            ("test".into(), "grok-4".into()),
        ],
    }
}
