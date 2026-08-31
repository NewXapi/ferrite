//! 模型展示面板 mock 数据:模型卡片 + 分组价格。

/// Per-group pricing row (分组价格 tab).
#[derive(Clone, Copy, PartialEq)]
pub struct GroupPrice {
    pub name: &'static str,
    pub input: &'static str,
    pub output: &'static str,
    pub cache: &'static str,
}

/// One model detail card's data.
#[derive(Clone, PartialEq)]
pub struct ModelInfo {
    pub name: &'static str,
    pub vendor: &'static str,
    pub description: &'static str,
    /// 输入 / 输出 / 缓存
    pub price_input: &'static str,
    pub price_output: &'static str,
    pub price_cache: &'static str,
    /// 数据展示
    pub tokens_24h: &'static str,
    pub cost_24h: &'static str,
    pub requests_24h: &'static str,
    pub success_rate: &'static str,
    pub latency_p50: &'static str,
    /// 画图展示: 24 sparkline points (0..=100) + 24 hourly heat levels (0..=4)
    pub trend: &'static [u8],
    pub heat: &'static [u8],
    /// 分组价格
    pub groups: &'static [GroupPrice],
}

const TREND_A: &[u8] = &[
    30, 38, 34, 46, 52, 47, 60, 58, 66, 61, 72, 78, 70, 82, 76, 88, 84, 92, 86, 95, 90, 97, 93, 100,
];
const HEAT_A: &[u8] = &[
    1, 0, 2, 1, 3, 2, 0, 4, 2, 1, 3, 0, 2, 4, 1, 2, 3, 0, 1, 2, 4, 3, 2, 1,
];
const TREND_B: &[u8] = &[
    70, 62, 66, 54, 60, 48, 56, 44, 52, 40, 46, 34, 42, 30, 38, 28, 36, 26, 34, 40, 30, 44, 36, 50,
];
const HEAT_B: &[u8] = &[
    3, 4, 2, 3, 1, 2, 4, 0, 2, 3, 1, 4, 2, 0, 3, 2, 1, 3, 4, 2, 1, 0, 2, 3,
];

pub const MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "gpt-5.2",
        vendor: "openai",
        description: "旗舰通用模型，长上下文与工具调用强化。",
        price_input: "12.5",
        price_output: "100",
        price_cache: "1.25",
        tokens_24h: "70,514,208",
        cost_24h: "41.80",
        requests_24h: "3,204",
        success_rate: "99.2%",
        latency_p50: "1.4s",
        trend: TREND_A,
        heat: HEAT_A,
        groups: &[
            GroupPrice {
                name: "默认",
                input: "12.5",
                output: "100",
                cache: "1.25",
            },
            GroupPrice {
                name: "奶酪",
                input: "8",
                output: "10",
                cache: "1",
            },
            GroupPrice {
                name: "牛奶",
                input: "10",
                output: "80",
                cache: "1.1",
            },
            GroupPrice {
                name: "芝士",
                input: "15",
                output: "120",
                cache: "1.5",
            },
        ],
    },
    ModelInfo {
        name: "deepseek-chat",
        vendor: "deepseek",
        description: "高性价比对话模型，缓存命中价格极低。",
        price_input: "10",
        price_output: "20",
        price_cache: "0.1",
        tokens_24h: "48,713,006",
        cost_24h: "18.02",
        requests_24h: "5,861",
        success_rate: "98.7%",
        latency_p50: "0.9s",
        trend: TREND_B,
        heat: HEAT_B,
        groups: &[
            GroupPrice {
                name: "默认",
                input: "10",
                output: "20",
                cache: "0.1",
            },
            GroupPrice {
                name: "蓝纹奶酪",
                input: "12",
                output: "24",
                cache: "0.12",
            },
            GroupPrice {
                name: "芝士",
                input: "9",
                output: "18",
                cache: "0.09",
            },
        ],
    },
    ModelInfo {
        name: "claude-opus-4.7",
        vendor: "anthropic",
        description: "长程推理模型，代码与 Agent 任务首选。",
        price_input: "15",
        price_output: "75",
        price_cache: "1.5",
        tokens_24h: "31,206,544",
        cost_24h: "27.35",
        requests_24h: "2,047",
        success_rate: "99.5%",
        latency_p50: "2.1s",
        trend: TREND_A,
        heat: HEAT_B,
        groups: &[GroupPrice {
            name: "默认",
            input: "15",
            output: "75",
            cache: "1.5",
        }],
    },
];
