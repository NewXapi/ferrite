use super::card::KeyItem;

pub fn demo_keys() -> Vec<KeyItem> {
    vec![
        KeyItem {
            name: "production-key",
            key_preview: "sk-prod****abcd",
            status: 1,
            unlimited_quota: false,
            used_quota: 1500000,
            quota: 5000000,
            created_at: "2024-01-15",
        },
        KeyItem {
            name: "dev-test-key",
            key_preview: "sk-dev****efgh",
            status: 2,
            unlimited_quota: true,
            used_quota: 0,
            quota: 0,
            created_at: "2024-02-20",
        },
        KeyItem {
            name: "analytics-key",
            key_preview: "sk-analytics****ijkl",
            status: 1,
            unlimited_quota: false,
            used_quota: 3200000,
            quota: 10000000,
            created_at: "2024-03-10",
        },
    ]
}

pub fn fmt_quota(v: i64) -> String {
    if v == 0 {
        "无限".to_string()
    } else {
        format!("${:.2}", v as f64 / 500_000.0)
    }
}
