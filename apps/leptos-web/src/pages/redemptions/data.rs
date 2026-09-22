use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, CardHeader, CardTitle, CardContent};

// Static demo data - in real app, this would come from the backend API
#[derive(Debug, Clone, Copy)]
pub struct RedemptionCardData {
    key: &'static str,
    code_preview: &'static str,
    quota_cny: f64,
    status: u8, // 1: 未使用 / 2: 已核销 / 3: 已停用
    redeemed_by: Option<&'static str>,
    redeemed_at: &'static str,
    created: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct RedemptionDemoData {
    // Stats data
    total_count: i64,
    unused_count: i64,
    used_count: i64,
    disabled_count: i64,
    available_quota: f64,

    // List data
    redemptions: Vec<RedemptionCardData>,
}


pub fn demo_data() -> RedemptionDemoData {
    RedemptionDemoData {
        total_count: 48,
        unused_count: 35,
        used_count: 8,
        disabled_count: 5,
        available_quota: 15678.50,
        redemptions: vec![
            RedemptionCardData {
                key: "red-001",
                code_preview: "fx-086c****",
                quota_cny: 100.00,
                status: 1,
                redeemed_by: None,
                redeemed_at: "",
                created: "2026-09-01 00:00",
            },
            RedemptionCardData {
                key: "red-002",
                code_preview: "fx-4b9d****",
                quota_cny: 50.00,
                status: 2,
                redeemed_by: Some("张三"),
                redeemed_at: "2026-09-02 10:00",
                created: "2026-08-28 14:30",
            },
            RedemptionCardData {
                key: "red-003",
                code_preview: "fx-3e2a****",
                quota_cny: 200.00,
                status: 1,
                redeemed_by: None,
                redeemed_at: "",
                created: "2026-09-03 09:15",
            },
            RedemptionCardData {
                key: "red-004",
                code_preview: "fx-7f1c****",
                quota_cny: 0.00,
                status: 3,
                redeemed_by: Some("李四"),
                redeemed_at: "2026-08-30 16:45",
                created: "2026-08-25 11:20",
            },
            RedemptionCardData {
                key: "red-005",
                code_preview: "fx-9d4e****",
                quota_cny: 500.00,
                status: 1,
                redeemed_by: None,
                redeemed_at: "",
                created: "2026-09-04 13:00",
            },
            RedemptionCardData {
                key: "red-006",
                code_preview: "fx-2a8b****",
                quota_cny: 75.00,
                status: 1,
                redeemed_by: None,
                redeemed_at: "",
                created: "2026-09-04 14:30",
            },
        ],
    }
}

// Status badge component - adapted from dioxus tab-page-groups
