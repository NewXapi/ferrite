use dioxus::prelude::*;
use serde_json::json;
use client::ApiClient;
use contract::api::admin::{GroupDto, GroupUpsertRequest};
use ui::SegmentedCapsule;
use ui::ActionButtonGroup;
use ui::ActionSpec;
use ui::ActionTone;
use crate::api::{create_group_api, update_group_api};
use super::shared::parse_whitelist_raw;
// ============ 组件 ============

#[component]
pub fn StatCard(value: String, label: &'static str) -> Element {
    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3 transition-colors hover:border-zinc-600",
            p { class: "text-xl font-semibold tracking-tight text-white", "{value}" }
            p { class: "mt-0.5 text-xs text-zinc-500", "{label}" }
        }
    }
}

#[component]
pub fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}

/// 单个分组卡片 (对齐 UserCard 风格)
#[component]
pub fn GroupCard(
    group: GroupDto,
    is_default: bool,
    on_edit: EventHandler<()>,
    on_delete: EventHandler<()>,
    on_toggle_status: EventHandler<()>,
    // 倍率滑条拖动结束时回调新倍率 (卡片本地即时改, 由页面层写回后端)
    on_ratio_drag: EventHandler<f64>,
) -> Element {
    let m = group.ratio;

    // 倍率滑条两段式交互:
    // - 平时只显示静态色块 (无 thumb, 指针划过不吸附)。
    // - 第一次点击轨道 → 进入调整态: 出现 thumb, 拖动即时预览 (不写库)。
    // - 第二次点击 (含拖动后松开再点) → 确认: on_ratio_drag 写回后端, thumb 消失。
    // 选中态存 adjusting signal; local_ratio 只在调整态被拖动改写。
    let mut adjusting = use_signal(|| false);
    let mut local_ratio = use_signal(|| m);
    let show_thumb = adjusting();
    // 调整态预览 live; 静态态显示后端值 m (拖动失败/未确认不污染展示)
    let display_ratio = if show_thumb { local_ratio() } else { m };
    // 滑条域 0.0–2.0 (UI 上限截断; 写回原值可能 >2, 视觉封顶)
    const RATIO_MAX: f64 = 2.0;
    let live_pct = ((display_ratio / RATIO_MAX) * 100.0).clamp(0.0, 100.0);

    // 倍率状态与徽标 (滑条宽度由 live_pct 实时算, 不再固定 bar_width_pct)
    let (mult_badge_text, mult_badge_tone, bar_tone) = if (m - 1.0).abs() < 0.001 {
        (
            "基准 1.00×".to_string(),
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-zinc-200",
        )
    } else if m < 1.0 {
        let discount = ((1.0 - m) * 100.0).round() as i64;
        (
            format!("优惠 {m:.2}× (-{discount}%)"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
        )
    } else {
        let markup = ((m - 1.0) * 100.0).round() as i64;
        (
            format!("溢价 {m:.2}× (+{markup}%)"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
        )
    };

    let status_text = if group.status == 1 {
        "启用中"
    } else {
        "已停用"
    };
    let status_tone = if group.status == 1 {
        "border-emerald-500/30 bg-emerald-500/20 text-emerald-400"
    } else {
        "border-zinc-700 bg-zinc-800/80 text-zinc-400"
    };

    let example_cost = (100.0 * m).round() as i64;

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            "data-testid": "group-card",

            div { class: "space-y-3",
                // 头部:分组名 + 默认标签 (卡内勾选框已移除, 多选改到列表外的 chips 区)
                div { class: "min-w-0",
                    div { class: "flex items-center justify-between gap-2",
                        h3 { class: "truncate text-sm font-medium text-zinc-100", "{group.name}" }
                        if is_default {
                            span { class: "min-w-0 max-w-[140px] truncate rounded bg-blue-950/60 border border-blue-800/60 px-1.5 py-0.5 text-[10px] font-mono text-blue-300 shrink-0",
                                title: "默认",
                                "默认"
                            }
                        }
                    }
                }

                // 徽标行: 倍率徽标 + (default) 系统内置 + 状态; 非默认组不再单独挂「自定义分组」
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: mult_badge_text, tone: mult_badge_tone }
                    if is_default {
                        Badge { text: "系统内置".to_string(), tone: "border-blue-500/30 bg-blue-500/20 text-blue-300" }
                    }
                    Badge { text: status_text.to_string(), tone: status_tone }
                }

                // 倍率滑条 (两段式): 静态时只有色块无 thumb (指针划过不吸附);
                // 第一次点击 → 进入调整态出现 thumb 可拖预览; 第二次点击 → 确认写回 + thumb 消失。
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "计费倍率" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{display_ratio:.2}" }
                    }
                    div {
                        class: "relative h-4 w-full touch-none select-none",
                        class: if show_thumb { "relative h-4 w-full cursor-ew-resize touch-none select-none" } else { "relative h-4 w-full touch-none select-none" },
                        id: "group-ratio-slider",
                        "data-testid": "ratio-slider",
                        onclick: move |e| {
                            if show_thumb {
                                // 第二次点击 = 确认: 写回预览值, 退出调整态
                                on_ratio_drag.call(local_ratio.peek().max(0.05));
                                adjusting.set(false);
                            } else {
                                // 第一次点击 = 进入调整态, 从点击处开始预览
                                local_ratio.set(m);
                                adjusting.set(true);
                                // 立即把预览挪到点击位置 (复用同一换算)
                                let doc = web_sys::window().and_then(|w| w.document());
                                let el = doc
                                    .as_ref()
                                    .and_then(|d| d.get_element_by_id("group-ratio-slider"));
                                let client_x = e.client_coordinates().x;
                                if let Some(el) = el {
                                    let r = el.get_bounding_client_rect();
                                    let frac = ((client_x - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0);
                                    let next = ((frac * RATIO_MAX) / 0.05).round() * 0.05;
                                    local_ratio.set(next.max(0.05));
                                }
                            }
                        },
                        onpointermove: move |e| {
                            // 仅调整态且有按键按住时拖动预览; 静态态划过不动 (不吸附)。
                            // 用 held_buttons 而非 trigger_button: pointermove 在触屏上
                            // 无「触发键」, held_buttons 按住触摸时含 Primary, 兼容触屏拖动。
                            if !show_thumb || e.held_buttons().is_empty() {
                                return;
                            }
                            // 拖动: client_x 相对轨道 rect 换算比例 (w-full 响应式,
                            // 每次从 document 取 bounding rect)
                            let doc = web_sys::window().and_then(|w| w.document());
                            let el = doc
                                .as_ref()
                                .and_then(|d| d.get_element_by_id("group-ratio-slider"));
                            let client_x = e.client_coordinates().x;
                            let frac = if let Some(el) = el {
                                let r = el.get_bounding_client_rect();
                                ((client_x - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0)
                            } else {
                                0.5
                            };
                            let next = ((frac * RATIO_MAX) / 0.05).round() * 0.05;
                            local_ratio.set(next.max(0.05));
                        },
                        // track 灰底 + 左色块
                        div { class: "absolute left-0 top-1/2 h-1.5 w-full -translate-y-1/2 rounded-full bg-zinc-800" }
                        div { class: "absolute left-0 top-1/2 h-1.5 -translate-y-1/2 rounded-full {bar_tone} transition-colors duration-200",
                            style: "width: {live_pct}%;"
                        }
                        // thumb 只在调整态渲染
                        if show_thumb {
                            div {
                                class: "absolute top-1/2 h-3.5 w-3.5 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-zinc-100 bg-zinc-900 shadow",
                                "data-testid": "ratio-thumb",
                                style: "left: {live_pct}%;"
                            }
                        }
                    }
                }

                // 详情指标行: 保留「100额度实扣」与「调度作用域」;
                // 「费率模式」行与弹窗预览的「标准消耗」同义 (溢价/优惠/标准 已在徽标表达), 删除。
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "100额度实扣" }
                        span { class: "font-medium text-zinc-200 font-mono", "{example_cost} 点" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "调度作用域" }
                        span { class: "font-medium text-zinc-200", "全模型匹配" }
                    }
                }
            }

            // 底部操作按钮组: [编辑] [启用/停用] [删除/内置]
            // 用 ui::ActionButtonGroup 统一渲染; 下标语义: 0=编辑 1=启停 2=删除/内置
            div { class: "mt-1",
                ActionButtonGroup {
                    testid_prefix: "group-actions".to_string(),
                    on_press: move |i: usize| {
                        match i {
                            0 => on_edit.call(()),
                            1 => on_toggle_status.call(()),
                            2 => on_delete.call(()),
                            _ => {}
                        }
                    },
                    actions: {
                        let mut acts = vec![ActionSpec {
                            label: "编辑".to_string(),
                            tone: ActionTone::Neutral,
                            disabled: false,
                            testid: Some("edit-group".into()),
                        }];
                        // 启停位: 启用中→停用(默认组为禁用占位); 已停用→启用
                        if group.status == 1 {
                            acts.push(ActionSpec {
                                label: "停用".to_string(),
                                tone: if is_default { ActionTone::Disabled } else { ActionTone::Neutral },
                                disabled: is_default,
                                testid: Some("disable-group".into()),
                            });
                        } else {
                            acts.push(ActionSpec {
                                label: "启用".to_string(),
                                tone: ActionTone::Success,
                                disabled: false,
                                testid: Some("enable-group".into()),
                            });
                        }
                        // 删除位: 默认组为禁用占位「内置」
                        if is_default {
                            acts.push(ActionSpec {
                                label: "内置".to_string(),
                                tone: ActionTone::Disabled,
                                disabled: true,
                                testid: None,
                            });
                        } else {
                            acts.push(ActionSpec {
                                label: "删除".to_string(),
                                tone: ActionTone::Danger,
                                disabled: false,
                                testid: Some("delete-group".into()),
                            });
                        }
                        acts
                    }
                }
            }
        }
    }
}

// ============ 弹窗 ============

#[component]
pub(crate) fn Modal(title: String, on_close: EventHandler<()>, children: Element) -> Element {
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_close.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "{title}" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        svg {
                            class: "h-5 w-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
                        }
                    }
                }
                {children}
            }
        }
    }
}

const MODAL_INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none";

#[component]
pub fn GroupFormModal(
    editing: bool,
    group_key: Option<String>,
    name: Signal<String>,
    ratio: Signal<String>,
    remark: Signal<String>,
    whitelist: Signal<String>,
    // 映射别名候选 (后端 models 域列表的 name); 空 = 拉取失败/无候选, 只读回退。
    alias_options: Vec<String>,
    // 映射别名草稿 (逗号分隔字符串; MVP 仅登记展示, 不随提交落库, 文案见 tab1)
    f_alias: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑分组"
    } else {
        "新建分组"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建分组"
    };

    // 编辑弹窗双 tab: 0=分组信息(名称/倍率/备注/白名单), 1=映射别名
    let mut active_tab = use_signal(|| 0usize);
    let tab_labels = vec!["分组信息".to_string(), "映射别名".to_string()];

    let parsed_ratio = ratio().trim().parse::<f64>().unwrap_or(1.0).max(0.0);

    let submitting = use_signal(|| false);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let group_key2 = group_key.clone();
    let whitelist2 = whitelist;
    let do_submit = move |_| {
        let key = group_key2.clone();
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let r = ratio.peek().trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        let rm = remark.peek().clone();
        let wl = parse_whitelist_raw(&whitelist2.peek());
        let (mut sub, cb) = (submitting2, on_submit2);
        spawn(async move {
            sub.set(true);
            let client = ApiClient::shared().clone();
            let req = GroupUpsertRequest {
                name: n,
                ratio: r,
                model_whitelist: json!(wl),
                remark: rm,
            };
            let res = match key {
                Some(kk) => update_group_api(&client, &kk, &req).await,
                None => create_group_api(&client, &req).await,
            };
            let _ = res;
            sub.set(false);
            cb.call(());
        });
    };

    let preset_ratios = [
        ("0.5× 半价", "0.5"),
        ("0.8× 优惠", "0.8"),
        ("1.0× 基准", "1.0"),
        ("1.2× 溢价", "1.2"),
        ("1.5× 高配", "1.5"),
        ("2.0× 双倍", "2.0"),
    ];

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            // tab 栏
            div { class: "mb-4",
                SegmentedCapsule {
                    items: tab_labels,
                    active: active_tab(),
                    on_select: move |i| active_tab.set(i),
                    testid_prefix: "group-tab".to_string(),
                }
            }

            div { class: "space-y-4",
                // tab 0: 分组信息
                if active_tab() == 0 {
                    div { class: "space-y-4",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "分组标识 (英文唯一标识)" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-name",
                                placeholder: "例如: vip, claude, fast",
                                value: "{name}",
                                disabled: editing && name() == "default",
                                oninput: move |e| name.set(e.value()),
                            }
                            if editing && name() == "default" {
                                p { class: "mt-1 text-xs text-zinc-500", "默认分组标识不可更改" }
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "展示备注 (可选)" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-remark",
                                placeholder: "例如: VIP会员专线、高峰备用组",
                                value: "{remark}",
                                oninput: move |e| remark.set(e.value()),
                            }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "模型白名单 (逗号分隔,可选)" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-whitelist",
                                placeholder: "例如: gpt-4o, claude-3.5",
                                value: "{whitelist}",
                                oninput: move |e| whitelist.set(e.value()),
                            }
                            p { class: "mt-1 text-[11px] text-zinc-500", "留空 = 全模型可用;填了 = 仅这些模型" }
                        }

                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (ratio ≥ 0)" }
                            input {
                                class: "{MODAL_INPUT} font-mono",
                                r#type: "text",
                                "data-testid": "group-ratio",
                                placeholder: "1.0",
                                value: "{ratio}",
                                oninput: move |e| ratio.set(e.value()),
                            }
                        }

                        // 快捷预设按钮
                        div { class: "space-y-1.5",
                            p { class: "text-[11px] text-zinc-500", "快捷倍率预设" }
                            div { class: "flex flex-wrap gap-1.5",
                                for (lbl, val) in preset_ratios {
                                    {
                                        let is_active = (parsed_ratio - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                        let btn_tone = if is_active {
                                            "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                        } else {
                                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                        };
                                        rsx! {
                                            button {
                                                class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                                onclick: move |_| ratio.set(val.to_string()),
                                                "{lbl}"
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // 计费预览: 仅保留「该分组实际扣费」(「标准消耗」行已删)
                        div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                            div { class: "flex justify-between font-medium",
                                span { class: "text-zinc-300", "该分组实际扣费" }
                                span { class: if parsed_ratio < 1.0 { "text-emerald-400" } else if parsed_ratio > 1.0 { "text-amber-400" } else { "text-zinc-200" },
                                    "{(100.0 * parsed_ratio).round() as i64} 点额度"
                                }
                            }
                        }
                    }
                }

                // tab 1: 映射别名 (从后端 models 域候选挑选; 失败留空 → 只读回退)
                if active_tab() == 1 {
                    div { class: "space-y-3",
                        div {
                            label { class: "mb-1.5 block text-xs text-zinc-400", "映射别名 (多选,逗号分隔)" }
                            input {
                                class: MODAL_INPUT,
                                "data-testid": "group-alias",
                                placeholder: "例如: gpt-4o, claude-3.5",
                                value: "{f_alias}",
                                oninput: move |e| f_alias.set(e.value()),
                            }
                            p { class: "mt-1 text-[11px] text-zinc-500", "本 MVP 仅登记, 后端暂无映射列; 留空 = 不映射" }
                        }
                        if alias_options.is_empty() {
                            p { class: "text-xs text-zinc-500", "暂无可选模型别名" }
                        } else {
                            div { class: "flex flex-wrap gap-1.5",
                                for opt in alias_options.clone() {
                                    {
                                        let picked = parse_whitelist_raw(&f_alias.peek()).iter().any(|a| a == &opt);
                                        let cls = if picked {
                                            "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                        } else {
                                            "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                        };
                                        rsx! {
                                            button {
                                                class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {cls}",
                                                "data-testid": "group-alias-opt",
                                                onclick: move |_| {
                                                    let mut cur = parse_whitelist_raw(&f_alias.peek());
                                                    if let Some(pos) = cur.iter().position(|a| a == &opt) {
                                                        cur.remove(pos);
                                                    } else {
                                                        cur.push(opt.clone());
                                                    }
                                                    f_alias.set(cur.join(", "));
                                                },
                                                "{opt}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    "data-testid": "group-cancel",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    "data-testid": "group-submit",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}
