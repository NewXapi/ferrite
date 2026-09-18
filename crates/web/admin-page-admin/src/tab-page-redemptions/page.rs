//! 兑换码管理页:卡片式网格,对齐 GroupsPage / ChannelsPage / UsersPanel 规范。
//! 数据来自真实后端 `/api/redemption`(列表 / 批量生成 / 停用)。
//! 后端语义:核销 (status→2)、DELETE 即停用 (status→3);无硬删、无重新启用;
//! 明文码只在生成响应里出现一次,页面用弹窗展示。
//!
//! 本文件只保留状态与写回逻辑(拉取 / `commit_generate` / `disable_red`);
//! 渲染拆成 `card`(卡片)与 `modal`
//! (生成弹窗 / 明文码展示),共享类型与映射见 `shared`。

use client::ApiClient;
use dioxus::prelude::*;
use ui::{RedemptionCard as PrototypeRedemptionCard, SegmentedCapsule};

use super::card::RedemptionCard;
use super::modal::{GeneratedCodesModal, RedemptionGenerateModal};
use super::shared::{
    RedModalState, RedRowFE, SEC_FILTER, SEC_LIST, SEC_STATS, map_redemption_view,
};
use crate::api::{disable_redemption_api, generate_redemptions_api, list_redemptions_api};
use crate::tab_page_groups::StatCard;

/// 兑换码管理页
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
                    reds.set(items.into_iter().map(map_redemption_view).collect());
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
    // 2=已核销 / 3=已停用(对齐后端 admin-billing/redeem.rs 写库口径)
    let used_count = red_list.iter().filter(|r| r.status == 2).count();
    let disabled_count = red_list.iter().filter(|r| r.status == 3).count();

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
                // tier 与 filter_options 一一对应:1=未使用 / 2=已核销 / 3=已停用
                match tier {
                    1 => r.status == 1,
                    2 => r.status == 2,
                    3 => r.status == 3,
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
                section {
                    id: "reds-sec-stats",
                    "data-testid": "redemptions-stats",
                    role: "region",
                    "aria-label": "兑换码概览",
                    class: "scroll-mt-8 space-y-3",
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
                    "data-testid": "redemptions-filter",
                    role: "search",
                    "aria-label": "兑换码筛选与操作",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按状态分级筛选;停用后不可重新启用" }
                        }
                        button {
                            "data-testid": "generate-redemptions",
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
                        "data-testid": "redemptions-search",
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
                section {
                    id: "reds-sec-list",
                    "data-testid": "redemptions-list",
                    role: "list",
                    "aria-label": "兑换码列表",
                    class: "scroll-mt-8 space-y-4",
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
                        div {
                            "data-testid": "redemptions-error",
                            role: "alert",
                            "aria-label": "兑换码加载失败",
                            class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                            p { class: "text-sm text-red-300", "加载兑换码失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                "data-testid": "retry-redemptions",
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
                        div {
                            "data-testid": "redemptions-empty",
                            class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的兑换码" }
                        }
                    } else {
                        if let Some(row) = filtered_rows.first().cloned() {
                            {
                                let redeemed_at = (!row.redeemed_at.is_empty()).then_some(row.redeemed_at.clone());
                                rsx! {
                                    div {
                                        class: "mb-4 grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                                        role: "region",
                                        "aria-label": "新卡示例",
                                        "data-testid": "redemption-card-prototype",
                                        PrototypeRedemptionCard {
                                            redemption_key: row.key,
                                            code_preview: row.code_preview,
                                            quota_cny: row.quota_cny,
                                            status: i16::from(row.status),
                                            redeemed_by: row.redeemed_by,
                                            redeemed_at,
                                            created_at: row.created,
                                        }
                                    }
                                }
                            }
                        }
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
