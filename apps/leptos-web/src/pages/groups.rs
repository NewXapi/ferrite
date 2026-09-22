//! 分组页面组件 - 根据 dioxus admin-page-admin 的 tab-page-groups 设计，移植为 leptos 风格
//!
//!
//! - 数据用静态演示，写成普通函数（无网络请求）
//! - 使用 leptos 的 RwSignal 实现交互状态
//! - 直接从 dioxus 源码逐字搬 Tailwind class，从 assets/tailwind.css 取样式
//!

use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;

/// 分组数据类型 - 静态演示数据
#[derive(Clone, Debug, PartialEq)]
pub struct GroupDto {
    pub key: String,
    pub name: String,
    pub remark: String,
    pub ratio: f64,
    pub status: i16, // 1: enabled, 2: disabled
    pub is_default: bool,
    pub whitelist: Vec<String>, // 模型白名单
    pub alias: Vec<String>, // 别名
}

/// 静态演示数据生成函数
fn sample_groups() -> Vec<GroupDto> {
    vec![
        GroupDto {
            key: "default".to_string(),
            name: "默认".to_string(),
            remark: "系统默认分组".to_string(),
            ratio: 1.0,
            status: 1,
            is_default: true,
            whitelist: vec![],
            alias: vec![],
        },
        GroupDto {
            key: "vip".to_string(),
            name: "VIP".to_string(),
            remark: "VIP会员专线，高峰备用组".to_string(),
            ratio: 2.0,
            status: 1,
            is_default: false,
            whitelist: vec!["gpt-4o".to_string(), "claude-3.5".to_string()],
            alias: vec![],
        },
        GroupDto {
            key: "fast".to_string(),
            name: "Fast".to_string(),
            remark: "高速通道".to_string(),
            ratio: 0.5,
            status: 2,
            is_default: false,
            whitelist: vec![],
            alias: vec!["gpt-4o".to_string()],
        },
    ]
}

/// 分组概览统计区
#[component]
pub fn GroupsStatsSection(stats: Vec<(String, &'static str)>) -> impl IntoView {
    view! {
        <section class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-5">
            {stats.into_iter().map(|(value, label)| view! { <StatCard value label/> }).collect_view()}
        </section>
    }
}

/// 概览统计卡(单张:大号数值 + 小号标签)
#[component]
pub fn StatCard(value: String, label: &'static str) -> impl IntoView {
    view! {
        <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 hover:border-zinc-600 transition-colors">
            <div class="text-xl font-semibold tracking-tight text-white">{value}</div>
            <div class="mt-0.5 text-xs text-zinc-500">{label}</div>
        </div>
    }
}

/// 分组筛选与批量操作区
#[component]
pub fn GroupsToolbar(
    groups: Vec<GroupDto>,
    filter_options: Vec<String>,
    search: RwSignal<String>,
    filter_tier: RwSignal<usize>,
    selected: RwSignal<Vec<String>>,
    on_refresh: impl Fn() + 'static,
    on_new: impl Fn() + 'static,
    on_bulk_enable: impl Fn() + 'static,
    on_bulk_disable: impl Fn() + 'static,
    on_bulk_clear: impl Fn() + 'static,
) -> impl IntoView {
    let filtered_count = move || {
        let groups = groups.clone();
        let search = search.get();
        let tier = filter_tier.get();
        groups.into_iter().filter(|g| {
            g.key.contains(&search) || g.remark.contains(&search)
        }).filter(|g| {
            match tier {
                0 => true,
                1 => g.status == 1,
                2 => g.status == 2,
                _ => true,
            }
        }).count()
    };
    
    view! {
        <section class="rounded-2xl border border-zinc-800 bg-zinc-900/40 p-4">
            <div class="flex flex-wrap items-center gap-3">
                <div class="relative">
                    <input
                        type="text"
                        class="w-64 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none"
                        placeholder="搜索分组标识或备注..."
                        prop:value=move || search.get()
                        on:input=move |ev| search.set(event_target_value(&ev))
                    />
                </div>
                
                <div class="flex gap-2">
                    <button
                        class="rounded-full border px-3 py-1 text-xs font-medium transition-colors"
                        class=("bg-zinc-800 text-zinc-300 hover:bg-zinc-700", filter_tier.get() == 0)
                        on:click=move |_| filter_tier.set(0)
                    >
                        "全部"
                    </button>
                    <button
                        class=("bg-emerald-950/60 text-emerald-400 border-emerald-800", filter_tier.get() == 1)
                        on:click=move |_| filter_tier.set(1)
                    >
                        "启用中"
                    </button>
                    <button
                        class=("bg-red-950/60 text-red-400 border-red-800", filter_tier.get() == 2)
                        on:click=move |_| filter_tier.set(2)
                    >
                        "已停用"
                    </button>
                </div>
                
                <div class="ml-auto flex gap-2">
                    <button
                        class="rounded-lg bg-zinc-800 px-3 py-1.5 text-sm text-zinc-100 hover:bg-zinc-700"
                        on:click=move |_| on_refresh()
                    >
                        "刷新"
                    </button>
                    <button
                        class="flex items-center gap-1 rounded-lg bg-white px-3 py-1.5 text-sm font-medium text-zinc-900 hover:bg-zinc-100"
                        on:click=move |_| on_new()
                    >
                        "✚ 新建分组"
                    </button>
                </div>
            </div>
            
            <div class="mt-3 flex items-center justify-between text-sm text-zinc-400">
                <div>
                    {move || format!("共找到 {} 个分组", filtered_count())}
                </div>
                <div class="flex items-center gap-2">
                    <div class="rounded-full bg-zinc-800 px-3 py-1 text-xs">
                        {move || {
                            let selected_len = selected.get().len();
                            if selected_len > 0 {
                                format!("已选 {} 项", selected_len)
                            } else {
                                "未选择".to_string()
                            }
                        }}
                    </div>
                    <div class="flex gap-2">
                        <button
                            class="rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-100 hover:bg-zinc-700 disabled:opacity-40"
                            disabled=move || selected.get().is_empty()
                            on:click=move |_| {
                                if !selected.get().is_empty() {
                                    on_bulk_enable();
                                }
                            }
                        >
                            "批量启用"
                        </button>
                        <button
                            class="rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-100 hover:bg-zinc-700 disabled:opacity-40"
                            disabled=move || selected.get().is_empty()
                            on:click=move |_| {
                                if !selected.get().is_empty() {
                                    on_bulk_disable();
                                }
                            }
                        >
                            "批量停用"
                        </button>
                        <button
                            class="rounded-lg bg-zinc-800 px-3 py-1.5 text-xs text-zinc-100 hover:bg-zinc-700 disabled:opacity-40"
                            disabled=move || selected.get().is_empty()
                            on:click=move |_| {
                                if !selected.get().is_empty() {
                                    on_bulk_clear();
                                }
                            }
                        >
                            "清除"
                        </button>
                    </div>
                </div>
            </div>
        </section>
    }
}

/// 分组列表(加载中 / 错误 / 空态 / 数据)
#[component]
pub fn GroupsList(
    filtered: Vec<GroupDto>,
    loading: bool,
    err: Option<String>,
    on_edit: impl Fn(String) + 'static,
    on_write: impl Fn((String, WriteOp)) + 'static,
    on_retry: impl Fn() + 'static,
) -> impl IntoView {
    view! {
        <section class="scroll-mt-8 space-y-4" id="groups-sec-list">
            <div class="flex items-center justify-between">
                <h2 class="text-lg font-medium text-zinc-100">"分组列表"</h2>
                <div class="rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-300">
                    {move || {
                        let total = filtered.len();
                        if total > 0 {
                            format!("{} 组", total)
                        } else {
                            "加载中…".to_string()
                        }
                    }}
                </div>
            </div>
            
            {match err {
                Some(e) => view! {
                    <div class="rounded-2xl border-2 border-red-800/60 bg-red-950/40 py-10">
                        <div class="text-center text-red-400">{e}</div>
                        <div class="mt-3 text-center">
                            <button
                                class="rounded-lg bg-red-950/60 px-4 py-2 text-sm text-red-300 hover:bg-red-900"
                                on:click=move |_| on_retry()
                            >
                                "重试"
                            </button>
                        </div>
                    </div>
                }.into_view(),
                None if loading => view! {
                    <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16">
                        <div class="text-center text-zinc-400">"正在加载分组…"</div>
                    </div>
                }.into_view(),
                None if filtered.is_empty() => view! {
                    <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16">
                        <div class="text-center text-zinc-400">"没有匹配的分组"</div>
                    </div>
                }.into_view(),
                None => view! {
                    <CardGrid>
                        {filtered.into_iter().map(|group| {
                            let is_default = group.key == "default";
                            view! {
                                <GroupCard
                                    group
                                    is_default
                                    on_edit: move || on_edit(group.key.clone()),
                                    on_delete: move || on_write((group.key.clone(), WriteOp::Delete)),
                                    on_toggle_status: move || {
                                        let op = if group.status == 1 {
                                            WriteOp::Disable
                                        } else {
                                            WriteOp::Enable
                                        };
                                        on_write((group.key.clone(), op));
                                    },
                                    on_ratio_drag: move |v| on_write((group.key.clone(), WriteOp::SetRatio(v))),
                                />
                            }
                        }).collect_view()}
                    </CardGrid>
                }.into_view(),
            }}
        </section>
    }
}

/// 单个分组卡片(对齐 UserCard 风格)
#[component]
pub fn GroupCard(
    group: GroupDto,
    is_default: bool,
    on_edit: impl Fn() + 'static,
    on_delete: impl Fn() + 'static,
    on_toggle_status: impl Fn() + 'static,
    on_ratio_drag: impl Fn(f64) + 'static,
) -> impl IntoView {
    let (adjusting, set_adjusting) = signal(false);
    let (local_ratio, set_local_ratio) = signal(group.ratio);
    
    let ratio_percent = move || {
        let max = 3.0;
        let val = local_ratio.get().max(0.05).min(max);
        (val / max * 100.0) as f32
    };
    
    let status_tone = move || {
        if group.status == 1 {
            "text-emerald-400"
        } else {
            "text-red-400"
        }
    };
    
    let status_text = move || {
        if group.status == 1 {
            "启用中"
        } else {
            "已停用"
        }
    };
    
    let badge_tone = move || {
        if group.ratio >= 2.0 {
            "text-amber-400"
        } else if group.ratio >= 1.0 {
            "text-zinc-300"
        } else {
            "text-emerald-400"
        }
    };
    
    let ratio_badge = move || {
        if group.ratio >= 2.0 {
            view! { <span class="text-[11px] font-medium">"溢价"</span> }
        } else if group.ratio >= 1.0 {
            view! { <span class="text-[11px] font-medium">"基准"</span> }
        } else {
            view! { <span class="text-[11px] font-medium">"优惠"</span> }
        }
    };
    
    view! {
        <div
            class="group relative flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 hover:border-zinc-600 hover:bg-zinc-900/80 transition-all duration-200"
            role="region"
            aria-label=group.name
            data-testid="group-card"
        >
            <div class=("absolute left-0 top-0 bottom-0 w-1 rounded-l-xl bg-blue-950/60 border-blue-800/60", !is_default)>
            </div>
            
            <div class="flex items-start justify-between">
                <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                        <h3 class="truncate text-base font-semibold text-zinc-100">{group.name}</h3>
                        {if is_default {
                            view! { <span class="rounded-full border border-blue-800 bg-blue-950/60 px-2 py-0.5 text-xs text-blue-300">"默认"</span> }
                        } else {
                            view! { <></> }
                        }}
                    </div>
                    <p class="mt-1 text-xs text-zinc-500">{group.remark}</p>
                </div>
                <button
                    class="rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200"
                    on:click=move |_| on_edit()
                >
                    <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z"></path>
                    </svg>
                </button>
            </div>
            
            <div class="mt-3 flex items-center justify-between">
                <div class="flex items-center gap-2">
                    <span class="text-[11px] font-medium text-zinc-400">"计费倍率"</span>
                    {ratio_badge()}
                </div>
                <div class="text-xs font-mono">
                    {format!("{:.2}×", group.ratio)}
                </div>
            </div>
            
            <div class="mt-4">
                <div class="relative h-4 w-full">
                    <div class="absolute inset-0 h-1.5 w-full rounded-full bg-zinc-800"></div>
                    <div
                        class=("absolute inset-y-0 left-0 rounded-full bg-emerald-500 transition-all", status_tone())
                        style=move || format!("width: {}%", ratio_percent())
                        on:pointerdown=move |ev| {
                            set_adjusting(true);
                            let rect = ev.target_rect().unwrap();
                            let x = ev.client_x() - rect.left();
                            let width = rect.width();
                            let ratio = (x / width * 3.0).max(0.05).min(3.0);
                            set_local_ratio(ratio);
                        }
                        on:pointermove=move |ev| {
                            if adjusting.get() {
                                let rect = ev.target_rect().unwrap();
                                let x = ev.client_x() - rect.left();
                                let width = rect.width();
                                let ratio = (x / width * 3.0).max(0.05).min(3.0);
                                set_local_ratio(ratio);
                            }
                        }
                        on:pointerup=move |_| {
                            set_adjusting(false);
                            on_ratio_drag(local_ratio.get());
                        }
                    ></div>
                    {move || if adjusting.get() {
                        view! {
                            <div
                                class="absolute h-3.5 w-3.5 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow"
                                style=move || format!("left: {}%; transform: translateX(-50%)", ratio_percent())
                                data-testid="ratio-thumb"
                            ></div>
                        }
                    } else {
                        view! { <></> }
                    }}
                </div>
            </div>
            
            <div class="mt-4 grid grid-cols-2 gap-2">
                <button
                    class="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700"
                    on:click=move |_| on_edit()
                >
                    "编辑"
                </button>
                <button
                    class=("rounded-lg border px-3 py-1.5 text-xs font-medium transition-colors", status_tone())
                    class=("bg-emerald-950/60 border-emerald-800 text-emerald-300", group.status == 1)
                    class=("bg-red-950/60 border-red-800 text-red-300", group.status == 2)
                    on:click=move |_| on_toggle_status()
                >
                    {move || if group.status == 1 { "停用" } else { "启用" }}
                </button>
                <button
                    class=("rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700", if is_default { "opacity-40 cursor-not-allowed" } else { "" })
                    disabled=move || is_default
                    on:click=move |_| {
                        if !is_default {
                            on_delete();
                        }
                    }
                >
                    "删除"
                </button>
                {if is_default {
                    view! {
                        <button
                            class="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-500 cursor-not-allowed"
                            disabled=true
                        >
                            "内置"
                        </button>
                    }
                } else {
                    view! { <></> }
                }}
            </div>
        </div>
    }
}

/// 状态/属性小徽标(圆角胶囊,配色由调用方按语义传入)
#[component]
pub fn Badge(text: String, tone: &'static str) -> impl IntoView {
    view! {
        <span class="rounded-full border px-2 py-0.5 text-[11px] font-medium" {tone}> {text} </span>
    }
}

/// 分页器
#[component]
pub fn Pager(total: usize, page: RwSignal<usize>, on_change: impl Fn(usize) + 'static) -> impl IntoView {
    let total_pages = (total + 9) / 10; // 每页10条
    
    view! {
        <div class="flex justify-center gap-2">
            <button
                class="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-300 disabled:opacity-40"
                disabled=move || page.get() == 0
                on:click=move |_| {
                    if page.get() > 0 {
                        page.set(page.get() - 1);
                        on_change(page.get() - 1);
                    }
                }
            >
                "上一页"
            </button>
            {move || (0..=total_pages.min(4)).map(|p| {
                view! {
                    <button
                        class=("rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors", 
                            if p == page.get() {
                                "bg-white text-zinc-900 border-zinc-300"
                            } else {
                                "bg-zinc-900 text-zinc-300 border-zinc-700 hover:bg-zinc-800"
                            }
                        )
                        on:click=move |_| {
                            page.set(p);
                            on_change(p);
                        }
                    >
                        {p + 1}
                    </button>
                }
            }).collect_view()}
            <button
                class="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-300 disabled:opacity-40"
                disabled=move || page.get() >= total_pages - 1
                on:click=move |_| {
                    if page.get() < total_pages - 1 {
                        page.set(page.get() + 1);
                        on_change(page.get() + 1);
                    }
                }
            >
                "下一页"
            </button>
        </div>
    }
}

/// 分页辅助函数
fn page_slice<T>(arr: &[T], page: usize, size: usize) -> &[T] {
    let start = page * size;
    let end = (start + size).min(arr.len());
    &arr[start..end]
}

/// 写操作枚举
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WriteOp {
    Delete,
    Enable,
    Disable,
    SetRatio(f64),
}

/// 分组管理主组件
#[component]
pub fn GroupsPage() -> impl IntoView {
    let (groups, set_groups) = signal(sample_groups());
    let (loading, set_loading) = signal(false);
    let (err, set_err) = signal(None::<String>);
    let (search, set_search) = signal(String::new());
    let (filter_tier, set_filter_tier) = signal(0usize); // 0: all, 1: enabled, 2: disabled
    let (selected, set_selected) = signal(Vec::<String>::new());
    let (current_page, set_page) = signal(0usize);
    let (modal_state, set_modal_state) = signal(ModalState::Closed);
    
    let filtered_groups = move || {
        let groups = groups.get();
        let search = search.get();
        let tier = filter_tier.get();
        let page = current_page.get();
        let size = 10;
        let filtered: Vec<GroupDto> = groups.into_iter().filter(|g| {
            g.key.contains(&search) || g.remark.contains(&search)
        }).filter(|g| {
            match tier {
                0 => true,
                1 => g.status == 1,
                2 => g.status == 2,
                _ => true,
            }
        }).collect();
        let paged = page_slice(&filtered, page, size);
        paged.to_vec()
    };
    
    let total_groups = move || groups.get().len();
    let enabled_count = move || groups.get().iter().filter(|g| g.status == 1).count();
    let disabled_count = move || groups.get().iter().filter(|g| g.status == 2).count();
    let avg_ratio = move || {
        let groups = groups.get();
        if groups.is_empty() { 0.0 } else {
            let sum: f64 = groups.iter().map(|g| g.ratio).sum();
            sum / groups.len() as f64
        }
    };
    let custom_ratio_count = move || groups.get().iter().filter(|g| g.ratio != 1.0).count();
    
    let stats = vec![
        (total_groups().to_string(), "总分组数"),
        (enabled_count().to_string(), "启用中"),
        (disabled_count().to_string(), "已停用"),
        (format!("{:.2}×", avg_ratio()), "平均倍率"),
        (custom_ratio_count().to_string(), "非基准倍率"),
    ];
    
    let on_refresh = move || {
        set_loading(true);
        set_err(None);
        set_timeout(
            move || {
                set_groups(sample_groups());
                set_loading(false);
            },
            300,
        );
    };
    
    let on_new = move || {
        set_modal_state(ModalState::New);
    };
    
    let on_bulk_enable = move || {
        let keys_to_enable = selected.get();
        let mut updated = groups.get().clone();
        let mut results = Vec::new();
        for key in keys_to_enable.iter() {
            for g in updated.iter_mut() {
                if g.key == *key && g.status == 2 {
                    g.status = 1;
                    results.push((key.clone(), true));
                }
            }
        }
        set_groups(updated);
        if !results.is_empty() {
            let summary = format!("批量操作：{} 成功，{} 失败", 
                results.iter().filter(|(_, ok)| *ok).count(),
                results.iter().filter(|(_, ok)| !ok).count()
            );
            set_err(Some(summary));
        }
        set_selected(Vec::new());
    };
    
    let on_bulk_disable = move || {
        let keys_to_disable = selected.get();
        let mut updated = groups.get().clone();
        let mut results = Vec::new();
        for key in keys_to_disable.iter() {
            for g in updated.iter_mut() {
                if g.key == *key && g.status == 1 {
                    g.status = 2;
                    results.push((key.clone(), true));
                }
            }
        }
        set_groups(updated);
        if !results.is_empty() {
            let summary = format!("批量操作：{} 成功，{} 失败", 
                results.iter().filter(|(_, ok)| *ok).count(),
                results.iter().filter(|(_, ok)| !ok).count()
            );
            set_err(Some(summary));
        }
        set_selected(Vec::new());
    };
    
    let on_bulk_clear = move || {
        set_selected(Vec::new());
    };
    
    let on_edit = move |key: String| {
        set_modal_state(ModalState::Edit(key));
    };
    
    let on_write = move |(key, op): (String, WriteOp)| {
        match op {
            WriteOp::Delete => {
                let mut updated = groups.get().clone();
                updated.retain(|g| g.key != key);
                set_groups(updated);
            }
            WriteOp::Enable | WriteOp::Disable => {
                let status = if matches!(op, WriteOp::Enable) { 1 } else { 2 };
                let mut updated = groups.get().clone();
                for g in updated.iter_mut() {
                    if g.key == key {
                        g.status = status;
                        break;
                    }
                }
                set_groups(updated);
            }
            WriteOp::SetRatio(ratio) => {
                let mut updated = groups.get().clone();
                for g in updated.iter_mut() {
                    if g.key == key {
                        g.ratio = ratio;
                        break;
                    }
                }
                set_groups(updated);
            }
        }
    };
    
    let on_retry = move || {
        on_refresh();
    };
    
    let on_submit = move || {
        on_refresh();
        set_modal_state(ModalState::Closed);
    };
    
    let on_cancel = move || {
        set_modal_state(ModalState::Closed);
    };
    
    view! {
        <div class="flex flex-col gap-6" role="region" aria-label="分组管理">
            <GroupsStatsSection stats />
            
            <GroupsToolbar
                groups=groups.get()
                filter_options=vec!["全部".to_string(), "启用中".to_string(), "已停用".to_string()]
                search
                filter_tier
                selected
                on_refresh
                on_new
                on_bulk_enable
                on_bulk_disable
                on_bulk_clear
            />
            
            <GroupsList
                filtered=filtered_groups()
                loading=loading.get()
                err=err.get()
                on_edit
                on_write
                on_retry
            />
            
            <Pager total=total_groups() page=current_page on_change=set_page/>
            
            {match modal_state.get() {
                ModalState::Closed => view! { <></> }.into_view(),
                ModalState::New => view! {
                    <Modal
                        title="新建分组"
                        on_close=move |_| set_modal_state(ModalState::Closed)
                    >
                        <GroupFormModal
                            editing=false
                            group_key=None
                            name=Signal::new("".to_string())
                            ratio=Signal::new("1.0".to_string())
                            remark=Signal::new("".to_string())
                            whitelist=Signal::new("".to_string())
                            alias_options=Vec::new()
                            f_alias=Signal::new("".to_string())
                            on_cancel=move |_| set_modal_state(ModalState::Closed)
                            on_submit=move |_| on_submit()
                        />
                    </Modal>
                }.into_view(),
                ModalState::Edit(key) => {
                    let group_opt = groups.get().iter().find(|g| g.key == key).cloned();
                    match group_opt {
                        Some(group) => view! {
                            <Modal
                                title="编辑分组"
                                on_close=move |_| set_modal_state(ModalState::Closed)
                            >
                                <GroupFormModal
                                    editing=true
                                    group_key=Some(group.key.clone())
                                    name=Signal::new(group.name.clone())
                                    ratio=Signal::new(format!("{:.2}", group.ratio))
                                    remark=Signal::new(group.remark.clone())
                                    whitelist=Signal::new(
                                        if group.whitelist.is_empty() {
                                            "".to_string()
                                        } else {
                                            group.whitelist.join(",")
                                        }
                                    )
                                    alias_options=Vec::new()
                                    f_alias=Signal::new(
                                        if group.alias.is_empty() {
                                            "".to_string()
                                        } else {
                                            group.alias.join(",")
                                        }
                                    )
                                    on_cancel=move |_| set_modal_state(ModalState::Closed)
                                    on_submit=move |_| on_submit()
                                />
                            </Modal>
                        }.into_view(),
                        None => view! { <></> }.into_view(),
                    }
                },
            }}
        </div>
    }
}

#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(String), // key
}

fn event_target_value(event: &web_sys::Event) -> String {
    let event = event.dyn_ref::<web_sys::Event>().unwrap();
    event.as_event_target().unwrap().unchecked_into::<web_sys::Event>().target().unwrap().dyn_into::<web_sys::HtmlInputElement>().unwrap().value()
}