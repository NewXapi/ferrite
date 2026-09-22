use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, Dialog, DialogTrigger, CardHeader, CardTitle, CardContent};

#[derive(Clone)]
struct UsageLog {
    id: &'static str,
    model_name: &'static str,
    token_name: &'static str,
    channel_name: &'static str,
    created_at: &'static str,
    prompt_tokens: i64,
    completion_tokens: i64,
    use_time_ms: i64,
    quota: i64,
    is_stream: bool,
    ip: &'static str,
    request_id: &'static str,
}


pub fn demo_usage_logs() -> Vec<UsageLog> {
    vec![
        UsageLog {
            id: "log_001",
            model_name: "gpt-4o",
            token_name: "默认令牌",
            channel_name: "OpenAI",
            created_at: "2025-09-23 14:30",
            prompt_tokens: 1500,
            completion_tokens: 800,
            use_time_ms: 2500,
            quota: 15000,
            is_stream: true,
            ip: "192.168.1.100",
            request_id: "req_abc123",
        },
        UsageLog {
            id: "log_002",
            model_name: "claude-3.5-sonnet",
            token_name: "API令牌",
            channel_name: "Anthropic",
            created_at: "2025-09-23 13:45",
            prompt_tokens: 2000,
            completion_tokens: 1200,
            use_time_ms: 3200,
            quota: 24000,
            is_stream: false,
            ip: "192.168.1.101",
            request_id: "req_def456",
        },
        UsageLog {
            id: "log_003",
            model_name: "deepseek-r1",
            token_name: "测试令牌",
            channel_name: "DeepSeek",
            created_at: "2025-09-23 12:15",
            prompt_tokens: 800,
            completion_tokens: 600,
            use_time_ms: 1500,
            quota: 8000,
            is_stream: true,
            ip: "192.168.1.102",
            request_id: "req_ghi789",
        },
    ]
}


pub fn fmt_num(v: i64) -> String {
    v.to_string()
}


pub fn fmt_time(v: &str) -> String {
    v.to_string()
}


pub fn fmt_quota(v: i64) -> String {
    format!("${:.2}", v as f64 / 500_000.0)
}

