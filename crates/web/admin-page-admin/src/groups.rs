//! 分组管理页:卡片式设计,对齐用户管理面板 (UsersPanel) 视觉规范。
//! 数据来自真实后端:挂载时 `use_effect` 拉 `list_groups_api`,写入本地
//! `groups` signal;删除走 `delete_group_api`,新建/编辑走
//! `create_group_api` / `update_group_api`。

use dioxus::prelude::*;
use serde_json::json;
use ui::ActionButtonGroup;
use ui::ActionSpec;
use ui::ActionTone;
use ui::SegmentedCapsule;
// 统计卡收敛到 ui-components 后改为 re-export：crate 内 channels/aliases/system/
// redemptions 仍走 `crate::groups::StatCard`，引用路径不变。
pub(crate) use ui::StatCard;

use client::ApiClient;
use contract::api::admin::{GroupDto, GroupUpsertRequest};

use crate::api::{
    create_group_api, delete_group_api, list_groups_api, set_group_status_api, update_group_api,
    update_group_ratio_api,
};

/// 弹窗状态
#[derive(Clone, PartialEq)]
enum ModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类(目前仅删除;工厂保留扩展位)
#[derive(Clone, Copy)]
enum WriteOp {
    Delete,
    /// 启用/停用切换 (status: 1=启用, 2=停用)
    ToggleStatus(i16),
    /// 倍率滑条拖动写回 (ratio)
    SetRatio(f64),
}

const SEC_STATS: &str = "分组概览";
const SEC_FILTER: &str = "筛选与操作";
const SEC_LIST: &str = "分组列表";

/// 把用户输入的逗号/分号分隔白名单拆成模型名数组(去空、trim)。
/// 与后端 `validate_whitelist` 对齐:每项必须是非空字符串。
pub fn parse_whitelist_raw(raw: &str) -> Vec<String> {
    raw.split([',', '，', ';', '；'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// 从后端返回的 `model_whitelist` JSON(字符串数组或空)取回白名单。
pub fn parse_whitelist(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[component]
pub fn GroupsPage() -> Element {
    // 真实数据 + 加载/错误态(本地 signal,不触碰 EntityStore)
    let mut groups = use_signal(Vec::<GroupDto>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    // 写操作进行中 / 成功提示
    let busy = use_signal(|| false);
    let notice = use_signal(|| None::<String>);
    // reload 计数:触发一次即重拉列表(写操作后刷新)
    let mut reload = use_signal(|| 0u32);

    let mut search = use_signal(String::new);
    let mut filter_tier = use_signal(|| 0usize);
    let mut modal_state = use_signal(|| ModalState::Closed);
    // 多选集合 (卡片勾选框); 批量动作只在选中数>0 时可用
    let mut selected = use_signal(Vec::<String>::new);

    // 表单状态
    let mut f_name = use_signal(String::new);
    let mut f_ratio = use_signal(|| "1.0".to_string());
    let mut f_remark = use_signal(String::new);
    // 模型白名单:逗号分隔输入,提交时拆分;后端校验非空字符串数组
    let mut f_whitelist = use_signal(String::new);
    // 映射别名(编辑弹窗 tab2): 逗号分隔; MVP 仅登记
    let mut f_alias = use_signal(String::new);

    // 映射别名候选 (models 域 name 列表); 与分组列表并行拉取, 失败留空
    let mut alias_options = use_signal(Vec::<String>::new);
    use_effect(move || {
        spawn(async move {
            let client = ApiClient::shared().clone();
            match crate::api::list_models_api(&client).await {
                Ok(v) => alias_options.set(v.into_iter().map(|m| m.name).collect()),
                Err(_) => alias_options.set(Vec::new()),
            }
        });
    });

    // 挂载即拉取真实列表;reload 变化时重拉
    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match list_groups_api(&client).await {
                Ok(list) => {
                    groups.set(list);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = groups();
    let total = list.len();
    let enabled_count = list.iter().filter(|g| g.status == 1).count();
    let disabled_count = list.iter().filter(|g| g.status != 1).count();
    let avg_ratio = if total > 0 {
        list.iter().map(|g| g.ratio).sum::<f64>() / (total as f64)
    } else {
        1.0
    };
    let custom_count = list
        .iter()
        .filter(|g| (g.ratio - 1.0).abs() > 0.001)
        .count();

    let stats: [(String, &str); 5] = [
        (total.to_string(), "总分组数"),
        (enabled_count.to_string(), "启用中"),
        (disabled_count.to_string(), "已停用"),
        (format!("{:.2}×", avg_ratio), "平均倍率"),
        (custom_count.to_string(), "非基准倍率"),
    ];

    let filter_options = vec![
        format!("全部 ({total})"),
        format!("启用中 ({enabled_count})"),
        format!("已停用 ({disabled_count})"),
    ];

    // 过滤列表
    let filtered: Vec<GroupDto> = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        list.into_iter()
            .filter(|g| {
                if !q.is_empty()
                    && !g.name.to_lowercase().contains(&q)
                    && !g.remark.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => g.status == 1,
                    2 => g.status != 1,
                    _ => true,
                }
            })
            .collect()
    };

    let open_new = move |_| {
        f_name.set(String::new());
        f_ratio.set("1.0".to_string());
        f_remark.set(String::new());
        f_whitelist.set(String::new());
        f_alias.set(String::new());
        modal_state.set(ModalState::New);
    };

    let mut open_edit = move |key: String| {
        if let Some(g) = groups().iter().find(|g| g.key == key) {
            f_name.set(g.name.clone());
            f_ratio.set(format!("{}", g.ratio));
            f_remark.set(g.remark.clone());
            f_whitelist.set(parse_whitelist(&g.model_whitelist).join(", "));
            // MVP: 映射别名暂不落库, 打开时清空; 后端补列后从 g 回填
            f_alias.set(String::new());
            modal_state.set(ModalState::Edit(key));
        }
    };

    // 写操作助手工厂:返回独立闭包,交给卡片(删除/启停/倍率)。
    // 启用/停用与倍率是局部状态变更 — 成功后就地更新本地 groups 里对应项
    // (用 groups.update 原位改, 避免与批量路径整列表 set 互踩丢更新),
    // 不触发整页重拉(reload),避免列表闪烁; 失败时除通知外追加 reload
    // 触发整页重拉, 让 UI 与服务端真值重新同步; 删除必须整页重拉(行消失)。
    let make_write = || {
        let busy_sig = busy;
        let notice_sig = notice;
        let reload_sig = reload;
        let groups_sig = groups;
        move |key: String, op: WriteOp| {
            let (mut b, mut n, mut r, mut g) = (busy_sig, notice_sig, reload_sig, groups_sig);
            spawn(async move {
                b.set(true);
                n.set(None);
                let client = ApiClient::shared().clone();
                let res = match op {
                    WriteOp::Delete => {
                        let r = delete_group_api(&client, &key).await;
                        r.map(|_| serde_json::json!(true))
                    }
                    WriteOp::ToggleStatus(s) => set_group_status_api(&client, &key, s)
                        .await
                        .map(|_| serde_json::json!(true)),
                    WriteOp::SetRatio(v) => update_group_ratio_api(&client, &key, v)
                        .await
                        .map(|_| serde_json::json!(true)),
                };
                match res {
                    Ok(_) => {
                        n.set(Some("操作成功".to_string()));
                        match op {
                            WriteOp::Delete => {
                                // 删除:行要消失,必须整页重拉
                                r.set(r() + 1);
                            }
                            WriteOp::ToggleStatus(s) => {
                                // 启停:就地更新本地 status, 不重拉, 无闪烁;
                                // with_mut 原位改, 不整表 set, 避免与批量路径并发写互踩
                                g.with_mut(|list| {
                                    if let Some(hit) = list.iter_mut().find(|x| x.key == key) {
                                        hit.status = s;
                                    }
                                });
                            }
                            WriteOp::SetRatio(v) => {
                                // 倍率:就地更新本地 ratio, 不重拉, 卡片即时反映
                                g.with_mut(|list| {
                                    if let Some(hit) = list.iter_mut().find(|x| x.key == key) {
                                        hit.ratio = v;
                                    }
                                });
                            }
                        }
                    }
                    Err(e) => {
                        n.set(Some(format!("操作失败:{e}")));
                        // 失败也要整页重拉: 本地列表可能已与服务端漂移(如并发
                        // 写冲突/他人改动), 重拉一次与真值重新同步。删除成功
                        // 分支已有重拉, 这里只在失败路径统一追加, 不重复。
                        r.set(r() + 1);
                    }
                }
                b.set(false);
            });
        }
    };
    let write_delete = make_write();
    let write_toggle = make_write();

    // 批量动作执行闭包工厂: 参数为 target status (1=启用, 2=停用), 返回独立的
    // onclick 闭包。不复用 make_write —— 那是逐条 fire-and-forget, 无错误反馈;
    // 这里自包含地顺序 await 每个 key 的状态写回: 进入时快照选中集合(空则直接
    // return), 成功者就地更新本地 groups 对应项(与单发路径同为 with_mut 原位改),
    // 收集失败 key, 结束后一次性汇总通知, 最后无论成败清空选中。
    let make_bulk_toggle = |target: i16| {
        move |_| {
            let keys = selected.peek().clone();
            if keys.is_empty() {
                return;
            }
            let (mut g, mut n, mut sel) = (groups, notice, selected);
            spawn(async move {
                let client = ApiClient::shared().clone();
                let mut ok = 0usize;
                let mut failed: Vec<String> = Vec::new();
                for k in &keys {
                    match set_group_status_api(&client, k, target).await {
                        Ok(_) => {
                            ok += 1;
                            g.with_mut(|list| {
                                if let Some(hit) = list.iter_mut().find(|x| &x.key == k) {
                                    hit.status = target;
                                }
                            });
                        }
                        Err(_) => failed.push(k.clone()),
                    }
                }
                let msg = if failed.is_empty() {
                    "批量操作成功".to_string()
                } else {
                    // 失败 key 截断到前 3 个, 更长以 … 结尾提示
                    let shown = failed
                        .iter()
                        .take(3)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(", ");
                    let more = if failed.len() > 3 { "…" } else { "" };
                    format!(
                        "批量操作：{ok} 成功，{} 失败（key: {shown}{more}）",
                        failed.len()
                    )
                };
                n.set(Some(msg));
                sel.set(Vec::new());
            });
        }
    };
    // 两个批量按钮各自持有独立执行闭包实例(工厂按 target 生成的两份)
    let bulk_enable = make_bulk_toggle(1);
    let bulk_disable = make_bulk_toggle(2);

    // 弹窗关闭并触发重拉
    let close_and_reload = move |_| {
        modal_state.set(ModalState::Closed);
        reload.set(reload() + 1);
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            role: "region",
            "aria-label": "分组管理",
            // 通知条(成功/错误/进行中)
                if let Some(msg) = notice() {
                    div { class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                        "{msg}"
                        if busy() { " ···" }
                    }
                }

                // 1. 概览统计区
                section { id: "groups-sec-stats", class: "scroll-mt-8 space-y-3",
                    h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                    div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                        for (value, label) in stats {
                            StatCard { value, label }
                        }
                    }
                }

                // 2. 筛选与操作区
                section {
                    id: "groups-sec-filter",
                    class: "scroll-mt-8 flex flex-col gap-4 rounded-xl border border-zinc-800 bg-zinc-900 p-5",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-sm font-medium text-zinc-300", "{SEC_FILTER}" }
                            span { class: "text-xs text-zinc-500", "按倍率分级或关键词筛选" }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white",
                                "data-testid": "refresh-groups",
                                onclick: move |_| reload.set(reload() + 1),
                                "刷新"
                            }
                            button {
                                class: "shrink-0 rounded-xl bg-white px-4 py-2 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-200 active:bg-zinc-300",
                                "data-testid": "new-group",
                                onclick: open_new,
                                "✚ 新建分组"
                            }
                        }
                    }

                    input {
                        class: "w-full rounded-xl border border-zinc-700/80 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-100 placeholder:text-zinc-500 outline-none transition focus:border-zinc-500",
                        r#type: "text",
                        "data-testid": "group-search",
                        placeholder: "搜索分组标识或备注...",
                        value: "{search}",
                        oninput: move |e| search.set(e.value()),
                    }

                    // 分类胶囊
                    div { class: "flex flex-wrap gap-2",
                        SegmentedCapsule {
                            items: filter_options,
                            active: filter_tier(),
                            on_select: move |i: usize| filter_tier.set(i),
                        }
                    }

                    // 批量多选区 (卡牌外): 列全部分组 chips, 点选加入/移出选中集合;
                    // 选中数>0 时下方出现批量动作条 (批量启停 / 清除)。
                    div { class: "space-y-2",
                        p { class: "mb-1.5 text-[11px] text-zinc-500", "批量操作: 点选分组" }
                        div { class: "flex flex-wrap gap-1.5",
                            "data-testid": "bulk-select",
                            for g in groups().iter().cloned() {
                                {
                                    let key = g.key.clone();
                                    let gn = g.name.clone();
                                    let picked = selected.peek().contains(&key);
                                    let cls = if picked {
                                        "border-zinc-100 bg-zinc-100 text-zinc-900 font-semibold"
                                    } else {
                                        "border-zinc-700 bg-zinc-900 text-zinc-300 hover:border-zinc-500"
                                    };
                                    rsx! {
                                        button {
                                            class: "rounded-lg border px-2.5 py-1 text-xs transition-colors {cls}",
                                            "data-testid": "bulk-select-chip",
                                            onclick: move |_| {
                                                let mut s = selected.peek().to_vec();
                                                if let Some(pos) = s.iter().position(|k| *k == key) {
                                                    s.remove(pos);
                                                } else {
                                                    s.push(key.clone());
                                                }
                                                selected.set(s);
                                            },
                                            "{gn}"
                                        }
                                    }
                                }
                            }
                        }
                        // 批量动作条: 勾选后出现
                        if !selected().is_empty() {
                            div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/80 bg-zinc-950 px-3 py-2.5",
                                "data-testid": "bulk-bar",
                                span { class: "text-xs text-zinc-400", "已选 {selected().len()} 项" }
                                button {
                                    class: "rounded-lg border border-emerald-700/50 bg-emerald-900/30 px-2.5 py-1 text-xs font-medium text-emerald-400 transition-colors hover:bg-emerald-800/50",
                                    "data-testid": "bulk-enable",
                                    onclick: bulk_enable,
                                    "批量启用"
                                }
                                button {
                                    class: "rounded-lg border border-zinc-700/80 bg-zinc-800/60 px-2.5 py-1 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-700",
                                    "data-testid": "bulk-disable",
                                    onclick: bulk_disable,
                                    "批量停用"
                                }
                                button {
                                    class: "rounded-lg border border-zinc-700/80 px-2.5 py-1 text-xs text-zinc-400 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                                    "data-testid": "bulk-clear",
                                    onclick: move |_| selected.set(Vec::new()),
                                    "清除"
                                }
                            }
                        }
                    }
                }

                // 3. 卡片网格区
                section { id: "groups-sec-list", class: "scroll-mt-8 space-y-4",
                    div { class: "flex items-center justify-between",
                        h2 { class: "text-lg font-medium text-zinc-100", "{SEC_LIST}" }
                        span { class: "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400",
                            if loading() { "加载中…" } else { "{filtered.len()} 组" }
                        }
                    }

                    if let Some(e) = err() {
                        div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 py-10 text-center",
                            p { class: "text-sm text-red-300", "加载分组失败" }
                            p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                            button {
                                class: "mt-3 rounded-xl border border-zinc-700 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-800",
                                "data-testid": "retry-groups",
                                onclick: move |_| reload.set(reload() + 1),
                                "重试"
                            }
                        }
                    } else if loading() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "正在加载分组…" }
                        }
                    } else if filtered.is_empty() {
                        div { class: "rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-16 text-center",
                            p { class: "text-zinc-400", "没有匹配的分组" }
                        }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            "data-testid": "groups-list",
                            for g in filtered {
                                {
                                    let edit_key = g.key.clone();
                                    let delete_key = g.key.clone();
                                    let toggle_key = g.key.clone();
                                    let ratio_key = g.key.clone();
                                    let current_status = g.status;
                                    let is_default = g.name == "default";
                                    // 启用/停用:按当前 status 取目标值 (1↔2);
                                    // target 在 group 值移入 rsx 前算好, 避免 move 后再借用
                                    let toggle_target = if current_status == 1 { 2 } else { 1 };
                                    let on_toggle_status = move |_| {
                                        write_toggle(
                                            toggle_key.clone(),
                                            WriteOp::ToggleStatus(toggle_target),
                                        );
                                    };
                                    // 倍率滑条松手写回: 复用同一写工厂, 就地更新本地 ratio
                                    let on_ratio_drag = move |v: f64| {
                                        write_toggle(
                                            ratio_key.clone(),
                                            WriteOp::SetRatio(v),
                                        );
                                    };
                                    rsx! {
                                        GroupCard {
                                            key: "{g.key}",
                                            group: g,
                                            is_default,
                                            on_edit: move |_| open_edit(edit_key.clone()),
                                            on_delete: move |_| write_delete(delete_key.clone(), WriteOp::Delete),
                                            on_toggle_status,
                                            on_ratio_drag,
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 新建 / 编辑弹窗
            if matches!(modal_state(), ModalState::New | ModalState::Edit(_)) {
                GroupFormModal {
                    editing: matches!(modal_state(), ModalState::Edit(_)),
                    group_key: match modal_state() {
                        ModalState::Edit(k) => Some(k),
                        _ => None,
                    },
                    name: f_name,
                    ratio: f_ratio,
                    remark: f_remark,
                    whitelist: f_whitelist,
                    alias_options: alias_options(),
                    f_alias,
                    on_cancel: move |_| modal_state.set(ModalState::Closed),
                    on_submit: close_and_reload,
                }
            }
    }
}

// ============ 组件 ============

#[component]
pub(crate) fn Badge(text: String, tone: &'static str) -> Element {
    rsx! {
        span { class: "rounded-full border px-2 py-0.5 text-[11px] font-medium {tone}",
            "{text}"
        }
    }
}

/// 单个分组卡片 (对齐 UserCard 风格)
#[component]
fn GroupCard(
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
fn GroupFormModal(
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
