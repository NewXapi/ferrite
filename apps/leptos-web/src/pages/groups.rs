//! 分组页面组件 - 根据 dioxus admin-page-admin 的 tab-page-groups 设计，移植为 leptos 风格
//!
//!
//! - 数据用静态演示，写成普通函数（无网络请求）
//! - 使用 leptos 的 RwSignal 实现交互状态
//! - 直接从 dioxus 源码逐字搬 Tailwind class，从 assets/tailwind.css 取样式
//!

use crate::ui::CardGrid;
use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;

/// 分组数据类型 - 静态演示数据
#[derive(Clone, Debug, PartialEq)]
pub struct GroupDto {
    /// 分组标识（后端唯一 key）
    pub key: String,
    /// 分组名称
    pub name: String,
    /// 展示备注
    pub remark: String,
    /// 计费倍率
    pub ratio: f64,
    /// 状态：1: enabled, 2: disabled
    pub status: i16,
    /// 是否系统默认分组
    pub is_default: bool,
    /// 模型白名单
    pub whitelist: Vec<String>,
    /// 别名
    pub alias: Vec<String>,
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
///
/// `stats` 为 (已格式化数值, 标签) 列表，由调用方（页面）在响应式上下文中算好后传入。
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
///
/// `groups` 以 `RwSignal` 传入：筛选计数随信号实时刷新，且本组件不重挂载，
/// 搜索框输入焦点得以保留。
#[component]
#[allow(clippy::too_many_arguments)]
pub fn GroupsToolbar(
    groups: RwSignal<Vec<GroupDto>>,
    search: RwSignal<String>,
    filter_tier: RwSignal<usize>,
    selected: RwSignal<Vec<String>>,
    on_refresh: Callback<()>,
    on_new: Callback<()>,
    on_bulk_enable: Callback<()>,
    on_bulk_disable: Callback<()>,
    on_bulk_clear: Callback<()>,
) -> impl IntoView {
    let filtered_count = move || {
        let groups = groups.get();
        let search = search.get();
        let tier = filter_tier.get();
        groups
            .into_iter()
            .filter(|g| g.key.contains(&search) || g.remark.contains(&search))
            .filter(|g| match tier {
                0 => true,
                1 => g.status == 1,
                2 => g.status == 2,
                _ => true,
            })
            .count()
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
                        class=("bg-zinc-800 text-zinc-300 hover:bg-zinc-700", move || filter_tier.get() == 0)
                        on:click=move |_| filter_tier.set(0)
                    >
                        "全部"
                    </button>
                    <button
                        class="rounded-full border px-3 py-1 text-xs font-medium transition-colors"
                        class=("bg-emerald-950/60 text-emerald-400 border-emerald-800", move || filter_tier.get() == 1)
                        on:click=move |_| filter_tier.set(1)
                    >
                        "启用中"
                    </button>
                    <button
                        class="rounded-full border px-3 py-1 text-xs font-medium transition-colors"
                        class=("bg-red-950/60 text-red-400 border-red-800", move || filter_tier.get() == 2)
                        on:click=move |_| filter_tier.set(2)
                    >
                        "已停用"
                    </button>
                </div>

                <div class="ml-auto flex gap-2">
                    <button
                        class="rounded-lg bg-zinc-800 px-3 py-1.5 text-sm text-zinc-100 hover:bg-zinc-700"
                        on:click=move |_| on_refresh.run(())
                    >
                        "刷新"
                    </button>
                    <button
                        class="flex items-center gap-1 rounded-lg bg-white px-3 py-1.5 text-sm font-medium text-zinc-900 hover:bg-zinc-100"
                        on:click=move |_| on_new.run(())
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
                                format!("已选 {selected_len} 项")
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
                                    on_bulk_enable.run(());
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
                                    on_bulk_disable.run(());
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
                                    on_bulk_clear.run(());
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
///
/// `on_edit` / `on_write` / `on_retry` 需为 `Copy`：列表为每张卡片各建一份回调闭包，
/// 会多次按值捕获它们。
#[component]
pub fn GroupsList(
    filtered: Vec<GroupDto>,
    loading: bool,
    err: Option<String>,
    on_edit: Callback<String>,
    on_write: Callback<(String, WriteOp)>,
    on_retry: Callback<()>,
) -> impl IntoView {
    // 计数徽标在建视图前算好，避免 move 闭包先借用 filtered、
    // 随后下方 CardGrid 分支再把它移走。
    let total = filtered.len();
    let count_text = if total > 0 {
        format!("{total} 组")
    } else {
        "加载中…".to_string()
    };

    view! {
        <section class="scroll-mt-8 space-y-4" id="groups-sec-list">
            <div class="flex items-center justify-between">
                <h2 class="text-lg font-medium text-zinc-100">"分组列表"</h2>
                <div class="rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-300">
                    {count_text}
                </div>
            </div>

            {match err {
                Some(e) => view! {
                    <div class="rounded-2xl border-2 border-red-800/60 bg-red-950/40 py-10">
                        <div class="text-center text-red-400">{e}</div>
                        <div class="mt-3 text-center">
                            <button
                                class="rounded-lg bg-red-950/60 px-4 py-2 text-sm text-red-300 hover:bg-red-900"
                                on:click=move |_| on_retry.run(())
                            >
                                "重试"
                            </button>
                        </div>
                    </div>
                }.into_any(),
                None if loading => view! {
                    <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16">
                        <div class="text-center text-zinc-400">"正在加载分组…"</div>
                    </div>
                }.into_any(),
                None if filtered.is_empty() => view! {
                    <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16">
                        <div class="text-center text-zinc-400">"没有匹配的分组"</div>
                    </div>
                }.into_any(),
                None => view! {
                    <CardGrid>
                        {filtered.into_iter().map(|group| {
                            let is_default = group.key == "default";
                            // 每张卡的四个回调各持有独立的 key 副本，
                            // 避免同一 group 被多个 move 闭包重复移走。
                            let key_for_edit = group.key.clone();
                            let key_for_delete = group.key.clone();
                            let key_for_toggle = group.key.clone();
                            let key_for_ratio = group.key.clone();
                            let status = group.status;
                            view! {
                                <GroupCard
                                    group
                                    is_default
                                    on_edit=Callback::new(move |_: ()| on_edit.run(key_for_edit.clone()))
                                    on_delete=Callback::new(move |_: ()| on_write.run((key_for_delete.clone(), WriteOp::Delete)))
                                    on_toggle_status=Callback::new(move |_: ()| {
                                        let op = if status == 1 {
                                            WriteOp::Disable
                                        } else {
                                            WriteOp::Enable
                                        };
                                        on_write.run((key_for_toggle.clone(), op));
                                    })
                                    on_ratio_drag=Callback::new(move |v: f64| on_write.run((key_for_ratio.clone(), WriteOp::SetRatio(v))))
                                />
                            }
                        }).collect_view()}
                    </CardGrid>
                }.into_any(),
            }}
        </section>
    }
}

/// 拖拽偏移 → 倍率：按轨道宽度取比例，钳到 [0.05, 3.0]。
/// 宽度取不到（元素不可见或目标非元素）时归到下限，避免 0/0 得 NaN。
fn ratio_from_offset(x: f64, width: f64) -> f64 {
    if width > 0.0 {
        (x / width * 3.0).clamp(0.05, 3.0)
    } else {
        0.05
    }
}

/// 单个分组卡片(对齐 UserCard 风格)
///
/// `on_edit` 由卡片内的「编辑」图标与「编辑」文字按钮共用；
/// `Callback` 本身是 `Copy`，无需再手动 `clone()` 分持。
#[component]
pub fn GroupCard(
    group: GroupDto,
    is_default: bool,
    on_edit: Callback<()>,
    on_delete: Callback<()>,
    on_toggle_status: Callback<()>,
    on_ratio_drag: Callback<f64>,
) -> impl IntoView {
    let adjusting = RwSignal::new(false);
    let local_ratio = RwSignal::new(group.ratio);

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

    let ratio_badge = move || {
        if group.ratio >= 2.0 {
            view! { <span class="text-[11px] font-medium">"溢价"</span> }
        } else if group.ratio >= 1.0 {
            view! { <span class="text-[11px] font-medium">"基准"</span> }
        } else {
            view! { <span class="text-[11px] font-medium">"优惠"</span> }
        }
    };

    // `aria-label` 与标题各消费一次 `name`：先克隆一份供属性用，标题用原值。
    let name_for_label = group.name.clone();

    view! {
        <div
            class="group relative flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 hover:border-zinc-600 hover:bg-zinc-900/80 transition-all duration-200"
            role="region"
            aria-label=name_for_label
            data-testid="group-card"
        >
            <div class=("absolute left-0 top-0 bottom-0 w-1 rounded-l-xl bg-blue-950/60 border-blue-800/60", !is_default)>
            </div>

            <div class="flex items-start justify-between">
                <div class="min-w-0 flex-1">
                    <div class="flex items-center gap-2">
                        <h3 class="truncate text-base font-semibold text-zinc-100">{group.name}</h3>
                        {if is_default {
                            view! { <span class="rounded-full border border-blue-800 bg-blue-950/60 px-2 py-0.5 text-xs text-blue-300">"默认"</span> }.into_any()
                        } else {
                            None::<AnyView>.into_any()
                        }}
                    </div>
                    <p class="mt-1 text-xs text-zinc-500">{group.remark}</p>
                </div>
                <button
                    class="rounded-lg p-1.5 text-zinc-500 hover:bg-zinc-800 hover:text-zinc-200"
                    on:click=move |_| on_edit.run(())
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
                        class=format!("absolute inset-y-0 left-0 rounded-full bg-emerald-500 transition-all {}", status_tone())
                        style=move || format!("width: {}%", ratio_percent())
                        on:pointerdown=move |ev| {
                            adjusting.set(true);
                            let width = ev
                                .target()
                                .map(|t| t.unchecked_into::<leptos::web_sys::Element>())
                                .map(|el| el.client_width() as f64)
                                .unwrap_or(0.0);
                            let x = ev.offset_x() as f64;
                            let ratio = ratio_from_offset(x, width);
                            local_ratio.set(ratio);
                        }
                        on:pointermove=move |ev| {
                            if adjusting.get() {
                                let width = ev
                                    .target()
                                    .map(|t| t.unchecked_into::<leptos::web_sys::Element>())
                                    .map(|el| el.client_width() as f64)
                                    .unwrap_or(0.0);
                                let x = ev.offset_x() as f64;
                                let ratio = ratio_from_offset(x, width);
                                local_ratio.set(ratio);
                            }
                        }
                        on:pointerup=move |_| {
                            adjusting.set(false);
                            on_ratio_drag.run(local_ratio.get());
                        }
                    ></div>
                    {move || if adjusting.get() {
                        view! {
                            <div
                                class="absolute h-3.5 w-3.5 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow"
                                style=move || format!("left: {}%; transform: translateX(-50%)", ratio_percent())
                                data-testid="ratio-thumb"
                            ></div>
                        }.into_any()
                    } else {
                        None::<AnyView>.into_any()
                    }}
                </div>
            </div>

            <div class="mt-4 grid grid-cols-2 gap-2">
                <button
                    class="rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700"
                    on:click=move |_| on_edit.run(())
                >
                    "编辑"
                </button>
                <button
                    class=format!("rounded-lg border px-3 py-1.5 text-xs font-medium transition-colors {}", status_tone())
                    class=("bg-emerald-950/60 border-emerald-800 text-emerald-300", group.status == 1)
                    class=("bg-red-950/60 border-red-800 text-red-300", group.status == 2)
                    on:click=move |_| on_toggle_status.run(())
                >
                    {move || if group.status == 1 { "停用" } else { "启用" }}
                </button>
                <button
                    class=format!(
                        "rounded-lg border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-300 transition-colors hover:bg-zinc-700 {}",
                        if is_default { "opacity-40 cursor-not-allowed" } else { "" }
                    )
                    disabled=move || is_default
                    on:click=move |_| {
                        if !is_default {
                            on_delete.run(());
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
                    }.into_any()
                } else {
                    None::<AnyView>.into_any()
                }}
            </div>
        </div>
    }
}

/// 分页器
///
/// `on_change` 用 `Callback` 承载：页码按钮在循环里各建一份闭包，
/// `Callback` 是 `Copy` 且 `Send + Sync`，可安全多路捕获。
#[component]
pub fn Pager(total: usize, page: RwSignal<usize>, on_change: Callback<usize>) -> impl IntoView {
    let total_pages = total.div_ceil(10); // 每页10条

    view! {
        <div class="flex justify-center gap-2">
            <button
                class="rounded-lg border border-zinc-700 bg-zinc-900 px-3 py-1.5 text-sm text-zinc-300 disabled:opacity-40"
                disabled=move || page.get() == 0
                on:click=move |_| {
                    if page.get() > 0 {
                        page.set(page.get() - 1);
                        on_change.run(page.get() - 1);
                    }
                }
            >
                "上一页"
            </button>
            {move || (0..=total_pages.min(4)).map(|p| {
                view! {
                    <button
                        class=move || format!(
                            "rounded-lg border px-3 py-1.5 text-sm font-medium transition-colors {}",
                            if p == page.get() {
                                "bg-white text-zinc-900 border-zinc-300"
                            } else {
                                "bg-zinc-900 text-zinc-300 border-zinc-700 hover:bg-zinc-800"
                            }
                        )
                        on:click=move |_| {
                            page.set(p);
                            on_change.run(p);
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
                        on_change.run(page.get() + 1);
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
    /// 删除分组
    Delete,
    /// 启用分组
    Enable,
    /// 停用分组
    Disable,
    /// 修改计费倍率
    SetRatio(f64),
}

/// 分组管理主组件
#[component]
pub fn GroupsPage() -> impl IntoView {
    let groups = RwSignal::new(sample_groups());
    let loading = RwSignal::new(false);
    let err = RwSignal::new(None::<String>);
    let search = RwSignal::new(String::new());
    let filter_tier = RwSignal::new(0usize); // 0: all, 1: enabled, 2: disabled
    let selected = RwSignal::new(Vec::<String>::new());
    let current_page = RwSignal::new(0usize);
    let modal_state = RwSignal::new(ModalState::Closed);

    let filtered_groups = move || {
        let groups = groups.get();
        let search = search.get();
        let tier = filter_tier.get();
        let page = current_page.get();
        let size = 10;
        let filtered: Vec<GroupDto> = groups
            .into_iter()
            .filter(|g| g.key.contains(&search) || g.remark.contains(&search))
            .filter(|g| match tier {
                0 => true,
                1 => g.status == 1,
                2 => g.status == 2,
                _ => true,
            })
            .collect();
        let paged = page_slice(&filtered, page, size);
        paged.to_vec()
    };

    let total_groups = move || groups.get().len();
    let enabled_count = move || groups.get().iter().filter(|g| g.status == 1).count();
    let disabled_count = move || groups.get().iter().filter(|g| g.status == 2).count();
    let avg_ratio = move || {
        let groups = groups.get();
        if groups.is_empty() {
            0.0
        } else {
            let sum: f64 = groups.iter().map(|g| g.ratio).sum();
            sum / groups.len() as f64
        }
    };
    let custom_ratio_count = move || groups.get().iter().filter(|g| g.ratio != 1.0).count();

    let stats = move || {
        vec![
            (total_groups().to_string(), "总分组数"),
            (enabled_count().to_string(), "启用中"),
            (disabled_count().to_string(), "已停用"),
            (format!("{:.2}×", avg_ratio()), "平均倍率"),
            (custom_ratio_count().to_string(), "非基准倍率"),
        ]
    };

    let on_refresh = Callback::new(move |_: ()| {
        loading.set(true);
        err.set(None);
        set_timeout(
            move || {
                groups.set(sample_groups());
                loading.set(false);
            },
            std::time::Duration::from_millis(300),
        );
    });

    let on_new = Callback::new(move |_: ()| {
        modal_state.set(ModalState::New);
    });

    let on_bulk_enable = Callback::new(move |_: ()| {
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
        groups.set(updated);
        if !results.is_empty() {
            let summary = format!(
                "批量操作：{} 成功，{} 失败",
                results.iter().filter(|(_, ok)| *ok).count(),
                results.iter().filter(|(_, ok)| !ok).count()
            );
            err.set(Some(summary));
        }
        selected.set(Vec::new());
    });

    let on_bulk_disable = Callback::new(move |_: ()| {
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
        groups.set(updated);
        if !results.is_empty() {
            let summary = format!(
                "批量操作：{} 成功，{} 失败",
                results.iter().filter(|(_, ok)| *ok).count(),
                results.iter().filter(|(_, ok)| !ok).count()
            );
            err.set(Some(summary));
        }
        selected.set(Vec::new());
    });

    let on_bulk_clear = Callback::new(move |_: ()| {
        selected.set(Vec::new());
    });

    let on_edit = Callback::new(move |key: String| {
        modal_state.set(ModalState::Edit(key));
    });

    let on_write = Callback::new(move |(key, op): (String, WriteOp)| match op {
        WriteOp::Delete => {
            let mut updated = groups.get().clone();
            updated.retain(|g| g.key != key);
            groups.set(updated);
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
            groups.set(updated);
        }
        WriteOp::SetRatio(ratio) => {
            let mut updated = groups.get().clone();
            for g in updated.iter_mut() {
                if g.key == key {
                    g.ratio = ratio;
                    break;
                }
            }
            groups.set(updated);
        }
    });

    let on_retry = Callback::new(move |_: ()| {
        on_refresh.run(());
    });

    let on_submit = Callback::new(move |_: ()| {
        on_refresh.run(());
        modal_state.set(ModalState::Closed);
    });

    let on_cancel = Callback::new(move |_: ()| {
        modal_state.set(ModalState::Closed);
    });

    view! {
        <div class="flex flex-col gap-6" role="region" aria-label="分组管理">
            {move || view! { <GroupsStatsSection stats=stats() /> }}

            <GroupsToolbar
                groups=groups
                search
                filter_tier
                selected
                on_refresh
                on_new
                on_bulk_enable
                on_bulk_disable
                on_bulk_clear
            />

            {move || view! {
                <GroupsList
                    filtered=filtered_groups()
                    loading=loading.get()
                    err=err.get()
                    on_edit
                    on_write
                    on_retry
                />
            }}

            {move || view! {
                <Pager total=total_groups() page=current_page on_change=Callback::new(move |p| current_page.set(p))/>
            }}

            {move || match modal_state.get() {
                ModalState::Closed => {
                    None::<AnyView>.into_any()
                },
                ModalState::New => view! {
                    <Modal
                        title="新建分组"
                        on_close=Callback::new(move |_: ()| {
                            modal_state.set(ModalState::Closed);
                        })
                    >
                        <GroupFormModal
                            editing=false
                            group_key=None
                            name=RwSignal::new(String::new())
                            ratio=RwSignal::new("1.0".to_string())
                            remark=RwSignal::new(String::new())
                            whitelist=RwSignal::new(String::new())
                            alias_options=Vec::new()
                            f_alias=RwSignal::new(String::new())
                            on_cancel
                            on_submit
                        />
                    </Modal>
                }.into_any(),
                ModalState::Edit(key) => {
                    let group_opt = groups.get().iter().find(|g| g.key == key).cloned();
                    match group_opt {
                        Some(group) => view! {
                            <Modal
                                title="编辑分组"
                                on_close=Callback::new(move |_: ()| {
                                    modal_state.set(ModalState::Closed);
                                })
                            >
                                <GroupFormModal
                                    editing=true
                                    group_key=Some(group.key.clone())
                                    name=RwSignal::new(group.name.clone())
                                    ratio=RwSignal::new(format!("{:.2}", group.ratio))
                                    remark=RwSignal::new(group.remark.clone())
                                    whitelist=RwSignal::new(
                                        if group.whitelist.is_empty() {
                                            "".to_string()
                                        } else {
                                            group.whitelist.join(",")
                                        }
                                    )
                                    alias_options=Vec::new()
                                    f_alias=RwSignal::new(
                                        if group.alias.is_empty() {
                                            "".to_string()
                                        } else {
                                            group.alias.join(",")
                                        }
                                    )
                                    on_cancel
                                    on_submit
                                />
                            </Modal>
                        }.into_any(),
                        None => {
                            None::<AnyView>.into_any()
                        },
                    }
                },
            }}
        </div>
    }
}

/// 弹窗状态（关闭 / 新建 / 编辑指定 key 的分组）
#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(String), // key
}

/// 把用户输入的逗号/分号分隔白名单拆成模型名数组（去空、trim）。
fn parse_whitelist_raw(raw: &str) -> Vec<String> {
    raw.split([',', '，', ';', '；'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

// ============ 弹窗（自 dioxus tab-page-groups/modal.rs 移植） ============

/// 通用弹窗外壳（遮罩 + 标题栏 + 关闭按钮 + slot 内容）。
///
/// 【是什么】半透明遮罩上的居中弹窗：标题行（标题 + 右上角 ×）+ `children` 插槽。
///
/// 【交互逻辑】点遮罩或右上角关闭按钮 → `on_close` 抛回调用方；点弹窗内部
/// `stop_propagation` 阻止冒泡，不会误触发遮罩关闭。不发任何网络请求。
///
/// 【数据流】入：`title` 弹窗标题、`on_close` 关闭回调、`children` 插槽内容；
/// 出：`on_close` → 调用方关弹窗。`on_copy` 需为 `Copy`：遮罩与 × 按钮各捕获一次。
#[component]
pub fn Modal(title: &'static str, on_close: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <div
            class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm"
            on:click=move |_| on_close.run(())
        >
            <div
                class="w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl"
                on:click=|ev| ev.stop_propagation()
            >
                <div class="mb-5 flex items-center justify-between">
                    <h3 class="text-base font-semibold text-zinc-100">{title}</h3>
                    <button
                        class="rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200"
                        aria-label="关闭"
                        on:click=move |_| on_close.run(())
                    >
                        <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24" stroke-width="2">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12"></path>
                        </svg>
                    </button>
                </div>
                {children()}
            </div>
        </div>
    }
}

/// 弹窗表单输入框的共用 class（自 dioxus `MODAL_INPUT` 逐字搬运）
const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

/// 新建 / 编辑分组弹窗（双页签表单）。
///
/// 【是什么】分组的新建/编辑表单，内含两个页签：0「分组信息」（名称 / 备注 /
/// 白名单 / 倍率 + 预设 + 计费预览），1「映射别名」（输入框 + 候选 chips 多选）。
///
/// 【做什么】渲染表单、就地预览倍率；静态演示环境无网络请求，提交只回调
/// `on_submit`（由页面负责刷新与关窗）。不负责开关弹窗（由页面 `modal_state`
/// 控制）、不负责回填（页面已把初值写进传入的信号）。
///
/// 【交互逻辑】用户操作 → 组件行为：
/// - 输入框 / 白名单 / 倍率 / 别名 → 直接 `set` 对应传入信号（纯本地）。
/// - 点倍率预设按钮 → `ratio.set(预设值)`，预设高亮与计费预览随信号实时刷新。
/// - 点页签胶囊 → `active_tab.set(i)` 切换页签（仅页签容器响应，输入焦点不受影响）。
/// - 点别名候选 chip → 在 `f_alias` 里按逗号解析后加入/移出该项，再拼回字符串。
/// - 点「取消」 → `on_cancel` 抛回页面关弹窗。
/// - 点提交 → `do_submit`：名称 trim 后为空则直接返回（不提交），否则读取各字段
///   并回调 `on_submit`。
///
/// 【数据流】
/// - 对内（入）：`editing`（标题与提交文案二选一）、`group_key`（编辑时的 key，
///   None 走新建）、`name` / `ratio` / `remark` / `whitelist` / `f_alias`（页面持有的
///   表单信号，双向读写）、`alias_options`（映射别名候选名，为空时只读回退）、
///   `on_cancel` / `on_submit`。
/// - 对外（出）：信号写回停留本地表单（仅提交时读取）；`on_cancel` → 页面关弹窗；
///   `on_submit` → 页面刷新列表并关窗。
#[component]
#[allow(clippy::too_many_arguments)]
pub fn GroupFormModal(
    editing: bool,
    group_key: Option<String>,
    name: RwSignal<String>,
    ratio: RwSignal<String>,
    remark: RwSignal<String>,
    whitelist: RwSignal<String>,
    // 映射别名候选（后端 models 域列表的 name）；空 = 拉取失败/无候选，只读回退。
    alias_options: Vec<String>,
    // 映射别名草稿（逗号分隔字符串；MVP 仅登记展示，不随提交落库，文案见 tab1）
    f_alias: RwSignal<String>,
    on_cancel: Callback<()>,
    on_submit: Callback<()>,
) -> impl IntoView {
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建分组"
    };

    // 编辑弹窗双 tab：0=分组信息(名称/倍率/备注/白名单), 1=映射别名
    let active_tab = RwSignal::new(0usize);
    let tab_labels = ["分组信息", "映射别名"];
    let submitting = RwSignal::new(false);

    let preset_ratios = [
        ("0.5× 半价", "0.5"),
        ("0.8× 优惠", "0.8"),
        ("1.0× 基准", "1.0"),
        ("1.2× 溢价", "1.2"),
        ("1.5× 高配", "1.5"),
        ("2.0× 双倍", "2.0"),
    ];

    let parsed_ratio = move || ratio.get().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let do_submit = move |_| {
        // 演示页无网络提交链路：group_key 只占位读取
        // （dioxus 版此处按其选择 update_group_api / create_group_api）。
        let _has_key = group_key.is_some();
        let n = name.get().trim().to_string();
        if n.is_empty() {
            return;
        }
        let _r = ratio.get();
        let _rm = remark.get();
        let _wl = parse_whitelist_raw(&whitelist.get());
        on_submit.run(());
    };

    view! {
        // tab 栏
        <div class="mb-4">
            <div class="flex w-full flex-wrap overflow-hidden rounded-full border border-zinc-700 bg-zinc-950 text-xs sm:w-fit">
                {tab_labels.into_iter().enumerate().map(|(i, label)| {
                    view! {
                        <button
                            data-testid=format!("group-tab-{i}")
                            role="tab"
                            aria-selected=move || (i == active_tab.get()).to_string()
                            class=move || if i == active_tab.get() {
                                "min-w-[30%] flex-1 truncate border-r border-zinc-800 bg-zinc-100 px-3 py-1.5 text-center text-xs font-medium text-zinc-900 last:border-r-0 sm:min-w-0 sm:flex-none"
                            } else {
                                "min-w-[30%] flex-1 truncate border-r border-zinc-800 px-3 py-1.5 text-center text-xs text-zinc-400 transition-colors last:border-r-0 hover:bg-zinc-800 hover:text-zinc-200 sm:min-w-0 sm:flex-none"
                            }
                            on:click=move |_| active_tab.set(i)
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>

        <div class="space-y-4">
            // 页签内容：外层只依赖 active_tab，具体字段的实时刷新下沉到
            // 各自的 class / text / prop:value 嵌套闭包，输入时不会重挂载。
            {move || if active_tab.get() == 0 {
                view! {
                    <div class="space-y-4">
                        <div>
                            <label class="mb-1.5 block text-xs text-zinc-400">"分组标识 (英文唯一标识)"</label>
                            <input
                                class=MODAL_INPUT
                                type="text"
                                data-testid="group-name"
                                placeholder="例如: vip, claude, fast"
                                prop:value=move || name.get()
                                disabled=move || editing && name.get() == "default"
                                on:input=move |ev| name.set(event_target_value(&ev))
                            />
                            {move || if editing && name.get() == "default" {
                                view! { <p class="mt-1 text-xs text-zinc-500">"默认分组标识不可更改"</p> }.into_any()
                            } else {
                                None::<AnyView>.into_any()
                            }}
                        </div>

                        <div>
                            <label class="mb-1.5 block text-xs text-zinc-400">"展示备注 (可选)"</label>
                            <input
                                class=MODAL_INPUT
                                type="text"
                                data-testid="group-remark"
                                placeholder="例如: VIP会员专线、高峰备用组"
                                prop:value=move || remark.get()
                                on:input=move |ev| remark.set(event_target_value(&ev))
                            />
                        </div>

                        <div>
                            <label class="mb-1.5 block text-xs text-zinc-400">"模型白名单 (逗号分隔,可选)"</label>
                            <input
                                class=MODAL_INPUT
                                type="text"
                                data-testid="group-whitelist"
                                placeholder="例如: gpt-4o, claude-3.5"
                                prop:value=move || whitelist.get()
                                on:input=move |ev| whitelist.set(event_target_value(&ev))
                            />
                            <p class="mt-1 text-[11px] text-zinc-500">"留空 = 全模型可用;填了 = 仅这些模型"</p>
                        </div>

                        <div>
                            <label class="mb-1.5 block text-xs text-zinc-400">"计费倍率 (ratio ≥ 0)"</label>
                            <input
                                class="w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none font-mono"
                                type="text"
                                data-testid="group-ratio"
                                placeholder="1.0"
                                prop:value=move || ratio.get()
                                on:input=move |ev| ratio.set(event_target_value(&ev))
                            />
                        </div>

                        // 快捷预设按钮
                        <div class="space-y-1.5">
                            <p class="text-[11px] text-zinc-500">"快捷倍率预设"</p>
                            <div class="flex flex-wrap gap-1.5">
                                {preset_ratios.into_iter().map(|(lbl, val)| {
                                    view! {
                                        <button
                                            class=move || {
                                                let current = ratio.get().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
                                                let preset = val.parse::<f64>().unwrap_or(0.0);
                                                let tone = if (current - preset).abs() < 0.001 {
                                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                                } else {
                                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                                };
                                                format!("rounded-lg border px-2.5 py-1 text-xs transition-colors {tone}")
                                            }
                                            on:click=move |_| ratio.set(val.to_string())
                                        >
                                            {lbl}
                                        </button>
                                    }
                                }).collect_view()}
                            </div>
                        </div>

                        // 计费预览: 仅保留「该分组实际扣费」
                        <div class="rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5">
                            <div class="flex justify-between font-medium">
                                <span class="text-zinc-300">"该分组实际扣费"</span>
                                <span class=move || {
                                    let current = parsed_ratio();
                                    if current < 1.0 {
                                        "text-emerald-400"
                                    } else if current > 1.0 {
                                        "text-amber-400"
                                    } else {
                                        "text-zinc-200"
                                    }
                                }>
                                    {move || format!("{} 点额度", (100.0 * parsed_ratio()).round() as i64)}
                                </span>
                            </div>
                        </div>
                    </div>
                }.into_any()
            } else {
                let options_empty = alias_options.is_empty();
                view! {
                    <div class="space-y-3">
                        <div>
                            <label class="mb-1.5 block text-xs text-zinc-400">"映射别名 (多选,逗号分隔)"</label>
                            <input
                                class=MODAL_INPUT
                                type="text"
                                data-testid="group-alias"
                                placeholder="例如: gpt-4o, claude-3.5"
                                prop:value=move || f_alias.get()
                                on:input=move |ev| f_alias.set(event_target_value(&ev))
                            />
                            <p class="mt-1 text-[11px] text-zinc-500">"本 MVP 仅登记, 后端暂无映射列; 留空 = 不映射"</p>
                        </div>
                        {if options_empty {
                            view! { <p class="text-xs text-zinc-500">"暂无可选模型别名"</p> }.into_any()
                        } else {
                            view! {
                                <div class="flex flex-wrap gap-1.5">
                                    {alias_options.iter().map(|opt| {
                                        // 三个独立的 String 副本，分别交给 class 闭包、
                                        // 点击闭包与文本，避免同一值被多次 move。
                                        let opt_class = opt.clone();
                                        let opt_click = opt.clone();
                                        let opt_text = opt.clone();
                                        view! {
                                            <button
                                                class=move || {
                                                    let picked = parse_whitelist_raw(&f_alias.get())
                                                        .iter()
                                                        .any(|a| a == &opt_class);
                                                    let tone = if picked {
                                                        "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                                    } else {
                                                        "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                                    };
                                                    format!("rounded-lg border px-2.5 py-1 text-xs transition-colors {tone}")
                                                }
                                                data-testid="group-alias-opt"
                                                on:click=move |_| {
                                                    let mut cur = parse_whitelist_raw(&f_alias.get());
                                                    if let Some(pos) = cur.iter().position(|a| a == &opt_click) {
                                                        cur.remove(pos);
                                                    } else {
                                                        cur.push(opt_click.clone());
                                                    }
                                                    f_alias.set(cur.join(", "));
                                                }
                                            >
                                                {opt_text}
                                            </button>
                                        }
                                    }).collect_view()}
                                </div>
                            }.into_any()
                        }}
                    </div>
                }.into_any()
            }}
        </div>

        <div class="mt-6 flex gap-3">
            <button
                class="flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800"
                data-testid="group-cancel"
                on:click=move |_| on_cancel.run(())
            >
                "取消"
            </button>
            <button
                class="flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40"
                data-testid="group-submit"
                disabled=move || submitting.get()
                on:click=do_submit
            >
                {submit_label}
            </button>
        </div>
    }
}
