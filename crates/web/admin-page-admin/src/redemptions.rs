//! 兑换码管理页:卡片式网格,对齐 GroupsPage / ChannelsPage / UsersPanel 规范。
//! 数据来自真实后端 `/api/redemption`(列表 / 批量生成 / 停用)。
//! 后端语义:DELETE 即停用 (status→2,无硬删、无重新启用);
//! 明文码只在生成响应里出现一次,页面用弹窗展示。

use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::api::{
    RedemptionView, disable_redemption_api, generate_redemptions_api, list_redemptions_api,
};
use crate::groups::{Badge, Modal, StatCard};
use client::ApiClient;

const SEC_STATS: &str = "兑换码概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "兑换码列表";

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum RedModalState {
    Closed,
    Generate,
    /// 生成成功后的一次性明文码展示
    Codes(Vec<String>),
}

/// 页面内兑换码视图模型 (金额已换算为 ¥)。
#[derive(Clone, PartialEq)]
struct RedRowFE {
    key: String,
    code_preview: String,
    quota_cny: f64,
    status: u8, // 1 未用 / 2 停用 / 3 已核销
    redeemed_by: Option<String>,
    redeemed_at: String,
    created: String,
}

#[component]
pub fn RedemptionsPage() -> Element {
    let mut reds = use_signal(Vec::<RedRowFE>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| RedModalState::Closed);
    let mut copied_key = use_signal(|| None::<String>);

    // 生成弹窗字段(后端只支持 面额+数量;活动名/有效期无对应字段)
    let mut f_count = use_signal(|| "1".to_string());
    let mut f_quota = use_signal(|| "50".to_string());

    // 拉取(挂载/刷新/写回后)
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_redemptions_api(&client, None, Some(1), Some(100)).await {
                Ok((items, _total)) => {
                    reds.set(
                        items
                            .into_iter()
                            .map(|v: RedemptionView| RedRowFE {
                                key: v.key,
                                code_preview: v.code_preview,
                                quota_cny: v.quota as f64 / 500_000.0,
                                status: v.status as u8,
                                redeemed_by: v.redeemed_by,
                                redeemed_at: v.redeemed_at.unwrap_or_default(),
                                created: v.created_at,
                            })
                            .collect(),
                    );
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let red_list = reds.read().clone();
    let total = red_list.len();
    let unused_count = red_list.iter().filter(|r| r.status == 1).count();
    let used_count = red_list.iter().filter(|r| r.status == 3).count();
    let disabled_count = red_list.iter().filter(|r| r.status == 2).count();

    let total_quota: f64 = red_list.iter().map(|r| r.quota_cny).sum();
    let available_quota: f64 = red_list
        .iter()
        .filter(|r| r.status == 1)
        .map(|r| r.quota_cny)
        .sum();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总兑换码数"),
        (unused_count.to_string(), "未使用可兑"),
        (used_count.to_string(), "已兑换核销"),
        (format!("¥{total_quota:.1}"), "发行总面值"),
        (format!("¥{available_quota:.1}"), "待兑现金额"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("未使用 ({unused_count})"),
        format!("已兑换 ({used_count})"),
        format!("已停用 ({disabled_count})"),
    ];

    let filtered_rows: Vec<RedRowFE> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        red_list
            .into_iter()
            .filter(|r| {
                if !q.is_empty()
                    && !r.code_preview.to_lowercase().contains(&q)
                    && !r.key.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => r.status == 1,
                    2 => r.status == 3,
                    3 => r.status == 2,
                    _ => true,
                }
            })
            .collect()
    };

    let err_for_effect = err;
    let reload_for_effect = reload;

    let disable_red = move |key: String| {
        let mut err = err_for_effect;
        let mut reload = reload_for_effect;
        spawn(async move {
            let client = ApiClient::shared().clone();
            match disable_redemption_api(&client, &key).await {
                Ok(_) => reload.set(reload() + 1),
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
        });
    };

    let commit_generate = move |_| {
        let cnt = f_count
            .peek()
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
            .clamp(1, 100);
        // 后端 quota 是内部计费单位(500000 = ¥1);表单输入的是 ¥
        let q_cny = f_quota
            .peek()
            .trim()
            .parse::<f64>()
            .unwrap_or(10.0)
            .max(0.0);
        let quota_units = (q_cny * 500_000.0) as i64;
        if quota_units <= 0 {
            return;
        }

        let mut modal_state = modal_state;
        let mut err = err;
        let mut reload = reload;
        spawn(async move {
            let client = ApiClient::shared().clone();
            match generate_redemptions_api(&client, quota_units, cnt).await {
                Ok(codes) => {
                    reload.set(reload() + 1);
                    modal_state.set(RedModalState::Codes(codes));
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                }
            }
        });
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 1. 概览统计区
                section { id: "reds-sec-stats", "data-testid": "redemptions-panel", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "reds-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按状态分级筛选;停用后不可重新启用" }
                        }
                        button {
                            class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                            onclick: move |_| {
                                f_count.set("1".to_string());
                                f_quota.set("50".to_string());
                                modal_state.set(RedModalState::Generate);
                            },
                            "✚ 生成兑换码"
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索兑换码预览 (如 fx-086c****) ...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }
                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_tier(),
                            on_select: move |i: usize| filter_tier.set(i),
                        }
                    }
                }

                // 3. 卡片网格区
                section { id: "reds-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        if loading() {
                            span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400", "加载中…" }
                        } else {
                            span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                                "{filtered_rows.len()} 张"
                            }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                            p { class: "text-sm text-red-300", "加载兑换码失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载兑换码…" }
                        }
                    } else if filtered_rows.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的兑换码" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for r in filtered_rows {
                                {
                                    let is_just_copied = copied_key() == Some(r.key.clone());
                                    rsx! {
                                        RedemptionCard {
                                            key: "{r.key}",
                                            item: r,
                                            is_just_copied: is_just_copied,
                                            on_copy: move |k: String| {
                                                copied_key.set(Some(k));
                                            },
                                            on_disable: disable_red,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 生成弹窗 / 明文码展示
            match modal_state() {
                RedModalState::Generate => rsx! {
                    RedemptionGenerateModal {
                        count: f_count,
                        quota: f_quota,
                        on_cancel: move |_| modal_state.set(RedModalState::Closed),
                        on_submit: commit_generate,
                    }
                },
                RedModalState::Codes(codes) => rsx! {
                    GeneratedCodesModal { codes,
                        on_close: move |_| modal_state.set(RedModalState::Closed),
                    }
                },
                RedModalState::Closed => rsx! {},
            }
    }
}

/// 兑换码卡片
#[component]
fn RedemptionCard(
    item: RedRowFE,
    is_just_copied: bool,
    on_copy: EventHandler<String>,
    on_disable: EventHandler<String>,
) -> Element {
    let preview = item.code_preview.clone();
    let _ = preview;

    let (status_text, status_tone, bar_tone, bar_pct) = match item.status {
        1 => (
            "未使用",
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            100,
        ),
        2 => (
            "已停用",
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            40,
        ),
        _ => (
            "已核销",
            "border-zinc-700 bg-zinc-800/80 text-zinc-400",
            "bg-zinc-700",
            0,
        ),
    };

    let key_clone = item.key.clone();
    let disable_key = item.key.clone();

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "space-y-3",
                // 头部
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "¥"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate font-mono text-sm font-medium text-zinc-100", "{item.code_preview}" }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400 font-mono", "{item.key}" }
                    }
                }

                // 徽标行
                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: status_text.to_string(), tone: status_tone }
                    Badge { text: format!("面值 ¥{:.2}", item.quota_cny), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-200 font-mono" }
                }

                // 额度有效条
                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "可用面额" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200 font-mono", "¥ {item.quota_cny:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_pct}%" }
                    }
                }

                // 详情指标行
                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "生成时间" }
                        span { class: "font-mono text-zinc-400", "{item.created}" }
                    }
                    if let Some(by) = &item.redeemed_by {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "兑换人" }
                            span { class: "font-medium text-zinc-200", "{by}" }
                        }
                    }
                    if !item.redeemed_at.is_empty() {
                        div { class: "flex justify-between gap-2",
                            span { class: "shrink-0 text-zinc-400", "核销时间" }
                            span { class: "font-mono text-zinc-300", "{item.redeemed_at}" }
                        }
                    }
                }
            }

            // 底部操作区: [复制预览] [停用] — 后端仅支持停用(无硬删/无重新启用)
            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: if is_just_copied {
                        "flex-1 rounded-lg border border-emerald-500/80 bg-emerald-950/60 py-1.5 text-xs text-emerald-300 transition-colors font-medium"
                    } else {
                        "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white"
                    },
                    onclick: move |_| on_copy.call(key_clone.clone()),
                    if is_just_copied { "已复制" } else { "复制预览" }
                }
                if item.status == 1 {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-amber-400 transition-colors hover:bg-zinc-700 hover:text-amber-300",
                        onclick: move |_| on_disable.call(disable_key.clone()),
                        "停用"
                    }
                } else if item.status == 2 {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "已停用"
                    }
                } else {
                    button {
                        class: "flex-1 rounded-lg border border-zinc-800 bg-zinc-900 py-1.5 text-xs text-zinc-600 cursor-not-allowed",
                        disabled: true,
                        "已核销"
                    }
                }
            }
        }
    }
}

/// 批量生成兑换码弹窗(后端仅支持 面额 + 数量;无活动名/有效期字段)
#[component]
fn RedemptionGenerateModal(
    count: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let parsed_count = count().trim().parse::<u32>().unwrap_or(1).clamp(1, 100);
    let parsed_quota = quota().trim().parse::<f64>().unwrap_or(10.0).max(0.0);
    let total_value = (parsed_count as f64) * parsed_quota;

    let preset_quotas = [
        ("¥10", "10"),
        ("¥20", "20"),
        ("¥50", "50"),
        ("¥100", "100"),
        ("¥200", "200"),
    ];

    rsx! {
        Modal { title: "生成批量兑换码".to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4 max-h-[70vh] overflow-y-auto pr-1",
                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "生成数量 (1-100)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            r#type: "number",
                            min: "1",
                            max: "100",
                            value: "{count}",
                            oninput: move |e| count.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "单张面额 (元)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "50",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }
                }

                // 快捷面额按钮
                div { class: "space-y-1.5",
                    p { class: "text-[11px] text-zinc-500", "快捷面额预设" }
                    div { class: "flex flex-wrap gap-1.5",
                        for (lbl, val) in preset_quotas {
                            {
                                let is_active = (parsed_quota - val.parse::<f64>().unwrap_or(0.0)).abs() < 0.001;
                                let btn_tone = if is_active {
                                    "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                } else {
                                    "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                };
                                rsx! {
                                    button {
                                        class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {btn_tone}",
                                        onclick: move |_| quota.set(val.to_string()),
                                        "{lbl}"
                                    }
                                }
                            }
                        }
                    }
                }

                // 测算卡片
                div { class: "rounded-xl border border-zinc-800 bg-zinc-950 px-4 py-3 text-xs space-y-1.5",
                    div { class: "flex justify-between text-zinc-400",
                        span { "生成总批次" }
                        span { "{parsed_count} 张卡密" }
                    }
                    div { class: "flex justify-between font-medium",
                        span { class: "text-zinc-300", "发行总面值金额" }
                        span { class: "text-emerald-400 font-mono text-sm",
                            "¥ {total_value:.2}"
                        }
                    }
                }

                p { class: "text-[11px] text-zinc-600",
                    "提示: 明文卡密只在生成后显示一次,请及时复制保存;后端暂不支持活动名与有效期字段。"
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_submit.call(()),
                    "立即批量生成"
                }
            }
        }
    }
}

/// 生成成功后的一次性明文码展示(关闭后不再可见)
#[component]
fn GeneratedCodesModal(codes: Vec<String>, on_close: EventHandler<()>) -> Element {
    let joined = codes.join("\n");
    rsx! {
        Modal { title: format!("生成成功 · {} 张明文卡密(仅此一次)", codes.len()), on_close: move |_| on_close.call(()),
            div { class: "space-y-3",
                p { class: "text-xs text-amber-400",
                    "以下明文卡密关闭本窗口后无法再次查看,请立即复制保存。"
                }
                pre { class: "max-h-72 overflow-y-auto rounded-xl border border-zinc-700 bg-zinc-950 p-3 font-mono text-xs text-zinc-200 select-all",
                    "{joined}"
                }
            }
            div { class: "mt-6 flex",
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200",
                    onclick: move |_| on_close.call(()),
                    "我已保存,关闭"
                }
            }
        }
    }
}
