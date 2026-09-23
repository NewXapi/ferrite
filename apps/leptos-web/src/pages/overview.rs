use leptos::prelude::*;

use crate::ui::Card;

// Hardcoded statistics data for 5 cards
const STATS_DATA: [(&str, &str, &str); 5] = [
    ("1,234", "总用户", "users"),
    ("567", "启用渠道", "channels"),
    ("89.5万", "令牌", "tokens"),
    ("42", "分组", "groups"),
    ("7,890", "今日请求", "requests_today"),
];

// Hardcoded trend data for bar chart (24 hours of hourly values)
const TREND_DATA: [f64; 24] = [
    1.2, 1.5, 1.8, 2.1, 2.3, 2.6, 2.9, 3.2, 3.5, 3.8, 4.1, 4.5, 4.8, 5.2, 5.5, 5.9, 6.2, 6.5, 6.8,
    7.1, 7.4, 7.7, 8.0, 8.5,
];

// Helper function to calculate max value for scaling
fn get_max_trend_value() -> f64 {
    TREND_DATA.iter().fold(0.0, |max, &v| max.max(v))
}

#[component]
pub fn OverviewPage() -> impl IntoView {
    let timeframe = RwSignal::new("今天");
    let as_of_time = "今天 14:30";

    // Card component for statistics
    let stat_cards: Vec<_> = STATS_DATA
        .iter()
        .map(|(value, label, unit)| {
            view! {
                <Card class="p-4">
                    <div class="flex flex-col gap-2">
                        <div class="text-2xl font-bold">
                            {*value}
                        </div>
                        <div class="text-sm text-muted-foreground">
                            {*label}
                        </div>
                        <div class="text-xs text-muted-foreground">
                            {*unit}
                        </div>
                    </div>
                </Card>
            }
        })
        .collect_view();

    // Simple trend bar chart using div
    let max_value = get_max_trend_value();
    let trend_bars: Vec<_> = TREND_DATA
        .iter()
        .enumerate()
        .map(|(hour, &value)| {
            let height_class = match value / max_value {
                v if v > 0.8 => "bg-blue-600",
                v if v > 0.6 => "bg-blue-500",
                v if v > 0.4 => "bg-blue-400",
                v if v > 0.2 => "bg-blue-300",
                _ => "bg-blue-200",
            };
            view! {
                <div class="flex flex-col items-center gap-1">
                    <div
                        class=format!("h-8 w-8 rounded-sm transition-all hover:h-12 {}", height_class)
                        title=format!("{:.1} 点", value)
                    ></div>
                    <div class="text-xs text-muted-foreground">
                        {format!("{}", hour)}
                    </div>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="flex flex-col gap-6 p-6">
            // Statistics section with 5 cards
            <div class="grid grid-cols-1 md:grid-cols-3 lg:grid-cols-5 gap-4">
                {stat_cards}
            </div>

            // Trend bar chart section
            <div class="bg-card rounded-lg border p-6">
                <h3 class="text-lg font-semibold mb-4">"今日趋势"</h3>
                <div class="flex items-end justify-between h-32 gap-2">
                    {trend_bars}
                </div>
                <div class="mt-4 flex justify-between text-xs text-muted-foreground">
                    <span>"0:00"</span>
                    <span>"04:00"</span>
                    <span>"08:00"</span>
                    <span>"12:00"</span>
                    <span>"16:00"</span>
                    <span>"20:00"</span>
                    <span>"24:00"</span>
                </div>
            </div>

            <div class="flex items-center justify-between text-sm text-muted-foreground">
                <span>"数据更新于: " {as_of_time}</span>
                <span>"时间窗: " {move || timeframe.get()}</span>
            </div>
        </div>
    }
}
