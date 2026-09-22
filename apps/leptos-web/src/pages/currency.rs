use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;

const CURRENCIES: &[(&str, &str, &str)] = &[
    ("USD", "1.0000", "刚刚更新"),
    ("CNY", "7.0982", "1分钟前"),
    ("EUR", "0.9215", "刚刚更新"),
    ("JPY", "142.35", "3分钟前"),
];

#[component]
pub fn CurrencyPage() -> impl IntoView {
    let refreshed = RwSignal::new("刚刚更新");
    view! {
        <div class="space-y-6">
            <div class="flex justify-between items-center">
                <h1 class="text-3xl font-bold">"汇率概览"</h1>
                <button type="button" class="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700"
                    on:click=move |_| refreshed.set("刚刚刷新")>
                    "刷新"
                </button>
            </div>

            <CardGrid>
                {CURRENCIES.iter().map(|&(name, rate, time)| {
                    view! {
                        <Card>
                            <div class="p-6">
                                <div class="text-sm text-zinc-500 font-medium">{name}</div>
                                <div class="text-4xl font-semibold text-emerald-600 mt-2">{rate}</div>
                                <div class="mt-6 pt-4 border-t border-zinc-100 flex justify-between items-center">
                                    <span class="text-xs text-zinc-400">"更新时间"</span>
                                    <span class="text-sm text-zinc-600">{move || refreshed.get()}</span>
                                </div>
                            </div>
                        </Card>
                    }
                }).collect_view()}
            </CardGrid>
        </div>
    }
}
