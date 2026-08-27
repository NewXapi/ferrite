//! 管理面板的共享实体 store：分组 / 模型别名 / 渠道。
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
    /// 拉取回来、尚未进入拓扑的候补：(模型名, 是否勾选)
    pub candidates: Vec<(String, bool)>,
    /// 已加入拓扑的调度模型；名字来自上游，不可改
    pub dispatch: Vec<String>,
}

#[derive(Clone, Copy)]
pub struct EntityStore {
    pub groups: Signal<Vec<GroupRow>>,
    pub aliases: Signal<Vec<AliasRow>>,
    pub channels: Signal<Vec<ChannelRow>>,
}

impl EntityStore {
    pub fn seed() -> Self {
        Self {
            groups: Signal::new(vec![
                GroupRow {
                    name: "default".into(),
                    display: "默认分组".into(),
                },
                GroupRow {
                    name: "claude".into(),
                    display: "Claude 专用".into(),
                },
                GroupRow {
                    name: "gpt-5".into(),
                    display: "GPT-5".into(),
                },
                GroupRow {
                    name: "vip".into(),
                    display: "VIP".into(),
                },
            ]),
            aliases: Signal::new(vec![
                AliasRow {
                    alias: "gpt-4o".into(),
                    display: "GPT-4o".into(),
                },
                AliasRow {
                    alias: "gpt-5".into(),
                    display: "GPT-5".into(),
                },
                AliasRow {
                    alias: "claude-sonnet-4".into(),
                    display: "Claude Sonnet 4".into(),
                },
                AliasRow {
                    alias: "gemini-2.5-pro".into(),
                    display: "Gemini 2.5 Pro".into(),
                },
            ]),
            channels: Signal::new(vec![
                ChannelRow {
                    name: "OpenAI 官方".into(),
                    url: "https://api.openai.com/v1".into(),
                    keys: "sk-**************************".into(),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into(), "gpt-5".into()],
                },
                ChannelRow {
                    name: "Azure East".into(),
                    url: "https://east.azure.example/openai".into(),
                    keys: "az-****".into(),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into()],
                },
                ChannelRow {
                    name: "OneAPI 上游".into(),
                    url: "https://oneapi.example/v1".into(),
                    keys: "oa-****".into(),
                    candidates: vec![],
                    dispatch: vec!["gpt-4o".into(), "gpt-5".into(), "claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "Claude 官网".into(),
                    url: "https://api.anthropic.com".into(),
                    keys: "ak-****".into(),
                    candidates: vec![],
                    dispatch: vec!["claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "AWS Bedrock".into(),
                    url: "https://bedrock.us-east-1.amazonaws.com".into(),
                    keys: "aws-****".into(),
                    candidates: vec![],
                    dispatch: vec!["claude-sonnet-4".into()],
                },
                ChannelRow {
                    name: "Gemini".into(),
                    url: "https://generativelanguage.googleapis.com".into(),
                    keys: "gm-****".into(),
                    candidates: vec![],
                    dispatch: vec!["gemini-2.5-pro".into()],
                },
            ]),
        }
    }
}
