use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

#[derive(Clone, Copy)]
struct ModelRank {
    rank: usize,
    name: &'static str,
    score: &'static str,
    growth: f32,
}

const RANKINGS: &[ModelRank] = &[
    ModelRank {
        rank: 1,
        name: "Claude-3.5-Sonnet",
        score: "98.7",
        growth: 12.4,
    },
    ModelRank {
        rank: 2,
        name: "GPT-4o",
        score: "96.5",
        growth: -2.1,
    },
    ModelRank {
        rank: 3,
        name: "Gemini-1.5-Pro",
        score: "94.2",
        growth: 8.7,
    },
    ModelRank {
        rank: 4,
        name: "DeepSeek-R1",
        score: "93.8",
        growth: 15.2,
    },
    ModelRank {
        rank: 5,
        name: "Qwen2.5-72B",
        score: "91.9",
        growth: 5.3,
    },
    ModelRank {
        rank: 6,
        name: "Llama-3.1-405B",
        score: "89.4",
        growth: -4.6,
    },
];

#[component]
pub fn LeaderboardPage() -> impl IntoView {
    let timeframe = RwSignal::new("30天".to_string());
    let options = ["7天", "30天", "全部"];

    view! {
        <div class="flex flex-col gap-6 p-6">
            <div class="flex justify-between items-center">
                <h1 class="text-2xl font-semibold text-white">"模型排行榜"</h1>
                <div class="flex gap-1 bg-zinc-900 p-1 rounded-xl">
                    {options.iter().map(|&opt| {
                        let opt = opt.to_string();
                        let opt_active = opt.clone();
                        let opt_click = opt.clone();
                        let active = move || timeframe.get() == opt_active;
                        view! {
                            <Button
                                button_type="button"
                                class=MaybeProp::derive(move || {
                                    Some(
                                        if active() {
                                            "px-5 py-1.5 rounded-[10px] bg-white text-zinc-900 text-sm font-medium".to_string()
                                        } else {
                                            "px-5 py-1.5 rounded-[10px] text-zinc-400 hover:text-zinc-200 text-sm".to_string()
                                        },
                                    )
                                })
                                on:click=move |_| timeframe.set(opt_click.clone())
                            >
                                {opt}
                            </Button>
                        }
                    }).collect_view()}
                </div>
            </div>

            <CardGrid>
                {RANKINGS.iter().map(|r| {
                    let growth_class = if r.growth >= 0.0 {
                        "text-emerald-400"
                    } else {
                        "text-red-400"
                    };
                    let sign = if r.growth >= 0.0 { "+" } else { "" };
                    view! {
                        <Card class="p-6 bg-zinc-900 border border-zinc-700 rounded-2xl hover:border-zinc-500 transition-colors">
                            <div class="flex justify-between mb-6">
                                <div class="text-5xl font-bold text-zinc-700">#{r.rank}</div>
                                <div class=format!("text-xl font-semibold {}", growth_class)>
                                    {sign}{r.growth}"%"
                                </div>
                            </div>
                            <div class="font-medium text-white text-lg mb-1">{r.name}</div>
                            <div class="text-6xl font-bold text-white tracking-tighter tabular-nums">{r.score}</div>
                            <div class="text-xs uppercase tracking-widest text-zinc-500 mt-2">"综合评分"</div>
                        </Card>
                    }
                }).collect_view()}
            </CardGrid>
        </div>
    }
}
