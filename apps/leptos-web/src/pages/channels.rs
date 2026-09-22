use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;
use crate::ui::Dialog;

// Internal constants for channel data (replaces external module references)
const SAMPLE_CHANNELS: &[(&str, &str, &str, &str, i16, &str)] = &[
    ("channel_001", "OpenAI 官方", "openai", "https://api.openai.com/v1", 1, "官方渠道，优先级最高"),
    ("channel_002", "Azure East", "azure", "https://eastus.api.microsoft.com", 1, "微软云服务"),
    ("channel_003", "自定义测试", "custom", "https://custom.api.example.com", 2, "已停用"),
];

#[component]
pub fn ChannelsPage() -> impl IntoView {
    // State management using RwSignal
    let channels = RwSignal::new(SAMPLE_CHANNELS.to_vec());
    let search = RwSignal::new(String::new());
    let filter_tier: RwSignal<usize> = RwSignal::new(0);
    let modal_open = RwSignal::new(false);
    let current_key = RwSignal::new(String::new());

    // Filtered channels computation
    let filtered = move || {
        let q = search.get().trim().to_lowercase();
        let tier = filter_tier.get();
        channels.get().iter().filter(|&&c| {
            let name = c.1.to_lowercase();
            let remark = c.5.to_lowercase();
            if !q.is_empty() && !name.contains(&q) && !remark.contains(&q) {
                return false;
            }
            match tier {
                1 => c.4 == 1,
                2 => c.4 != 1,
                _ => true,
            }
        }).cloned().collect::<Vec<_>>()
    };

    // Event handlers
    let open_modal = move |key: String| {
        current_key.set(key);
        modal_open.set(true);
    };

    let close_modal = move || {
        modal_open.set(false);
        current_key.set(String::new());
    };

    view! {
        <div class="flex flex-col gap-6 p-6">
            <div class="flex justify-between items-center">
                <h1 class="text-2xl font-bold text-white">渠道管理</h1>
                <button 
                    class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-lg transition-colors"
                    button_type="button"
                    on:click=move |_| open_modal("new".to_string())
                >
                    新建渠道
                </button>
            </div>

            <div class="flex gap-4 mb-4">
                <input 
                    class="flex-1 px-4 py-2 bg-zinc-900 border border-zinc-700 rounded-lg text-white placeholder-zinc-500"
                    placeholder="搜索渠道..."
                    on:input=move |ev| search.set(event_target_value(&ev))
                />
                <button 
                    class="px-6 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                    button_type="button"
                    on:click=move |_| filter_tier.set(0)
                >全部</button>
                <button 
                    class="px-6 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                    button_type="button"
                    on:click=move |_| filter_tier.set(1)
                >启用</button>
                <button 
                    class="px-6 py-2 bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                    button_type="button"
                    on:click=move |_| filter_tier.set(2)
                >停用</button>
            </div>

            <CardGrid>
                {filtered().into_iter().map(|c| {
                    let key = c.0.to_string();
                    let is_enabled = c.4 == 1;
                    let status_text = if is_enabled { "启用中" } else { "已停用" };
                    let status_class = if is_enabled { "bg-green-500/20 text-green-400" } else { "bg-red-500/20 text-red-400" };

                    view! {
                        <Card class="bg-zinc-900 border border-zinc-700 rounded-xl overflow-hidden">
                            <div class="p-4">
                                <div class="flex justify-between items-start mb-3">
                                    <div>
                                        <div class="font-semibold text-white text-lg">{c.1}</div>
                                        <div class="text-xs text-zinc-500 font-mono mt-0.5">{c.3}</div>
                                    </div>
                                    <span class=format!("inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium {}", status_class)>
                                        {status_text}
                                    </span>
                                </div>

                                <div class="space-y-2 text-sm">
                                    <div class="flex justify-between">
                                        <span class="text-zinc-400">权重</span>
                                        <span class="text-white">1</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-zinc-400">分组</span>
                                        <span class="text-emerald-400">default</span>
                                    </div>
                                    <div class="flex justify-between">
                                        <span class="text-zinc-400">备注</span>
                                        <span class="text-zinc-400 text-right">{c.5}</span>
                                    </div>
                                </div>
                            </div>

                            <div class="border-t border-zinc-700 p-3 flex gap-2">
                                <button 
                                    class="flex-1 py-2 text-sm bg-zinc-800 hover:bg-zinc-700 text-white rounded-lg transition-colors"
                                    button_type="button"
                                    on:click=move |_| open_modal(key.clone())
                                >
                                    编辑
                                </button>
                                <button 
                                    class="px-4 py-2 text-sm bg-red-900/30 hover:bg-red-900/50 text-red-400 rounded-lg transition-colors border border-red-800/50"
                                    button_type="button"
                                >
                                    删除
                                </button>
                                <button 
                                    class=format!("px-4 py-2 text-sm rounded-lg transition-colors {}", if is_enabled { "bg-amber-900/30 text-amber-400 border border-amber-800/50" } else { "bg-emerald-900/30 text-emerald-400 border border-emerald-800/50" })
                                    button_type="button"
                                >
                                    {if is_enabled { "停用" } else { "启用" }}
                                </button>
                            </div>
                        </Card>
                    }
                }).collect::<Vec<_>>()}
            </CardGrid>

            <Dialog 
                open=modal_open
                dialog_trigger=DialogTrigger::from_children(|| view! { <span></span> })
            >
                <div class="p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">
                        {if current_key.get().is_empty() { "新建渠道" } else { "编辑渠道" }}
                    </h3>
                    <div class="space-y-4">
                        <div>
                            <label class="block text-sm text-zinc-400 mb-1">名称</label>
                            <input class="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-white" type="text" />
                        </div>
                        <div>
                            <label class="block text-sm text-zinc-400 mb-1">Base URL</label>
                            <input class="w-full bg-zinc-800 border border-zinc-700 rounded-lg px-3 py-2 text-white" type="text" />
                        </div>
                    </div>
                    <div class="flex gap-3 mt-8">
                        <button 
                            class="flex-1 py-2.5 text-sm border border-zinc-700 hover:bg-zinc-800 rounded-lg text-zinc-300 transition-colors"
                            button_type="button"
                            on:click=move |_| close_modal()
                        >
                            取消
                        </button>
                        <button 
                            class="flex-1 py-2.5 text-sm bg-white text-zinc-900 rounded-lg font-medium"
                            button_type="button"
                        >
                            保存
                        </button>
                    </div>
                </div>
            </Dialog>
        </div>
    }
}