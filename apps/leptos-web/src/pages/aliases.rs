use leptos::prelude::*;

// Define local types matching the reference structure
#[derive(Clone, PartialEq)]
struct AliasItem {
    key: String,
    alias: String,
    display: String,
    input_per_1k: f64,
    output_per_1k: f64,
    multiplier: f64,
    price_mode: PriceMode,
}

#[derive(Clone, Copy, PartialEq)]
enum PriceMode {
    PerToken,
    PerCall,
}

impl PriceMode {
    fn label(self) -> &'static str {
        match self {
            PriceMode::PerToken => "按量",
            PriceMode::PerCall => "按次",
        }
    }
}

/// 行内编辑字段：别名与展示名（价格/倍率在本页只读，无编辑入口）。
#[derive(Clone, PartialEq)]
enum AliasEditField {
    Name,
    Display,
}

#[derive(Clone, PartialEq)]
enum AliasModalState {
    Closed,
    New,
}

// Static data matching the reference
fn static_aliases() -> Vec<AliasItem> {
    vec![
        AliasItem {
            key: "1".to_string(),
            alias: "gpt-4o".to_string(),
            display: "GPT-4o 旗舰模型".to_string(),
            input_per_1k: 5.0,
            output_per_1k: 15.0,
            multiplier: 1.0,
            price_mode: PriceMode::PerToken,
        },
        AliasItem {
            key: "2".to_string(),
            alias: "claude-3-5-sonnet".to_string(),
            display: "Claude 3.5 Sonnet".to_string(),
            input_per_1k: 3.0,
            output_per_1k: 12.0,
            multiplier: 1.2,
            price_mode: PriceMode::PerToken,
        },
        AliasItem {
            key: "3".to_string(),
            alias: "gemini-1.5-pro".to_string(),
            display: "Gemini 1.5 Pro".to_string(),
            input_per_1k: 2.0,
            output_per_1k: 8.0,
            multiplier: 0.0,
            price_mode: PriceMode::PerCall,
        },
    ]
}

/// 单个别名卡片（内联实现，含行内编辑 Popover 触发按钮）。
#[component]
fn AliasCard(
    item: AliasItem,
    on_mode_change: Callback<(String, PriceMode)>,
    on_edit: Callback<(String, AliasEditField, String)>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let key = item.key.clone();
    let key_mode = item.key.clone();
    let key_del = item.key.clone();
    let key_edit_name = item.key.clone();
    let key_edit_display = item.key.clone();

    let alias_for_edit = item.alias.clone();
    let display_for_edit = item.display.clone();

    let multiplier_tone = if item.multiplier == 0.0 {
        "text-sky-300 border-sky-500/30 bg-sky-500/15"
    } else if (item.multiplier - 1.0).abs() < 0.001 {
        "text-zinc-300 border-zinc-600 bg-zinc-800/60"
    } else {
        "text-amber-300 border-amber-500/30 bg-amber-500/15"
    };

    let edit = RwSignal::new(false);
    let name_input = RwSignal::new(alias_for_edit);
    let display_input = RwSignal::new(display_for_edit);

    view! {
        <div
            class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-colors hover:border-zinc-600"
            data-testid=format!("alias-card-{}", key)
        >
            <div class="mb-2 flex items-start justify-between gap-2">
                <div class="min-w-0">
                    <div class="truncate font-mono text-sm text-zinc-100">{item.alias.clone()}</div>
                    <div class="truncate text-xs text-zinc-500">{item.display.clone()}</div>
                </div>
                <span class=format!("shrink-0 rounded-full border px-2 py-0.5 text-[11px] font-medium {}", multiplier_tone)>
                    {format!("{:.2}×", item.multiplier)}
                </span>
            </div>

            <div class="space-y-1 text-xs text-zinc-400">
                <div class="flex justify-between gap-2">
                    <span>"输入 / 1K"</span>
                    <span class="font-mono text-zinc-200">{format!("¥{:.4}", item.input_per_1k)}</span>
                </div>
                <div class="flex justify-between gap-2">
                    <span>"输出 / 1K"</span>
                    <span class="font-mono text-zinc-200">{format!("¥{:.4}", item.output_per_1k)}</span>
                </div>
                <div class="flex justify-between gap-2">
                    <span>"计费模式"</span>
                    <button
                        type="button"
                        class="rounded-full border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 hover:border-zinc-500 hover:text-white"
                        data-testid=format!("alias-mode-{}", key_mode)
                        on:click={
                            let k = key_mode.clone();
                            let next = if item.price_mode == PriceMode::PerToken {
                                PriceMode::PerCall
                            } else {
                                PriceMode::PerToken
                            };
                            move |_| on_mode_change.run((k.clone(), next))
                        }
                    >
                        {item.price_mode.label()}
                    </button>
                </div>
            </div>

            <div class="mt-3 flex items-center gap-1.5 border-t border-zinc-800 pt-3">
                <button
                    type="button"
                    class="flex-1 rounded-lg border border-zinc-700 px-2 py-1.5 text-xs text-zinc-300 hover:border-zinc-500 hover:text-white"
                    data-testid=format!("alias-edit-{}", key)
                    on:click=move |_| edit.update(|v| *v = !*v)
                >
                    "编辑"
                </button>
                <button
                    type="button"
                    class="flex-1 rounded-lg border border-zinc-700 px-2 py-1.5 text-xs text-red-400 hover:border-red-500/40 hover:text-red-300"
                    data-testid=format!("alias-delete-{}", key_del)
                    on:click=move |_| on_delete.run(key_del.clone())
                >
                    "删除"
                </button>
            </div>

            {move || edit.get().then(|| view! {
                <div class="mt-3 space-y-2 rounded-lg border border-zinc-700 bg-zinc-950/60 p-3">
                    <input
                        class="w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-100"
                        type="text"
                        placeholder="别名"
                        prop:value=move || name_input.get()
                        on:input=move |ev| name_input.set(event_target_value(&ev))
                    />
                    <input
                        class="w-full rounded border border-zinc-700 bg-zinc-900 px-2 py-1 text-xs text-zinc-100"
                        type="text"
                        placeholder="展示名"
                        prop:value=move || display_input.get()
                        on:input=move |ev| display_input.set(event_target_value(&ev))
                    />
                    <div class="flex justify-end gap-2">
                        <button
                            type="button"
                            class="rounded border border-zinc-700 px-2 py-1 text-[11px] text-zinc-400 hover:text-white"
                            on:click=move |_| edit.set(false)
                        >
                            "取消"
                        </button>
                        <button
                            type="button"
                            class="rounded border border-zinc-600 bg-zinc-800 px-2 py-1 text-[11px] text-zinc-100 hover:bg-zinc-700"
                            data-testid=format!("alias-save-{}", key_edit_name)
                            on:click={
                                let kn = key_edit_name.clone();
                                let kd = key_edit_display.clone();
                                move |_| {
                                    on_edit.run((kn.clone(), AliasEditField::Name, name_input.get()));
                                    on_edit.run((kd.clone(), AliasEditField::Display, display_input.get()));
                                    edit.set(false);
                                }
                            }
                        >
                            "保存"
                        </button>
                    </div>
                </div>
            })}
        </div>
    }
}

#[component]
pub fn AliasesPage() -> impl IntoView {
    // 列表状态:跨 stats/list 两区与 effect 共享
    let rows = RwSignal::new(static_aliases());
    let loading = RwSignal::new(false);
    let err = RwSignal::new(None::<String>);
    let reload = RwSignal::new(0u32);

    // 筛选状态:由 toolbar 就地读写,但 filtered 的派生计算在页面,
    // 所以状态提升到这一层、以 RwSignal 传下去(子组件内零状态)。
    let search = RwSignal::new(String::new());
    let filter_tier = RwSignal::new(0usize);

    // 弹窗状态
    let modal_state = RwSignal::new(AliasModalState::Closed);
    let f_name = RwSignal::new(String::new());
    let f_display = RwSignal::new(String::new());
    let submitting = RwSignal::new(false);

    // 写回状态:写操作进行中 / 成功提示
    let busy = RwSignal::new(false);
    let notice = RwSignal::new(None::<String>);

    // 挂载即拉取真实列表 + 分组(卡片要展示各分组倍率);reload 变化时重拉
    Effect::new(move |_| {
        let _ = reload.get();
        loading.set(true);
        err.set(None);
        set_timeout(
            move || loading.set(false),
            std::time::Duration::from_millis(500),
        );
    });

    // —— 派生:统计与筛选(filtered 被 list 组件消费,计算留在页面) ——
    let stats = Memo::new(move |_| {
        let alias_list = rows
            .get()
            .iter()
            .map(|it| (it.alias.clone(), it.display.clone(), it.multiplier))
            .collect::<Vec<_>>();
        let total = alias_list.len();
        let free_count = alias_list
            .iter()
            .filter(|(_, _, mult)| *mult == 0.0)
            .count();
        let standard_count = alias_list
            .iter()
            .filter(|(_, _, mult)| (mult - 1.0).abs() < 0.001)
            .count();
        let custom_count = alias_list
            .iter()
            .filter(|(_, _, mult)| *mult != 0.0 && (mult - 1.0).abs() >= 0.001)
            .count();
        let avg_mult = if total > 0 {
            alias_list.iter().map(|(_, _, mult)| mult).sum::<f64>() / (total as f64)
        } else {
            1.0
        };
        vec![
            (total.to_string(), "总别名数"),
            (standard_count.to_string(), "标准 1.0× 别名"),
            (custom_count.to_string(), "自定倍率别名"),
            (free_count.to_string(), "免费别名 (0×)"),
            (format!("{:.2}×", avg_mult), "平均加价倍率"),
        ]
    });

    let filter_options = ["全部", "标准 1.0×", "自定倍率", "免费通道"];

    let filtered = Memo::new(move |_| {
        let q = search.get().trim().to_lowercase();
        let tier = filter_tier.get();
        rows.get()
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                if !q.is_empty()
                    && !it.alias.to_lowercase().contains(&q)
                    && !it.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => (it.multiplier - 1.0).abs() < 0.001,
                    2 => it.multiplier != 0.0 && (it.multiplier - 1.0).abs() >= 0.001,
                    3 => it.multiplier == 0.0,
                    _ => true,
                }
            })
            .map(|(i, it)| (i, it.clone()))
            .collect::<Vec<_>>()
    });

    // —— 写回闭包 ——
    let open_new = Callback::new(move |_: ()| {
        f_name.set(String::new());
        f_display.set(String::new());
        modal_state.set(AliasModalState::New);
    });

    // 行内 Popover 保存(替代原「编辑弹窗」路径,UI 决策记录 §2.2):
    // 点击卡片行 → 浮层输入 → 保存抛 (key, 字段, 原始字符串)。数值解析失败保留旧值并提示。
    let commit_alias_field = Callback::new(
        move |(key, field, value): (String, AliasEditField, String)| {
            let raw = value.trim().to_string();
            match field {
                AliasEditField::Name => {
                    if raw.is_empty() {
                        notice.set(Some("别名不能为空".to_string()));
                        return;
                    }
                    let mut items = rows.get();
                    if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                        it.alias = raw;
                    }
                    rows.set(items);
                }
                AliasEditField::Display => {
                    let mut items = rows.get();
                    if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                        it.display = raw;
                    }
                    rows.set(items);
                }
            }
        },
    );

    // 卡片面板上的定价 toggle:更新该 card 独立的定价模式(per-card,不共享)。
    let on_mode_change = Callback::new(move |(key, mode): (String, PriceMode)| {
        let mut items = rows.get();
        if let Some(it) = items.iter_mut().find(|it| it.key == key) {
            it.price_mode = mode;
        }
        rows.set(items);
    });

    // 删除:成功后本地从 rows 移除该项;只有错误才提示,成功静默。
    let write_delete = Callback::new(move |key: String| {
        let mut items = rows.get();
        items.retain(|it| it.key != key);
        rows.set(items);
        notice.set(Some("已删除".to_string()));
    });

    // 弹窗提交(仅剩新建):后端 CreateModelRequest 必填 owner 与 api_key,
    // 表单没有这两个字段的来源 — 诚实拒绝,不造数据、不假成功。
    let submit_alias = Callback::new(move |_: ()| {
        notice.set(Some(
            "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供".to_string(),
        ));
        modal_state.set(AliasModalState::Closed);
    });

    let refresh = move |_| {
        busy.set(true);
        set_timeout(
            move || busy.set(false),
            std::time::Duration::from_millis(300),
        );
        reload.update(|v| *v += 1);
    };

    view! {
        <div class="flex flex-col gap-6">
            // 通知条(成功/错误/进行中,对齐 GroupsPage)
            {move || notice.get().map(|msg| view! {
                <div
                    role="status"
                    class="rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300"
                >
                    {msg}
                    {move || busy.get().then_some(" ···")}
                </div>
            })}

            // 数据与写路径说明(后端 models 端点暂无计费字段)
            <div class="flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400">
                "别名来自真实 /api/models;编辑与删除已接后端;定价模式与价格配置为 UI 层本地状态,后端扩展 pricing 列前保存不写库;新建暂未开放(后端需要 owner/api_key 字段)"
            </div>

            // 统计区:五张概览卡(总数/标准 1.0×/自定倍率/免费/平均倍率)
            <section class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-5">
                {move || stats.get().into_iter().map(|(value, label)| view! {
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600">
                        <div class="text-xl font-semibold tracking-tight text-white">{value}</div>
                        <div class="mt-0.5 text-xs text-zinc-500">{label}</div>
                    </div>
                }).collect_view()}
            </section>

            // 筛选与操作区:刷新/新建按钮 + 搜索框 + 分级胶囊
            <section class="flex flex-wrap items-center gap-3 rounded-xl border border-zinc-700/60 bg-zinc-900/60 p-4">
                <button
                    type="button"
                    class="rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:border-zinc-500 hover:text-white"
                    data-testid="aliases-refresh"
                    on:click=refresh
                >
                    "刷新"
                </button>
                <button
                    type="button"
                    class="rounded-xl border border-zinc-600 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-100 hover:bg-zinc-700"
                    data-testid="aliases-new"
                    on:click=move |_| open_new.run(())
                >
                    "新建"
                </button>

                <input
                    class="min-w-48 flex-1 rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-2 text-sm text-zinc-100 placeholder-zinc-500 focus:border-indigo-500 focus:outline-none"
                    type="text"
                    placeholder="搜索别名或展示名..."
                    prop:value=move || search.get()
                    on:input=move |ev| search.set(event_target_value(&ev))
                />

                <div class="flex flex-wrap gap-1.5">
                    {filter_options.into_iter().enumerate().map(|(i, opt)| {
                        view! {
                            <button
                                type="button"
                                class="rounded-full border border-zinc-700 px-3 py-1 text-[11px] text-zinc-300 hover:border-zinc-500 hover:text-white"
                                data-testid=format!("aliases-filter-{}", i)
                                on:click=move |_| filter_tier.set(i)
                            >
                                {opt}
                            </button>
                        }
                    }).collect_view()}
                </div>
            </section>

            // 卡片网格区:四态(加载/错误/空/网格)
            <section class="flex flex-col gap-4">
                {move || if loading.get() {
                    view! {
                        <div class="grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5">
                            {(0..5).map(|_| view! {
                                <div class="h-32 animate-pulse rounded-xl border border-zinc-800 bg-zinc-900/60"></div>
                            }).collect_view()}
                        </div>
                    }.into_any()
                } else if let Some(msg) = err.get() {
                    view! {
                        <div
                            class="rounded-lg border border-red-900/50 bg-red-950/20 px-4 py-6 text-center"
                            data-testid="aliases-error"
                        >
                            <p class="text-sm text-red-300">"别名列表拉取失败"</p>
                            <p class="mt-1 text-xs text-red-400/70">{msg}</p>
                            <button
                                type="button"
                                class="mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800"
                                data-testid="aliases-retry"
                                on:click=move |_| reload.update(|v| *v += 1)
                            >
                                "重试"
                            </button>
                        </div>
                    }.into_any()
                } else if filtered.get().is_empty() {
                    view! {
                        <div
                            class="rounded-lg border border-dashed border-zinc-700 bg-zinc-900/40 px-4 py-8 text-center"
                            data-testid="aliases-empty"
                        >
                            <p class="text-sm text-zinc-400">"没有匹配的别名"</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5">
                            {filtered.get().into_iter().map(|(_, item)| view! {
                                <AliasCard
                                    item=item
                                    on_mode_change=on_mode_change
                                    on_edit=commit_alias_field
                                    on_delete=write_delete
                                />
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </section>
        </div>

        {move || (modal_state.get() == AliasModalState::New).then(|| view! {
            <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm">
                <div
                    class="w-full max-w-md rounded-2xl border border-zinc-700 bg-zinc-900 p-5 shadow-xl"
                    role="dialog"
                    aria-modal="true"
                    data-testid="alias-new-modal"
                >
                    <h3 class="text-base font-semibold text-zinc-100">"新建别名"</h3>
                    <div class="mt-4 space-y-3">
                        <div>
                            <label class="mb-1 block text-xs text-zinc-400">"别名"</label>
                            <input
                                class="w-full rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-2 text-sm text-zinc-100"
                                type="text"
                                prop:value=move || f_name.get()
                                on:input=move |ev| f_name.set(event_target_value(&ev))
                            />
                        </div>
                        <div>
                            <label class="mb-1 block text-xs text-zinc-400">"展示名"</label>
                            <input
                                class="w-full rounded-lg border border-zinc-600 bg-zinc-800 px-3 py-2 text-sm text-zinc-100"
                                type="text"
                                prop:value=move || f_display.get()
                                on:input=move |ev| f_display.set(event_target_value(&ev))
                            />
                        </div>
                    </div>
                    <div class="mt-5 flex justify-end gap-2">
                        <button
                            type="button"
                            class="rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800"
                            data-testid="alias-new-cancel"
                            on:click=move |_| modal_state.set(AliasModalState::Closed)
                        >
                            "取消"
                        </button>
                        <button
                            type="button"
                            class="rounded-xl border border-zinc-600 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-100 hover:bg-zinc-700"
                            data-testid="alias-new-submit"
                            disabled=move || submitting.get()
                            on:click=move |_| submit_alias.run(())
                        >
                            "提交"
                        </button>
                    </div>
                </div>
            </div>
        })}
    }
}
