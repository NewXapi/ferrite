use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

#[derive(Clone, Debug, PartialEq)]
pub struct NodeInfo {
    pub key: String,
    pub name: String,
    pub layer: u8,
    pub color: String,
    pub connections: Vec<String>,
}

fn sample_nodes() -> Vec<NodeInfo> {
    vec![
        NodeInfo {
            key: "group-default".to_string(),
            name: "默认分组".to_string(),
            layer: 1,
            color: "#e5484d".to_string(),
            connections: vec!["gpt-4o".to_string(), "claude-3".to_string()],
        },
        NodeInfo {
            key: "group-vip".to_string(),
            name: "VIP".to_string(),
            layer: 1,
            color: "#3e9bff".to_string(),
            connections: vec!["gpt-4o".to_string()],
        },
        NodeInfo {
            key: "alias-gpt-4o".to_string(),
            name: "gpt-4o".to_string(),
            layer: 2,
            color: "#a78bfa".to_string(),
            connections: vec!["OpenAI".to_string(), "DeepSeek".to_string()],
        },
        NodeInfo {
            key: "alias-claude-3".to_string(),
            name: "claude-3".to_string(),
            layer: 2,
            color: "#30a46c".to_string(),
            connections: vec!["Anthropic".to_string()],
        },
        NodeInfo {
            key: "dispatch-openai".to_string(),
            name: "OpenAI".to_string(),
            layer: 3,
            color: "#f97316".to_string(),
            connections: vec![],
        },
        NodeInfo {
            key: "dispatch-anthropic".to_string(),
            name: "Anthropic".to_string(),
            layer: 3,
            color: "#d946ef".to_string(),
            connections: vec![],
        },
        NodeInfo {
            key: "dispatch-deepseek".to_string(),
            name: "DeepSeek".to_string(),
            layer: 3,
            color: "#fb923c".to_string(),
            connections: vec![],
        },
    ]
}

#[component]
pub fn NetworkPage() -> impl IntoView {
    let nodes = sample_nodes();
    let selected_node = RwSignal::new(None::<String>);

    let groups: Vec<_> = nodes.iter().filter(|n| n.layer == 1).collect();
    let aliases: Vec<_> = nodes.iter().filter(|n| n.layer == 2).collect();
    let dispatches: Vec<_> = nodes.iter().filter(|n| n.layer == 3).collect();

    view! {
        <div class="relative w-full h-full bg-zinc-950 flex overflow-hidden">
            <div class="flex-1 relative p-8 bg-zinc-900/50">
                <div class="flex items-center justify-between mb-6">
                    <div>
                        <h1 class="text-xl font-semibold text-white">"网络拓扑"</h1>
                        <p class="text-zinc-400 text-sm">"分组 → 模型别名 → 调度渠道"</p>
                    </div>
                    <div class="flex gap-3">
                        <Button button_type="button" variant="outline" class="px-4 py-1.5 text-sm font-medium rounded-md">
                            "适配"
                        </Button>
                        <Button button_type="button" variant="outline" class="px-4 py-1.5 text-sm font-medium rounded-md">
                            "设置"
                        </Button>
                        <Button button_type="button" variant="outline" class="px-4 py-1.5 text-sm font-medium rounded-md">
                            "导入"
                        </Button>
                    </div>
                </div>

                <div class="relative h-[620px] border border-zinc-800 rounded-xl bg-zinc-950 overflow-hidden">
                    <svg class="absolute inset-0 w-full h-full pointer-events-none z-10" viewBox="0 0 1200 620">
                        <path d="M 180 140 Q 380 220 420 310" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                        <path d="M 280 140 Q 480 200 520 310" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                        <path d="M 380 140 Q 520 240 620 310" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                        <path d="M 520 360 Q 680 420 720 510" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                        <path d="M 620 360 Q 760 410 820 510" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                        <path d="M 720 360 Q 840 430 920 510" stroke="#475569" stroke-width="3" fill="none" stroke-dasharray="2,3"/>
                    </svg>

                    <div class="absolute top-[80px] left-[80px] flex flex-col gap-6">
                        {groups.iter().enumerate().map(|(i, node)| {
                            let left = 40 + i * 60;
                            view! {
                                <div
                                    class="w-28 h-10 rounded-lg border-2 cursor-pointer hover:shadow-xl hover:scale-105 transition-all flex items-center px-3"
                                    style=move || format!("left: {}px; border-color: {};", left, node.color)
                                    class=("bg-zinc-900", selected_node.get() == Some(node.key.clone()))
                                    on:click=move |_| selected_node.set(Some(node.key.clone()))
                                >
                                    <div class="w-3 h-3 rounded-full mr-2" style=move || format!("background-color: {};", node.color)></div>
                                    <span class="text-xs font-medium text-white truncate">{node.name}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>

                    <div class="absolute top-[260px] left-[420px] flex flex-col gap-4">
                        {aliases.iter().enumerate().map(|(i, node)| {
                            let left = 20 + (i % 2) * 140;
                            let top = if i > 1 { 80 } else { 0 };
                            view! {
                                <div
                                    class="w-28 h-10 rounded-lg border-2 cursor-pointer hover:shadow-xl hover:scale-105 transition-all flex items-center px-3 absolute"
                                    style=move || format!("left: {}px; top: {}px; border-color: {};", left, top, node.color)
                                    class=("bg-zinc-900", selected_node.get() == Some(node.key.clone()))
                                    on:click=move |_| selected_node.set(Some(node.key.clone()))
                                >
                                    <div class="w-3 h-3 rounded-full mr-2" style=move || format!("background-color: {};", node.color)></div>
                                    <span class="text-xs font-medium text-white truncate">{node.name}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>

                    <div class="absolute top-[480px] left-[720px] flex flex-col gap-4">
                        {dispatches.iter().map(|node| {
                            view! {
                                <div
                                    class="w-28 h-10 rounded-lg border-2 cursor-pointer hover:shadow-xl hover:scale-105 transition-all flex items-center px-3"
                                    style=move || format!("border-color: {};", node.color)
                                    class=("bg-zinc-900", selected_node.get() == Some(node.key.clone()))
                                    on:click=move |_| selected_node.set(Some(node.key.clone()))
                                >
                                    <div class="w-3 h-3 rounded-full mr-2" style=move || format!("background-color: {};", node.color)></div>
                                    <span class="text-xs font-medium text-white truncate">{node.name}</span>
                                </div>
                            }
                        }).collect_view()}
                    </div>

                    <div class="absolute bottom-6 right-6 bg-zinc-900/90 border border-zinc-700 rounded-lg p-4 max-w-[260px]">
                        <div class="text-[10px] text-zinc-400 leading-tight">
                            "滚轮缩放 · 拖拽平移 · 点击节点检视 · 拖拽端口连线"
                        </div>
                    </div>
                </div>
            </div>

            <div class="w-80 border-l border-zinc-800 bg-zinc-900 flex flex-col">
                <div class="shrink-0 border-b border-zinc-800 px-4 py-3">
                    <div class="flex items-center justify-between mb-3">
                        <div class="flex items-center gap-2">
                            <div class="w-2 h-2 rounded-full bg-emerald-400"></div>
                            <span class="text-sm font-medium text-white">"节点检视"</span>
                        </div>
                        <button class="text-zinc-400 hover:text-white text-xl leading-none">"×"</button>
                    </div>

                    <div class="flex border-b border-zinc-800 text-xs">
                        <button class="flex-1 py-2.5 text-white border-b-2 border-white font-medium">"节点"</button>
                        <button class="flex-1 py-2.5 text-zinc-500 hover:text-zinc-300">"设置"</button>
                        <button class="flex-1 py-2.5 text-zinc-500 hover:text-zinc-300">"导入"</button>
                    </div>
                </div>

                <div class="flex-1 overflow-auto p-5 space-y-6 text-sm">
                    {move || {
                        match selected_node.get() {
                            Some(key) => {
                                let node = nodes.iter().find(|n| n.key == key).cloned();
                                view! {
                                    <For
                                        each=move || {
                                            let mut v = Vec::new();
                                            if let Some(n) = node {
                                                v.push(n);
                                            }
                                            v
                                        }
                                        key=|node| node.key.clone()
                                        children=|node| view! {
                                            <Card class="mb-4">
                                                <CardHeader>
                                                    <CardTitle>{node.name.clone()}</CardTitle>
                                                </CardHeader>
                                                <CardContent>
                                                    <div class="space-y-3">
                                                        <div>
                                                            <div class="text-[11px] text-zinc-500 mb-1">"层级"</div>
                                                            <div class="text-white font-mono text-sm">{format!("层 {}", node.layer)}</div>
                                                        </div>
                                                        <div>
                                                            <div class="text-[11px] text-zinc-500 mb-1">"颜色"</div>
                                                            <div class="flex items-center gap-2">
                                                                <div class="w-4 h-4 rounded" style=move || format!("background-color: {};", node.color)></div>
                                                                <span class="text-zinc-400 font-mono text-xs">{node.color}</span>
                                                            </div>
                                                        </div>
                                                        <div>
                                                            <div class="text-[11px] text-zinc-500 mb-2">"连接"</div>
                                                            <div class="flex flex-wrap gap-2">
                                                                {node.connections.iter().map(|conn| {
                                                                    view! {
                                                                        <div class="inline-flex items-center gap-1.5 bg-zinc-800 border border-zinc-600 rounded-full px-3 py-1 text-xs text-zinc-300">
                                                                            {conn}
                                                                        </div>
                                                                    }
                                                                }).collect_view()}
                                                            </div>
                                                        </div>
                                                    </div>
                                                </CardContent>
                                            </Card>
                                        }
                                    />
                                }.into_view()
                            }
                            None => view! {
                                <div class="text-center text-zinc-500 py-16">
                                    <div class="text-zinc-400">"点击节点以查看详情"</div>
                                </div>
                            }.into_view()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
