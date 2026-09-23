#[derive(Clone, Copy)]
pub struct UsageLog {
    pub model_name: &'static str,
    pub token_name: &'static str,
    pub channel_name: &'static str,
    pub created_at: &'static str,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub use_time_ms: i64,
    pub quota: i64,
    pub is_stream: bool,
}

pub fn demo_usage_logs() -> Vec<UsageLog> {
    vec![
        UsageLog {
            model_name: "gpt-4o",
            token_name: "默认令牌",
            channel_name: "OpenAI",
            created_at: "2025-09-23 14:30",
            prompt_tokens: 1500,
            completion_tokens: 800,
            use_time_ms: 2500,
            quota: 15000,
            is_stream: true,
        },
        UsageLog {
            model_name: "claude-3.5-sonnet",
            token_name: "API令牌",
            channel_name: "Anthropic",
            created_at: "2025-09-23 13:45",
            prompt_tokens: 2000,
            completion_tokens: 1200,
            use_time_ms: 3200,
            quota: 24000,
            is_stream: false,
        },
        UsageLog {
            model_name: "deepseek-r1",
            token_name: "测试令牌",
            channel_name: "DeepSeek",
            created_at: "2025-09-23 12:15",
            prompt_tokens: 800,
            completion_tokens: 600,
            use_time_ms: 1500,
            quota: 8000,
            is_stream: true,
        },
    ]
}

pub fn fmt_num(v: i64) -> String {
    v.to_string()
}

pub fn fmt_quota(v: i64) -> String {
    format!("${:.2}", v as f64 / 500_000.0)
}
