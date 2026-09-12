//! 模型别名管理页:卡片式网格,对齐 GroupsPage 规范。
//!
//! 数据接线(对齐 GroupsPage 模式,本地 signal 不触碰 EntityStore):
//! - 列表:挂载/reload 时 `list_model_aliases_api` 拉 GET /api/models?size=100,
//!   只映射 ModelView 的 key(写路径定位 UUID)与 name;价格/倍率字段后端无
//!   对应列,展示为 0/1.0(见页面说明条)。
//! - 编辑:`update_model_alias_api` PUT /api/models/{key},请求体只携带 name
//!   (后端 models 域唯一与别名对应的列)。
//! - 删除:`delete_model_alias_api` DELETE /api/models/{key}。
//! - 新建:后端 POST /api/models 的 CreateModelRequest 必填 owner 与 api_key,
//!   表单没有这两个字段的来源 — 提交时诚实提示,不造数据、不假成功。

use client::ApiClient;
use contract::api::billing::AliasUpsertRequest;
use dioxus::prelude::*;
use ui::SegmentedCapsule;

use crate::api::{delete_model_alias_api, list_model_aliases_api, update_model_alias_api};
use crate::groups::{Badge, Modal, StatCard};
use crate::state::AliasRow;

/// 别名列表项:后端 ModelView 的 key(UUID) + 页面展示行。
/// key 不并入 AliasRow — AliasRow 被 entities.rs 结构体字面量构造,
/// 本页独立持有 key 以定位 PUT/DELETE 路径。
#[derive(Clone, PartialEq)]
struct AliasItem {
    key: String,
    row: AliasRow,
}

/// 弹窗状态(Edit 携带后端模型 UUID key)
#[derive(Clone, PartialEq)]
enum AliasModalState {
    Closed,
    New,
    Edit(String),
}

/// 别名管理页
#[component]
pub fn AliasesPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut rows = use_signal(Vec::<AliasItem>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| AliasModalState::Closed);

    let mut f_name = use_signal(String::new);
    let mut f_display = use_signal(String::new);
    let mut f_input = use_signal(|| "0.0175".to_string());
    let mut f_output = use_signal(|| "0.07".to_string());
    let mut f_mult = use_signal(|| "1.0".to_string());

    // 挂载即拉取真实列表;reload 变化时重拉(对齐 GroupsPage)
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_model_aliases_api(&client).await {
                Ok(list) => {
                    let mut items: Vec<AliasItem> = list
                        .into_iter()
                        .map(|m| AliasItem {
                            key: m.key,
                            row: AliasRow {
                                alias: m.name,
                                display: String::new(),
                                // 后端 models 域无价格/倍率列:保持展示 0/1.0
                                input_per_1k: 0.0,
                                output_per_1k: 0.0,
                                multiplier: 1.0,
                            },
                        })
                        .collect();
                    // 与 EntityStore::hydrate 的 /api/models 映射保持一致:按别名排序
                    items.sort_by(|a, b| a.row.alias.cmp(&b.row.alias));
                    rows.set(items);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let alias_list = rows
        .read()
        .iter()
        .map(|it| it.row.clone())
        .collect::<Vec<_>>();
    let total = alias_list.len();
    let free_count = alias_list.iter().filter(|a| a.multiplier == 0.0).count();
    let standard_count = alias_list
        .iter()
        .filter(|a| (a.multiplier - 1.0).abs() < 0.001)
        .count();
    let custom_count = alias_list
        .iter()
        .filter(|a| a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001)
        .count();
    let avg_mult = if total > 0 {
        alias_list.iter().map(|a| a.multiplier).sum::<f64>() / (total as f64)
    } else {
        1.0
    };

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总别名数"),
        (standard_count.to_string(), "标准 1.0× 别名"),
        (custom_count.to_string(), "自定倍率别名"),
        (free_count.to_string(), "免费别名 (0×)"),
        (format!("{:.2}×", avg_mult), "平均加价倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("标准 1.0× ({standard_count})"),
        format!("自定倍率 ({custom_count})"),
        format!("免费通道 ({free_count})"),
    ];

    let filtered: Vec<(usize, AliasItem)> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        rows()
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                let a = &it.row;
                if !q.is_empty()
                    && !a.alias.to_lowercase().contains(&q)
                    && !a.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => (a.multiplier - 1.0).abs() < 0.001,
                    2 => a.multiplier != 0.0 && (a.multiplier - 1.0).abs() >= 0.001,
                    3 => a.multiplier == 0.0,
                    _ => true,
                }
            })
            .map(|(i, it)| (i, it.clone()))
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_display.set(String::new());
        f_input.set("0.0175".to_string());
        f_output.set("0.07".to_string());
        f_mult.set("1.0".to_string());
        modal_state.set(AliasModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(it) = rows().iter().find(|it| it.key == key) {
            f_name.set(it.row.alias.clone());
            f_display.set(it.row.display.clone());
            f_input.set(format!("{}", it.row.input_per_1k));
            f_output.set(format!("{}", it.row.output_per_1k));
            f_mult.set(format!("{}", it.row.multiplier));
            modal_state.set(AliasModalState::Edit(key));
        }
    };

    // 删除:走真实 DELETE,成功后 notice + 重拉列表(对齐 GroupsPage 写路径)
    let write_delete = move |key: String| {
        let (mut b, mut n, mut r) = (busy, notice, reload);
        spawn(async move {
            b.set(true);
            n.set(None);
            let client = ApiClient::shared().clone();
            match delete_model_alias_api(&client, &key).await {
                Ok(_) => {
                    n.set(Some("操作成功".to_string()));
                    r.set(r() + 1);
                }
                Err(e) => n.set(Some(format!("操作失败:{e}"))),
            }
            b.set(false);
        });
    };

    // 弹窗关闭并触发重拉(保存后列表以服务端为准)
    let close_and_reload = move |_| {
        modal_state.set(AliasModalState::Closed);
        reload.set(reload() + 1);
    };

    rsx! {
            div { class: "flex flex-col gap-6",
                // 通知条(成功/错误/进行中,对齐 GroupsPage)
                if let Some(msg) = notice() {
                    div {
                        role: "status",
                        class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                        "{msg}"
                        if busy() { " ···" }
                    }
                }

                // 数据与写路径说明(后端 models 端点暂无计费字段)
                div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400",
                    span { "别名来自真实 /api/models;编辑与删除已接后端;新建暂未开放(后端需要 owner/api_key 字段);价格字段后端暂未提供,显示为 0" }
                }
                // 1. 统计区
                section { id: "aliases-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "别名概览" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "aliases-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "筛选与操作" }
                            span { class: "text-xs text-zinc-500", "按倍率与资费规则快速筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                "data-testid": "refresh-aliases",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                            button {
                                class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                onclick: open_new,
                                "✚ 新建别名"
                            }
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        placeholder: "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...",
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
                section { id: "aliases-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "别名列表" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            if loading() { "加载中…" } else { "{filtered.len()} 个" }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                            p { class: "text-sm text-red-300", "加载别名失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载模型别名…" }
                        }
                    } else if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的模型别名" }
                        }
                    } else {
                        div {
                            class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            role: "list",
                            "aria-label": "别名列表",
                            "data-testid": "aliases-list",
                            for (idx, it) in filtered {
                                {
                                    rsx! {
                                        AliasCard {
                                            key: "{it.key}",
                                            alias_key: it.key,
                                            alias: it.row,
                                            index: idx,
                                            on_edit: move |k| open_edit(k),
                                            on_delete: move |k| write_delete(k),
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if matches!(modal_state(), AliasModalState::New | AliasModalState::Edit(_)) {
                AliasFormModal {
                    editing: matches!(modal_state(), AliasModalState::Edit(_)),
                    alias_key: match modal_state() {
                        AliasModalState::Edit(k) => Some(k),
                        _ => None,
                    },
                    alias: f_name,
                    display: f_display,
                    input_rate: f_input,
                    output_rate: f_output,
                    multiplier: f_mult,
                    notice,
                    on_cancel: move |_| modal_state.set(AliasModalState::Closed),
                    on_submit: close_and_reload,
                }
            }
    }
}

/// 别名卡片
#[component]
fn AliasCard(
    alias_key: String,
    alias: AliasRow,
    index: usize,
    on_edit: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    // 回调各持一份克隆,避免单一 String 被两个闭包争用所有权
    let edit_key = alias_key.clone();
    let delete_key = alias_key;
    let initial = alias
        .alias
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let m = alias.multiplier;

    let (mult_badge_text, mult_badge_tone, bar_tone, bar_width_pct) = if m == 0.0 {
        (
            "免费 0×".to_string(),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            15,
        )
    } else if (m - 1.0).abs() < 0.001 {
        (
            "标准 1.00×".to_string(),
            "border-zinc-700 bg-zinc-800/80 text-zinc-300",
            "bg-zinc-200",
            50,
        )
    } else if m < 1.0 {
        (
            format!("优惠 {m:.2}×"),
            "border-emerald-500/30 bg-emerald-500/20 text-emerald-400",
            "bg-emerald-500",
            ((m / 2.0) * 100.0).clamp(15.0, 100.0) as u32,
        )
    } else {
        (
            format!("溢价 {m:.2}×"),
            "border-amber-500/30 bg-amber-500/20 text-amber-400",
            "bg-amber-500",
            ((m / 2.0) * 100.0).clamp(15.0, 100.0) as u32,
        )
    };

    let display_title = if alias.display.is_empty() {
        alias.alias.clone()
    } else {
        alias.display.clone()
    };

    rsx! {
        div { class: "group flex flex-col justify-between rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "space-y-3",
                div { class: "flex items-start gap-3",
                    div { class: "flex h-9 w-9 shrink-0 items-center justify-center rounded-full border border-zinc-700 bg-zinc-800 text-sm font-semibold text-zinc-200 group-hover:border-zinc-500 transition-colors",
                        "{initial}"
                    }
                    div { class: "min-w-0 flex-1",
                        div { class: "flex items-center justify-between gap-2",
                            h3 { class: "truncate text-sm font-medium text-zinc-100", "{alias.alias}" }
                            span { class: "shrink-0 rounded bg-zinc-800 px-1.5 py-0.5 text-[10px] font-mono text-zinc-400 border border-zinc-700/60",
                                "#{index + 1}"
                            }
                        }
                        p { class: "mt-0.5 truncate text-[11px] text-zinc-400", "{display_title}" }
                    }
                }

                div { class: "flex flex-wrap gap-1.5",
                    Badge { text: mult_badge_text, tone: mult_badge_tone }
                    Badge { text: format!("入: ¥{}/1k", alias.input_per_1k), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                    Badge { text: format!("出: ¥{}/1k", alias.output_per_1k), tone: "border-zinc-700 bg-zinc-800/80 text-zinc-300" }
                }

                div { class: "space-y-1.5",
                    div { class: "flex justify-between gap-2 text-[11px]",
                        span { class: "text-zinc-400", "计费倍率" }
                        span { class: "whitespace-nowrap font-medium text-zinc-200", "×{m:.2}" }
                    }
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone} transition-all duration-300", style: "width: {bar_width_pct}%" }
                    }
                }

                div { class: "space-y-1.5 text-xs pt-1",
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "输入单价" }
                        span { class: "font-mono font-medium text-zinc-200", "¥ {alias.input_per_1k} / 1k" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "输出单价" }
                        span { class: "font-mono font-medium text-zinc-200", "¥ {alias.output_per_1k} / 1k" }
                    }
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 text-zinc-400", "1M tokens 测算" }
                        span { class: "font-mono font-medium text-zinc-200",
                            "¥ {((alias.input_per_1k + alias.output_per_1k * 2.0) * 1000.0 * m).round() / 1000.0}"
                        }
                    }
                }
            }

            div { class: "mt-4 flex gap-1.5 border-t border-zinc-800 pt-3",
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700 hover:text-white",
                    "data-testid": "edit-alias",
                    onclick: move |_| on_edit.call(edit_key.clone()),
                    "编辑"
                }
                button {
                    class: "flex-1 rounded-lg border border-zinc-700/80 bg-zinc-800/60 py-1.5 text-xs font-medium text-red-400 transition-colors hover:bg-zinc-700 hover:text-red-300",
                    "data-testid": "delete-alias",
                    onclick: move |_| on_delete.call(delete_key.clone()),
                    "删除"
                }
            }
        }
    }
}

/// 别名新建/编辑弹窗
#[component]
fn AliasFormModal(
    editing: bool,
    alias_key: Option<String>,
    alias: Signal<String>,
    display: Signal<String>,
    input_rate: Signal<String>,
    output_rate: Signal<String>,
    multiplier: Signal<String>,
    notice: Signal<Option<String>>,
    on_cancel: EventHandler<()>,
    on_submit: EventHandler<()>,
) -> Element {
    let title = if editing {
        "编辑模型别名"
    } else {
        "新建模型别名"
    };
    let submit_label = if editing {
        "保存修改"
    } else {
        "创建别名"
    };

    let submitting = use_signal(|| false);

    // 工厂式复制,避免把原 signal 移动出闭包(供 rsx 中 submitting() 继续读取)
    let submitting2 = submitting;
    let on_submit2 = on_submit;
    let notice2 = notice;
    let key2 = alias_key.clone();
    let do_submit = move |_| {
        let key = key2.clone();
        let name = alias.peek().trim().to_string();
        if name.is_empty() {
            return;
        }
        let (mut sub, cb, mut note) = (submitting2, on_submit2, notice2);
        match key {
            // 编辑:PUT /api/models/{key}。请求体只带 name — 后端 models 域
            // 与别名页对应的列只有 name,display/价格/倍率无对应列,置空后
            // 由后端 UpdateModelRequest(全 Option)忽略,不写库。
            Some(k) => {
                spawn(async move {
                    sub.set(true);
                    let client = ApiClient::shared().clone();
                    let req = AliasUpsertRequest {
                        name,
                        ..Default::default()
                    };
                    if let Err(e) = update_model_alias_api(&client, &k, &req).await {
                        note.set(Some(format!("保存失败:{e}")));
                    }
                    sub.set(false);
                    cb.call(()); // 关闭弹窗并重拉列表(以服务端为准)
                });
            }
            // 新建:后端 CreateModelRequest 必填 owner 与 api_key,表单没有
            // 这两个字段的来源 — 诚实拒绝,不造数据、不假成功。
            None => {
                note.set(Some(
                    "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供".to_string(),
                ));
                cb.call(());
            }
        }
    };

    rsx! {
        Modal { title: title.to_string(), on_close: move |_| on_cancel.call(()),
            div { class: "space-y-4",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "别名标识 (API 请求匹配名)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: gpt-4o, claude-3-5-sonnet",
                        value: "{alias}",
                        oninput: move |e| alias.set(e.value()),
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "展示名称 (可选)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 focus:border-zinc-500 focus:outline-none",
                        placeholder: "例如: GPT-4o 旗舰模型",
                        value: "{display}",
                        oninput: move |e| display.set(e.value()),
                    }
                }

                div { class: "grid grid-cols-2 gap-3",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "输入单价 ¥/1k" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "0.0175",
                            value: "{input_rate}",
                            oninput: move |e| input_rate.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "输出单价 ¥/1k" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                            placeholder: "0.07",
                            value: "{output_rate}",
                            oninput: move |e| output_rate.set(e.value()),
                        }
                    }
                }

                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "计费倍率 (multiplier ≥ 0)" }
                    input {
                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 font-mono focus:border-zinc-500 focus:outline-none",
                        placeholder: "1.0",
                        value: "{multiplier}",
                        oninput: move |e| multiplier.set(e.value()),
                    }
                }
            }

            div { class: "mt-6 flex gap-3",
                button {
                    class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                    onclick: move |_| on_cancel.call(()),
                    "取消"
                }
                button {
                    class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                    disabled: submitting(),
                    onclick: do_submit,
                    "{submit_label}"
                }
            }
        }
    }
}
