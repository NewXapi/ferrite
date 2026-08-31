//! 账户线 mock 数据:密钥·资料面板、用量日志面板、邀请奖励面板。

/// 个人资料卡的四行内容。
#[derive(Clone, PartialEq)]
pub struct Profile {
    pub username: &'static str,
    pub email: &'static str,
    pub user_id: &'static str,
    pub registered_at: &'static str,
}

/// 一条 API 密钥。
#[derive(Clone, PartialEq)]
pub struct ApiKey {
    pub id: &'static str,
    pub name: &'static str,
    pub key: &'static str,
    pub status: &'static str,
    pub usage: &'static str,
    pub created: &'static str,
}

/// 一条请求日志。`timestamp` 是 Unix 秒。
#[derive(Clone, PartialEq)]
pub struct UsageLog {
    pub model: &'static str,
    pub success: bool,
    pub timestamp: i64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub cached_tokens: u32,
    pub first_token_ms: u32,
    pub duration_ms: u32,
    pub cost: f64,
    pub error: Option<&'static str>,
}

/// 钱包余额。
#[derive(Clone, PartialEq)]
pub struct Wallet {
    pub balance: &'static str,
    pub currency: &'static str,
}

/// 一条充值记录。
#[derive(Clone, PartialEq)]
pub struct Recharge {
    pub date: &'static str,
    pub method: &'static str,
    pub amount: &'static str,
}

/// 一张奖励统计卡。
#[derive(Clone, PartialEq)]
pub struct RewardStat {
    pub value: &'static str,
    pub label: &'static str,
    pub desc: &'static str,
}

/// 一位被邀请用户。
#[derive(Clone, PartialEq)]
pub struct Invitee {
    pub name: &'static str,
    pub date: &'static str,
    pub reward: &'static str,
}

/// 密钥·资料面板统计卡:(值, 标签)
pub const KEY_STATS: &[(&str, &str)] = &[
    ("12", "密钥总数"),
    ("¥248.6", "本月消耗"),
    ("18420", "本月请求"),
    ("3.2M", "剩余额度"),
    ("97.9%", "成功率"),
];

pub static PROFILE: Profile = Profile {
    username: "hathaway",
    email: "hathaway@wildtoken.com",
    user_id: "usr_8f3k9p2m",
    registered_at: "2024-11-03",
};

pub const KEYS: &[ApiKey] = &[
    ApiKey {
        id: "k1",
        name: "默认密钥",
        key: "sk-9x7v2m4p8q1w5e3r7t9y2u4i6o8p0",
        status: "启用",
        usage: "1.84M / 5M",
        created: "2025-06-12",
    },
    ApiKey {
        id: "k2",
        name: "测试密钥",
        key: "sk-4p8q2w6e9r3t7y1u5i0o2p4q6w8e",
        status: "限额",
        usage: "4.92M / 5M",
        created: "2025-07-01",
    },
    ApiKey {
        id: "k3",
        name: "生产环境",
        key: "sk-7t2y5u8i1o4p7q0w3e6r9t2y5u8i",
        status: "启用",
        usage: "0.92M / 10M",
        created: "2025-04-15",
    },
    ApiKey {
        id: "k4",
        name: "移动端集成",
        key: "sk-3r6t9y2u5i8o1p4q7w0e3r6t9y2u",
        status: "启用",
        usage: "2.14M / 5M",
        created: "2025-08-10",
    },
    ApiKey {
        id: "k5",
        name: "CI/CD 流水线",
        key: "sk-1q4w7e0r3t6y9u2i5o8p1q4w7e0r",
        status: "启用",
        usage: "0.31M / 2M",
        created: "2025-08-28",
    },
];

/// 用量日志面板统计卡:(值, 标签)
pub const USAGE_STATS: &[(&str, &str)] = &[
    ("1,284,392", "总 Token"),
    ("$87.42", "总花费"),
    ("2,847", "请求数"),
    ("2.1%", "失败率"),
];

/// 日志筛选可选模型,首项 "全部" 为不过滤。
pub const LOG_MODELS: &[&str] = &[
    "全部",
    "gpt-4o",
    "claude-3.5-sonnet",
    "deepseek-r1",
    "qwen2.5-72b",
    "gpt-4o-mini",
    "gemini-1.5-pro",
];

pub const LOGS: &[UsageLog] = &[
    UsageLog {
        model: "gpt-4o",
        success: true,
        timestamp: 1788060000,
        prompt_tokens: 656,
        completion_tokens: 142,
        cached_tokens: 0,
        first_token_ms: 900,
        duration_ms: 3000,
        cost: 0.0210,
        error: None,
    },
    UsageLog {
        model: "qwen2.5-72b",
        success: true,
        timestamp: 1788057600,
        prompt_tokens: 212060,
        completion_tokens: 521,
        cached_tokens: 206231,
        first_token_ms: 7500,
        duration_ms: 11000,
        cost: 0.152,
        error: None,
    },
    UsageLog {
        model: "deepseek-r1",
        success: false,
        timestamp: 1788055200,
        prompt_tokens: 0,
        completion_tokens: 0,
        cached_tokens: 0,
        first_token_ms: 0,
        duration_ms: 800,
        cost: 0.0,
        error: Some("Rate limit exceeded"),
    },
    UsageLog {
        model: "claude-3.5-sonnet",
        success: true,
        timestamp: 1788052800,
        prompt_tokens: 208606,
        completion_tokens: 196,
        cached_tokens: 206404,
        first_token_ms: 5600,
        duration_ms: 7000,
        cost: 0.64,
        error: None,
    },
    UsageLog {
        model: "gpt-4o-mini",
        success: true,
        timestamp: 1788050400,
        prompt_tokens: 206335,
        completion_tokens: 84,
        cached_tokens: 186595,
        first_token_ms: 5500,
        duration_ms: 5000,
        cost: 0.0089,
        error: None,
    },
    UsageLog {
        model: "gemini-1.5-pro",
        success: true,
        timestamp: 1788048000,
        prompt_tokens: 201030,
        completion_tokens: 5216,
        cached_tokens: 199750,
        first_token_ms: 11900,
        duration_ms: 46000,
        cost: 0.78,
        error: None,
    },
    UsageLog {
        model: "gpt-4o",
        success: true,
        timestamp: 1788045600,
        prompt_tokens: 199214,
        completion_tokens: 551,
        cached_tokens: 197972,
        first_token_ms: 3400,
        duration_ms: 8000,
        cost: 0.60,
        error: None,
    },
    UsageLog {
        model: "qwen2.5-72b",
        success: true,
        timestamp: 1788043200,
        prompt_tokens: 197204,
        completion_tokens: 783,
        cached_tokens: 184709,
        first_token_ms: 8200,
        duration_ms: 14000,
        cost: 0.59,
        error: None,
    },
    UsageLog {
        model: "deepseek-r1",
        success: true,
        timestamp: 1788040800,
        prompt_tokens: 185891,
        completion_tokens: 719,
        cached_tokens: 180843,
        first_token_ms: 31000,
        duration_ms: 34000,
        cost: 0.56,
        error: None,
    },
    UsageLog {
        model: "gpt-4o-mini",
        success: false,
        timestamp: 1788038400,
        prompt_tokens: 0,
        completion_tokens: 0,
        cached_tokens: 0,
        first_token_ms: 0,
        duration_ms: 450,
        cost: 0.0,
        error: Some("status_code=500, upstream connect failed"),
    },
    UsageLog {
        model: "claude-3.5-sonnet",
        success: true,
        timestamp: 1788036000,
        prompt_tokens: 184613,
        completion_tokens: 111,
        cached_tokens: 182110,
        first_token_ms: 7200,
        duration_ms: 9500,
        cost: 0.55,
        error: None,
    },
    UsageLog {
        model: "gpt-4o",
        success: true,
        timestamp: 1788033600,
        prompt_tokens: 165432,
        completion_tokens: 980,
        cached_tokens: 163002,
        first_token_ms: 4100,
        duration_ms: 6200,
        cost: 0.50,
        error: None,
    },
    UsageLog {
        model: "deepseek-r1",
        success: true,
        timestamp: 1788031200,
        prompt_tokens: 150022,
        completion_tokens: 664,
        cached_tokens: 148829,
        first_token_ms: 12500,
        duration_ms: 21000,
        cost: 0.45,
        error: None,
    },
    UsageLog {
        model: "gemini-1.5-pro",
        success: true,
        timestamp: 1788028800,
        prompt_tokens: 142100,
        completion_tokens: 890,
        cached_tokens: 140911,
        first_token_ms: 6700,
        duration_ms: 12000,
        cost: 0.43,
        error: None,
    },
    UsageLog {
        model: "qwen2.5-72b",
        success: false,
        timestamp: 1788026400,
        prompt_tokens: 0,
        completion_tokens: 0,
        cached_tokens: 0,
        first_token_ms: 0,
        duration_ms: 1200,
        cost: 0.0,
        error: Some("Context window exceeded"),
    },
];

pub static WALLET: Wallet = Wallet {
    balance: "2487.60",
    currency: "USDT",
};

pub const RECHARGES: &[Recharge] = &[
    Recharge {
        date: "2026-08-28",
        method: "兑换码充值",
        amount: "+500.00",
    },
    Recharge {
        date: "2026-08-15",
        method: "兑换码充值",
        amount: "+200.00",
    },
    Recharge {
        date: "2026-08-02",
        method: "兑换码充值",
        amount: "+1000.00",
    },
];

pub const REWARD_STATS: &[RewardStat] = &[
    RewardStat {
        value: "¥1248.50",
        label: "累计奖励",
        desc: "累计获得奖励金额",
    },
    RewardStat {
        value: "17",
        label: "已邀人数",
        desc: "成功邀请注册用户数",
    },
    RewardStat {
        value: "¥328.40",
        label: "待结算",
        desc: "可提现奖励余额",
    },
];

pub const INVITEES: &[Invitee] = &[
    Invitee {
        name: "小明",
        date: "2026-08-20",
        reward: "+¥85.20",
    },
    Invitee {
        name: "张伟",
        date: "2026-08-18",
        reward: "+¥124.00",
    },
    Invitee {
        name: "李娜",
        date: "2026-08-10",
        reward: "+¥42.50",
    },
    Invitee {
        name: "王芳",
        date: "2026-08-05",
        reward: "+¥67.80",
    },
    Invitee {
        name: "刘洋",
        date: "2026-07-28",
        reward: "+¥210.00",
    },
    Invitee {
        name: "陈静",
        date: "2026-07-15",
        reward: "+¥35.40",
    },
];

pub const INVITE_LINK: &str = "https://ferrite.ai/invite?code=FER12345678";
