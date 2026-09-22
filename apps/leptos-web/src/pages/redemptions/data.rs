use leptos::prelude::*;
use singlestage::*;
use crate::ui::{CardGrid, Card, CardHeader, CardTitle, CardContent};

// Static demo data - in real app, this would come from the backend API
#[derive(Debug, Clone, Copy)]
pub struct RedemptionCardData {
    pub key: &'static str,
    pub code_preview: &'static str,
    pub quota_cny: f64,
    pub status: u8, // 1: 未使用 / 2: 已核销 / 3: 已停用
    pub redeemed_by: Option<&'static str>,
    pub redeemed_at: &'static str,
    pub created: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct RedemptionDemoData {
    // Stats data
    pub total_count: i64,
    pub unused_count: i64,
    pub used_count: i64,
    pub disabled_count: i64,
    pub available_quota: f64,

    // List data
    pub redemptions: Vec<RedemptionCardData>,
}


pub fn demo_data() -> RedemptionDemoData {
    RedemptionDemoData {
        pub total_count: 48,
        pub unused_count: 35,
        pub used_count: 8,
        pub disabled_count: 5,
        pub available_quota: 15678.50,
        pub redemptions: vec![
            RedemptionCardData {
                key: "red-001",
                pub code_preview: "fx-086c****",
                pub quota_cny: 100.00,
                pub status: 1,
                pub redeemed_by: None,
                pub redeemed_at: "",
                pub created: "2026-09-01 00:00",
            },
            RedemptionCardData {
                key: "red-002",
                pub code_preview: "fx-4b9d****",
                pub quota_cny: 50.00,
                pub status: 2,
                pub redeemed_by: Some("张三"),
                pub redeemed_at: "2026-09-02 10:00",
                pub created: "2026-08-28 14:30",
            },
            RedemptionCardData {
                key: "red-003",
                pub code_preview: "fx-3e2a****",
                pub quota_cny: 200.00,
                pub status: 1,
                pub redeemed_by: None,
                pub redeemed_at: "",
                pub created: "2026-09-03 09:15",
            },
            RedemptionCardData {
                key: "red-004",
                pub code_preview: "fx-7f1c****",
                pub quota_cny: 0.00,
                pub status: 3,
                pub redeemed_by: Some("李四"),
                pub redeemed_at: "2026-08-30 16:45",
                pub created: "2026-08-25 11:20",
            },
            RedemptionCardData {
                key: "red-005",
                pub code_preview: "fx-9d4e****",
                pub quota_cny: 500.00,
                pub status: 1,
                pub redeemed_by: None,
                pub redeemed_at: "",
                pub created: "2026-09-04 13:00",
            },
            RedemptionCardData {
                key: "red-006",
                pub code_preview: "fx-2a8b****",
                pub quota_cny: 75.00,
                pub status: 1,
                pub redeemed_by: None,
                pub redeemed_at: "",
                pub created: "2026-09-04 14:30",
            },
        ],
    }
}

// Status badge component - adapted from dioxus tab-page-groups
