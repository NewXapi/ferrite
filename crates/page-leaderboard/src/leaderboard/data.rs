//! 排行榜数据层: 模型数值 + 派生计算.
//! 加新模型 = 在 `MODELS` 里加一行 (含立绘 asset); 角标/卡牌/图表全部由数值推导.

use dioxus::prelude::{Asset, asset};

/// 六维雷达原始值: (速度 tok/s, 价格 $/1M 输入, 上下文 K, 成功率 %, P50 延迟 s, 日请求量)
#[derive(Clone, PartialEq)]
pub struct ModelStat {
    pub name: &'static str,
    pub speed: f64,
    pub price: f64,
    pub ctx: f64,
    pub success: f64,
    pub p50: f64,
    pub daily_req: f64,
    /// 日 token 消耗 (M)
    pub tokens: f64,
    /// 同比增长率 (%)
    pub growth: f64,
    /// 海报描述
    pub desc: &'static str,
    /// 立绘; None = 首字母占位
    pub art: Option<Asset>,
}

pub const DIMS: [&str; 6] = ["速度", "性价比", "上下文", "成功率", "稳定性", "热度"];

/// ponytail: 这是唯一的模型注册点. 名字/数值/立绘写在这里, 其余 UI 全部自动派生.
pub static MODELS: &[ModelStat] = &[
    ModelStat {
        name: "gpt-5.6-sol",
        speed: 92.0,
        price: 3.0,
        ctx: 400.0,
        success: 99.9,
        p50: 0.8,
        daily_req: 3.9e5,
        tokens: 3200.0,
        growth: 18.5,
        desc: "OpenAI 旗舰推理模型, 综合分第一",
        art: Some(asset!("/assets/models/avif-60/gpt.avif")),
    },
    ModelStat {
        name: "claude-fable-5",
        speed: 78.0,
        price: 15.0,
        ctx: 200.0,
        success: 99.5,
        p50: 2.1,
        daily_req: 1.4e5,
        tokens: 1500.0,
        growth: 9.0,
        desc: "Anthropic 长程推理, 代码与 Agent 首选",
        art: Some(asset!("/assets/models/avif-60/claude.avif")),
    },
    ModelStat {
        name: "kimi-k3",
        speed: 110.0,
        price: 0.9,
        ctx: 256.0,
        success: 99.2,
        p50: 1.1,
        daily_req: 2.2e5,
        tokens: 1800.0,
        growth: 22.0,
        desc: "Moonshot 高速模型, 性价比与速度兼备",
        art: Some(asset!("/assets/models/avif-60/kimi.avif")),
    },
    ModelStat {
        name: "glm-5.2",
        speed: 64.0,
        price: 1.2,
        ctx: 128.0,
        success: 98.8,
        p50: 1.9,
        daily_req: 9.0e4,
        tokens: 700.0,
        growth: 14.0,
        desc: "智谱主力模型, 均衡型",
        art: Some(asset!("/assets/models/avif-60/zcode.avif")),
    },
    ModelStat {
        name: "deepseek-v4",
        speed: 120.0,
        price: 0.55,
        ctx: 256.0,
        success: 99.6,
        p50: 0.6,
        daily_req: 2.9e5,
        tokens: 2400.0,
        growth: 31.0,
        desc: "DeepSeek 最新代, 高速低价",
        art: Some(asset!("/assets/models/avif-60/deepseek.avif")),
    },
    ModelStat {
        name: "qwen-max-3",
        speed: 88.0,
        price: 1.6,
        ctx: 262.0,
        success: 99.0,
        p50: 1.4,
        daily_req: 1.1e5,
        tokens: 900.0,
        growth: 11.0,
        desc: "通义旗舰, 大上下文稳定输出",
        art: Some(asset!("/assets/models/avif-60/qwen.avif")),
    },
    ModelStat {
        name: "gemini-3-pro",
        speed: 71.0,
        price: 1.25,
        ctx: 1000.0,
        success: 98.5,
        p50: 1.8,
        daily_req: 7.0e4,
        tokens: 550.0,
        growth: 6.5,
        desc: "Google 百万上下文, 多模态",
        art: Some(asset!("/assets/models/avif-60/gemini.avif")),
    },
    ModelStat {
        name: "grok-4",
        speed: 95.0,
        price: 5.0,
        ctx: 128.0,
        success: 97.9,
        p50: 2.5,
        daily_req: 8.0e4,
        tokens: 650.0,
        growth: 8.0,
        desc: "xAI 旗舰, 高价但热度稳定",
        art: Some(asset!("/assets/models/avif-60/grok.avif")),
    },
];

/// 归一化单维到 0..=1; `inverse` 维 (价格, 延迟): 越低分越高.
pub fn norms(m: &ModelStat) -> [f64; 6] {
    [
        (m.speed / 130.0).min(1.0),
        (1.0 - (m.price - 0.5) / 15.0).clamp(0.0, 1.0),
        (m.ctx / 1024.0).min(1.0),
        ((m.success - 97.0) / 3.0).clamp(0.0, 1.0),
        (1.0 - (m.p50 - 0.5) / 2.2).clamp(0.0, 1.0),
        (m.daily_req / 4.0e5).min(1.0),
    ]
}

/// 综合分 = 六维归一值的均值 x100.
pub fn composite(m: &ModelStat) -> f64 {
    norms(m).iter().sum::<f64>() / 6.0 * 100.0
}

/// 六维展示用原始值 (格式化后).
pub fn dim_raw(m: &ModelStat) -> [String; 6] {
    [
        format!("{}", m.speed as u32),
        format!("${:.2}", m.price),
        format!("{}K", m.ctx as u32),
        format!("{:.1}%", m.success),
        format!("{:.1}s", m.p50),
        format!("{:.1}K/day", m.daily_req / 1e3),
    ]
}

/// 全模型各维均值 (雷达均值虚线用).
pub fn avg_norms() -> [f64; 6] {
    (0..6)
        .map(|i| MODELS.iter().map(|m| norms(m)[i]).sum::<f64>() / MODELS.len() as f64)
        .collect::<Vec<f64>>()
        .try_into()
        .unwrap()
}

/// 每个维度上该模型的名次 (1 = 最强), 跨全模型排位.
pub fn dim_rank(m: &ModelStat) -> [usize; 6] {
    let values = norms(m);
    core::array::from_fn(|i| {
        MODELS
            .iter()
            .filter(|o| norms(o)[i] > values[i] + 1e-9)
            .count()
            + 1
    })
}

/// 卡牌正面/背面共用的三条关键数据.
pub struct KeyStat {
    /// 缩写标签 (Token / Requests / Share)
    pub short: &'static str,
    /// 悬停展开的全称文案
    pub full: String,
    pub text: String,
}

pub fn key_stats(m: &ModelStat) -> [KeyStat; 3] {
    let total_tokens: f64 = MODELS.iter().map(|x| x.tokens).sum();
    let token = format!("{:.1}B", m.tokens / 1000.0);
    let requests = format!("{:.1}K/day", m.daily_req / 1e3);
    let share = format!("{:.1}%", m.tokens / total_tokens * 100.0);
    [
        KeyStat {
            short: "Token",
            full: format!("Total Token: {token}"),
            text: token,
        },
        KeyStat {
            short: "Requests",
            full: format!("Total Requests: {requests}"),
            text: requests,
        },
        KeyStat {
            short: "Share",
            full: format!("Share of all tokens: {share}"),
            text: share,
        },
    ]
}
