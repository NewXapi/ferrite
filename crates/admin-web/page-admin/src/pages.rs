//! 管理区功能页:每个 tab 一个操作面板页,面板按 1/3/5 栏响应式铺开
//! (手机 1 栏 / 平板 3 栏 / 桌面 5 栏)。交互对齐 new-api 对应功能区:
//! 渠道的状态速览/编辑/调度/批量,别名的计费,订阅与兑换码的生成与审计。
//!
//! 数据全走 `state::EntityStore`(mock);接 API 时把初始值换成请求结果即可。
//!
//! 布局约定(与项目 gate-checklist 一致):
//! - 桌面端面板间用「分隔线 + 独占行」表达从属关系,不占标签页;
//! - 交互控件以原生为主(select / number input / checkbox),自定义件必须带状态语义;
//! - 反馈一致:确认用「已保存/已生成/已测速」文字,危险操作用红色。

use dioxus::prelude::*;

use crate::entities::{EntityChip, InputCell, SelectCell, TextCell};
use crate::state::{CHANNEL_TYPES, EntityStore, PlanRow, RedRow};

// ============ 文案常量 (>=2 次复用) ============
const BTN_SAVE: &str = "保存";
const BTN_CANCEL: &str = "取消";
const STATUS_DISABLED: &str = "停用";
const KEY_ANNOUNCEMENT: &str = "站点公告";
const KEY_SIGNIN_BONUS: &str = "签到奖励";
const KEY_SENSITIVE_WORDS: &str = "敏感词";
const KEY_NOTICE: &str = "公告";
const DEFAULT_GROUP: &str = "默认分组";

// ============ 页面骨架 ============

/// 1/3 栏响应式网格(手机 1 / 平板与Web 3 栏)。
#[component]
pub fn GridShell(children: Element) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3", {children} }
    }
}

/// 面板基础件:标题 + 说明 + 内容。
#[component]
pub fn Panel(title: &'static str, hint: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            p { class: "text-sm font-medium text-zinc-100", "{title}" }
            p { class: "text-[11px] text-zinc-600", "{hint}" }
            {children}
        }
    }
}

/// 主按钮(确认 / 保存 / 生成等)。
#[component]
pub(crate) fn PushBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 危险操作(删除 / 停用)。
#[component]
pub(crate) fn DangerBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-red-900/60 px-3 py-1.5 text-xs text-red-400 hover:border-red-700",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 幽灵操作(清空 / 取消)。
#[component]
pub(crate) fn GhostBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 状态开关(对齐 new-api 的启用/停用徽章;带 on/off 文字态)。
#[component]
pub(crate) fn ToggleSwitch(on: bool, on_toggle: EventHandler<()>) -> Element {
    let track = if on { "bg-zinc-100" } else { "bg-zinc-700" };
    let knob = if on { "translate-x-4" } else { "translate-x-0" };
    rsx! {
        button {
            class: "relative h-5 w-9 shrink-0 rounded-full transition-colors {track}",
            role: "switch",
            "aria-checked": "{on}",
            onclick: move |_| on_toggle.call(()),
            span { class: "absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-zinc-950 transition-transform {knob}" }
        }
    }
}

/// 渠道状态文字(1 启用 / 0 手动停用 / 2 自动停用)。
fn channel_status_label(status: u8) -> &'static str {
    match status {
        1 => "启用",
        2 => "自动停用",
        _ => STATUS_DISABLED,
    }
}

/// 兑换码状态文字(1 未用 / 2 停用 / 3 已用)。
fn redemption_status_label(status: u8) -> &'static str {
    match status {
        2 => STATUS_DISABLED,
        3 => "已用",
        _ => "未用",
    }
}


// ============ 渠道页 ============

/// 渠道管理:5 块 = 顶部导入条 + 4 面板(状态速览、编辑渠道、模型调度、批量绑定)。
#[component]
pub fn ChannelsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut channels = store.channels;
    let mut current = use_signal(|| 0usize);
    let mut testing = use_signal(|| None::<usize>);
    let mut test_counter = use_signal(|| 0u32);
    let mut move_target = use_signal(|| String::from("default"));

    let count = channels.read().len();
    let idx = current().min(count.saturating_sub(1));

    rsx! {
        GridShell {
            // 顶部导入条:URL + Key + 渠道名,一键写入 store
            ChannelImportBar {}
            Panel {
                title: "渠道状态速览",
                hint: "点行加载到编辑;开关=启停,测速=mock 往返",
                if count == 0 {
                    p { class: "text-[11px] text-zinc-600", "还没有渠道,从右侧新建" }
                } else {
                    div { class: "space-y-1",
                        for (i, c) in channels.read().iter().enumerate() {
                            {
                                let name = c.name.clone();
                                let tone = if i == idx { "border-zinc-600 bg-zinc-900" } else { "border-zinc-800 bg-zinc-950/60 hover:border-zinc-700" };
                                rsx! {
                                    div { class: "rounded-lg border p-2 {tone}",
                                        button {
                                            class: "flex w-full items-center justify-between gap-2 text-left",
                                            onclick: move |_| current.set(i),
                                            span { class: "truncate text-xs text-zinc-200", "{name}" }
                                            span { class: "shrink-0 text-[11px] text-zinc-500", "{c.ctype} · {c.group}" }
                                        }
                                        div { class: "mt-1.5 flex items-center justify-between gap-2",
                                            span { class: "text-[11px] text-zinc-500",
                                                "{channel_status_label(c.status)}"
                                                if let Some(ms) = c.latency_ms {
                                                    " · {ms}ms"
                                                } else {
                                                    " · 未测"
                                                }
                                            }
                                            div { class: "flex items-center gap-2",
                                                ToggleSwitch {
                                                    on: c.status == 1,
                                                    on_toggle: move |_| {
                                                        let mut w = channels.write();
                                                        w[i].status = if w[i].status == 1 { 0 } else { 1 };
                                                    },
                                                }
                                                button {
                                                    class: "text-[11px] text-zinc-400 hover:text-zinc-200 disabled:opacity-40",
                                                    disabled: testing() == Some(i),
                                                    onclick: move |_| {
                                                        testing.set(Some(i));
                                                        let n = test_counter.peek().wrapping_add(1);
                                                        test_counter.set(n);
                                                        let ms = 120 + (n.wrapping_mul(97) % 380);
                                                        spawn(async move {
                                                            gloo_timers::future::TimeoutFuture::new(500).await;
                                                            channels.write()[i].latency_ms = Some(ms);
                                                            testing.set(None);
                                                        });
                                                    },
                                                    if testing() == Some(i) { "…" } else { "测速" }
                                                }
                                                button {
                                                    class: "text-[11px] text-red-500 hover:text-red-400",
                                                    onclick: move |_| { channels.write().remove(i); },
                                                    "✕"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 面板 2:编辑渠道(new-api 的编辑抽屉:类型/名称/URL/Key/分组)
            if let Some(c) = channels.read().get(idx).cloned() {
                Panel {
                    title: "编辑渠道",
                    hint: "改字段即时生效(mock)",
                    div { class: "space-y-2",
                        SelectCell {
                            label: "类型",
                            value: c.ctype.clone(),
                            options: CHANNEL_TYPES,
                            oninput: move |v: String| channels.write()[idx].ctype = v,
                        }
                        TextCell {
                            label: "渠道名称",
                            value: c.name.clone(),
                            placeholder: "OpenAI 官方",
                            oninput: move |v: String| channels.write()[idx].name = v,
                        }
                        TextCell {
                            label: "Base URL",
                            value: c.url.clone(),
                            placeholder: "https://…",
                            oninput: move |v: String| channels.write()[idx].url = v,
                        }
                        TextCell {
                            label: "API Key",
                            value: c.keys.clone(),
                            placeholder: "sk-…",
                            oninput: move |v: String| channels.write()[idx].keys = v,
                        }
                        label { class: "block space-y-1",
                            span { class: "text-[11px] text-zinc-500", "绑定分组" }
                            select {
                                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors focus:border-zinc-500",
                                value: "{c.group}",
                                oninput: move |e| channels.write()[idx].group = e.value(),
                                for g in store.groups.read().iter() {
                                    option { value: "{g.name}", selected: g.name == c.group, "{g.name}" }
                                }
                            }
                        }
                    }
                }
            }

            // 面板 3:模型调度(候补池 → 加入调度 → 拓扑)
            Panel {
                title: "模型调度",
                hint: "拉取上游 → 勾选候补 → 加入调度(进拓扑)",
                div { class: "space-y-2",
                    div { class: "flex items-center justify-between gap-2",
                        p { class: "text-[11px] text-zinc-400", "候补池" }
                        GhostBtn {
                            label: "拉取模型",
                            on_click: move |_| {
                                let pool = ["gpt-4o", "gpt-4o-mini", "gpt-5", "o3", "o3-mini"];
                                let mut w = channels.write();
                                let c = &mut w[idx];
                                let have: Vec<String> = c
                                    .candidates
                                    .iter()
                                    .map(|(n, _)| n.clone())
                                    .chain(c.dispatch.iter().cloned())
                                    .collect();
                                for m in pool {
                                    if !have.iter().any(|x| x == m) {
                                        c.candidates.push((m.to_string(), false));
                                    }
                                }
                            },
                        }
                    }
                    if channels.read()[idx].candidates.is_empty() {
                        p { class: "text-[11px] text-zinc-600", "点「拉取模型」获取候补" }
                    } else {
                        div { class: "space-y-0.5",
                            for (j, (m, on)) in channels.read()[idx].candidates.iter().enumerate() {
                                {
                                    let label = m.clone();
                                    let checked = *on;
                                    rsx! {
                                        label { class: "flex cursor-pointer items-center gap-2 rounded px-1.5 py-1 hover:bg-zinc-900",
                                            input {
                                                r#type: "checkbox",
                                                class: "accent-zinc-100",
                                                checked: checked,
                                                onchange: move |_| {
                                                    let mut w = channels.write();
                                                    let v = w[idx].candidates[j].1;
                                                    w[idx].candidates[j].1 = !v;
                                                },
                                            }
                                            span { class: "font-mono text-xs text-zinc-400", "{label}" }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "flex gap-1.5 border-t border-zinc-800 pt-2",
                            PushBtn {
                                label: "加入调度",
                                on_click: move |_| {
                                    let mut w = channels.write();
                                    let picked: Vec<String> = w[idx]
                                        .candidates
                                        .iter()
                                        .filter(|(_, on)| *on)
                                        .map(|(n, _)| n.clone())
                                        .collect();
                                    for p in &picked {
                                        if !w[idx].dispatch.contains(p) {
                                            w[idx].dispatch.push(p.clone());
                                        }
                                    }
                                    w[idx].candidates.retain(|(n, on)| !(*on && picked.contains(n)));
                                },
                            }
                            GhostBtn {
                                label: "清空",
                                on_click: move |_| channels.write()[idx].candidates.clear(),
                            }
                        }
                    }
                    div { class: "border-t border-zinc-800 pt-2",
                        p { class: "text-[11px] text-zinc-400", "调度模型(已进拓扑)" }
                        if channels.read()[idx].dispatch.is_empty() {
                            p { class: "mt-1 text-[11px] text-zinc-600", "从候补池加入" }
                        } else {
                            div { class: "mt-1 space-y-1",
                                for (j, m) in channels.read()[idx].dispatch.iter().enumerate() {
                                    {
                                        let label = m.clone();
                                        rsx! {
                                            div { class: "flex items-center justify-between rounded border border-zinc-800 bg-zinc-900 px-2 py-1",
                                                span { class: "truncate font-mono text-xs text-zinc-200", "{label}" }
                                                button {
                                                    class: "shrink-0 text-[11px] text-zinc-600 hover:text-red-400",
                                                    title: "移出拓扑,退回候补池",
                                                    onclick: move |_| {
                                                        let mut w = channels.write();
                                                        let m = w[idx].dispatch.remove(j);
                                                        w[idx].candidates.push((m, false));
                                                    },
                                                    "✕"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // 面板 4:批量操作(对齐 new-api 的 data-table-bulk-actions)
            Panel {
                title: "批量绑定分组",
                hint: "把全部渠道切到目标分组(拓扑随之更新)",
                div { class: "space-y-2",
                    label { class: "block space-y-1",
                        span { class: "text-[11px] text-zinc-500", "目标分组" }
                        select {
                            class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none focus:border-zinc-500",
                            value: "{move_target()}",
                            oninput: move |e| move_target.set(e.value()),
                            for g in store.groups.read().iter() {
                                option { value: "{g.name}", selected: g.name == move_target(), "{g.name}" }
                            }
                        }
                    }
                    PushBtn {
                        label: "应用到全部渠道",
                        on_click: move |_| {
                            let t = move_target();
                            for c in channels.write().iter_mut() {
                                c.group = t.clone();
                            }
                        },
                    }
                }
            }
        }
    }
}

// ============ 别名页 ============

/// 别名 + 计费:3 个面板 = 列表、新增/编辑、计费速览。
#[component]
pub fn AliasesPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut aliases = store.aliases;
    let groups = store.groups;
    let mut name = use_signal(String::new);
    let mut display = use_signal(String::new);
    let mut input_rate = use_signal(String::new);
    let mut output_rate = use_signal(String::new);
    let mut mult = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let d = display.peek().trim().to_string();
        let parse_pos = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let parse_m = |s: &str| s.trim().parse::<f64>().unwrap_or(1.0).max(0.0);
        match *editing.peek() {
            Some(i) => {
                aliases.write()[i].alias = n;
                aliases.write()[i].display = d;
                aliases.write()[i].input_per_1k = parse_pos(&input_rate.peek());
                aliases.write()[i].output_per_1k = parse_pos(&output_rate.peek());
                aliases.write()[i].multiplier = parse_m(&mult.peek());
            }
            None => aliases.write().push(crate::state::AliasRow {
                alias: n,
                display: d,
                input_per_1k: parse_pos(&input_rate.peek()),
                output_per_1k: parse_pos(&output_rate.peek()),
                multiplier: parse_m(&mult.peek()),
            }),
        }
        name.set(String::new());
        display.set(String::new());
        input_rate.set(String::new());
        output_rate.set(String::new());
        mult.set(String::new());
        editing.set(None);
    };

    rsx! {
        GridShell {
            Panel {
                title: "别名列表",
                hint: "点别名进入编辑;✕ 删除(拓扑边同步消失)",
                if aliases.read().is_empty() {
                    p { class: "text-[11px] text-zinc-600", "还没有别名" }
                } else {
                    div { class: "flex flex-wrap gap-2",
                        for (i, r) in aliases.read().iter().enumerate() {
                            EntityChip {
                                label: r.alias.clone(),
                                sub: if r.display.is_empty() {
                                    format!("×{}", r.multiplier)
                                } else {
                                    format!("{} ×{}", r.display, r.multiplier)
                                },
                                active: editing() == Some(i),
                                on_pick: move |_| {
                                    let r = aliases.read()[i].clone();
                                    name.set(r.alias);
                                    display.set(r.display);
                                    input_rate.set(format!("{}", r.input_per_1k));
                                    output_rate.set(format!("{}", r.output_per_1k));
                                    mult.set(format!("{}", r.multiplier));
                                    editing.set(Some(i));
                                },
                                on_remove: move |_| {
                                    aliases.write().remove(i);
                                    if editing() == Some(i) { editing.set(None); }
                                },
                            }
                        }
                    }
                }
            }

            Panel {
                title: "新增 / 编辑别名",
                hint: "别名为空不提交;单价 ≥ 0,倍率默认 1",
                div { class: "space-y-2",
                    InputCell { label: "别名", value: name, placeholder: "gpt-4o", grow: true }
                    InputCell { label: "展示名", value: display, placeholder: "GPT-4o(可选)", grow: true }
                    InputCell { label: "输入价 ¥/1k", value: input_rate, placeholder: "0.0175" }
                    InputCell { label: "输出价 ¥/1k", value: output_rate, placeholder: "0.07" }
                    InputCell { label: "倍率", value: mult, placeholder: "1.0" }
                    div { class: "flex gap-2 pt-1",
                        PushBtn {
                            label: BTN_SAVE,
                            on_click: commit,
                        }
                        if editing().is_some() {
                            GhostBtn {
                                label: BTN_CANCEL,
                                on_click: move |_| {
                                    editing.set(None);
                                    name.set(String::new());
                                    display.set(String::new());
                                    input_rate.set(String::new());
                                    output_rate.set(String::new());
                                    mult.set(String::new());
                                },
                            }
                        }
                    }
                }
            }

            Panel {
                title: "计费速览",
                hint: "每 1k tokens 价(¥);分组倍率见「分组」页",
                div { class: "space-y-1.5",
                    for r in aliases.read().iter() {
                        div { class: "rounded-lg border border-zinc-800 bg-zinc-950/60 px-2.5 py-1.5",
                            p { class: "truncate font-mono text-xs text-zinc-200", "{r.alias}" }
                            p { class: "mt-0.5 text-[11px] text-zinc-500",
                                "入 {r.input_per_1k} / 出 {r.output_per_1k} / ×{r.multiplier}"
                            }
                        }
                    }
                }
                div { class: "rounded-lg border border-dashed border-zinc-800 px-2.5 py-2 text-[11px] text-zinc-600",
                    "分组:共 {groups.read().len()} 个,倍率在各分组卡上"
                }
            }
        }
    }
}

// ============ 订阅页 ============

/// 订阅管理: 单栏卡牌展示 (web/平板/手机均为 1 栏) + 多 Tab 编辑弹窗 (对齐 new-api 订阅配置)
#[component]
pub fn SubscriptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut plans = store.plans;
    let groups = store.groups;

    let mut show_modal = use_signal(|| false);
    let mut modal_tab = use_signal(|| 0u8);
    let mut editing_idx = use_signal(|| None::<usize>);

    // 基本信息表单字段
    let mut f_id = use_signal(|| 0u32);
    let mut f_title = use_signal(String::new);
    let mut f_subtitle = use_signal(String::new);
    let mut f_price = use_signal(|| "0".to_string());
    let mut f_quota = use_signal(|| "0".to_string());
    let mut f_currency_price = use_signal(|| "0".to_string());
    let mut f_payment_method = use_signal(|| "仅扣菌种".to_string());
    let mut f_group = use_signal(|| "不升级".to_string());
    let mut f_downgrade_group = use_signal(|| "降级到购买前分组".to_string());
    let mut f_limit = use_signal(|| "0".to_string());
    let mut f_sort = use_signal(|| "0".to_string());

    // 规则与周期字段
    let mut f_enabled = use_signal(|| true);
    let mut f_allow_redeem = use_signal(|| true);
    let mut f_allow_wallet = use_signal(|| true);
    let mut f_period_val = use_signal(|| "1".to_string());
    let mut f_period_unit = use_signal(|| "个月".to_string());
    let mut f_reset_cycle = use_signal(|| "不重置".to_string());

    // 第三方支付字段
    let mut f_stripe_id = use_signal(String::new);
    let mut f_creem_id = use_signal(String::new);
    let mut f_waffo_id = use_signal(String::new);

    let mut open_edit = move |i: usize| {
        let p = plans.read()[i].clone();
        f_id.set(p.id);
        f_title.set(p.title);
        f_subtitle.set(p.subtitle);
        f_price.set(format!("{}", p.price));
        f_quota.set(format!("{}", p.quota));
        f_currency_price.set(format!("{}", p.currency_price));
        f_payment_method.set(p.payment_method);
        f_group.set(p.group);
        f_downgrade_group.set(p.downgrade_group);
        f_limit.set(format!("{}", p.max_per_user));
        f_sort.set(format!("{}", p.sort_order));
        f_enabled.set(p.enabled);
        f_allow_redeem.set(p.allow_redeem);
        f_allow_wallet.set(p.allow_wallet);
        f_period_val.set(format!("{}", p.period_val));
        f_period_unit.set(p.period_unit);
        f_reset_cycle.set(p.reset_cycle);
        f_stripe_id.set(p.stripe_price_id);
        f_creem_id.set(p.creem_product_id);
        f_waffo_id.set(p.waffo_product_id);
        editing_idx.set(Some(i));
        modal_tab.set(0);
        show_modal.set(true);
    };

    let open_new = move |_| {
        let next_id = plans.read().iter().map(|p| p.id).max().unwrap_or(0) + 1;
        f_id.set(next_id);
        f_title.set(String::new());
        f_subtitle.set(String::new());
        f_price.set("0".to_string());
        f_quota.set("0".to_string());
        f_currency_price.set("0".to_string());
        f_payment_method.set("仅扣菌种".to_string());
        f_group.set("不升级".to_string());
        f_downgrade_group.set("降级到购买前分组".to_string());
        f_limit.set("0".to_string());
        f_sort.set("0".to_string());
        f_enabled.set(true);
        f_allow_redeem.set(true);
        f_allow_wallet.set(true);
        f_period_val.set("1".to_string());
        f_period_unit.set("个月".to_string());
        f_reset_cycle.set("不重置".to_string());
        f_stripe_id.set(String::new());
        f_creem_id.set(String::new());
        f_waffo_id.set(String::new());
        editing_idx.set(None);
        modal_tab.set(0);
        show_modal.set(true);
    };

    let commit = move |_| {
        let t = f_title.peek().trim().to_string();
        if t.is_empty() {
            return;
        }
        let row = PlanRow {
            id: f_id(),
            title: t,
            subtitle: f_subtitle.peek().trim().to_string(),
            price: f_price.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            quota: f_quota.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            currency_price: f_currency_price.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0),
            payment_method: f_payment_method(),
            group: f_group(),
            downgrade_group: f_downgrade_group(),
            period_val: f_period_val.peek().trim().parse::<u32>().unwrap_or(1).max(1),
            period_unit: f_period_unit(),
            reset_cycle: f_reset_cycle(),
            priority: 0,
            enabled: f_enabled(),
            allow_redeem: f_allow_redeem(),
            allow_wallet: f_allow_wallet(),
            max_per_user: f_limit.peek().trim().parse::<u32>().unwrap_or(0),
            sort_order: f_sort.peek().trim().parse::<i32>().unwrap_or(0),
            stripe_price_id: f_stripe_id.peek().trim().to_string(),
            creem_product_id: f_creem_id.peek().trim().to_string(),
            waffo_product_id: f_waffo_id.peek().trim().to_string(),
        };
        match *editing_idx.peek() {
            Some(i) => {
                plans.write()[i] = row;
            }
            None => plans.write().insert(0, row),
        }
        show_modal.set(false);
    };

    rsx! {
        div { class: "flex flex-col gap-4 w-full",
            // 顶部栏: 提示横幅 + 新建按钮
            div { class: "flex flex-wrap items-center justify-between gap-3 rounded-xl border border-amber-500/20 bg-amber-500/5 px-4 py-3",
                div { class: "flex items-center gap-2 text-xs text-amber-300",
                    span { class: "flex h-5 w-5 items-center justify-center rounded-full bg-amber-500/20 font-bold", "ℹ" }
                    span { "Stripe / Creem 需在第三方平台创建商品并填入 ID" }
                }
                button {
                    class: "flex items-center gap-1.5 rounded-lg bg-amber-400 px-3.5 py-1.5 text-xs font-semibold text-zinc-950 transition-colors hover:bg-amber-300 shadow-sm",
                    onclick: open_new,
                    span { class: "text-sm", "+" }
                    "新建套餐"
                }
            }

            // 单栏卡牌列表容器 (Web / 平板 / 手机统一一栏优雅排布)
            div { class: "flex flex-col gap-3",
                for (i, p) in plans.read().iter().enumerate() {
                    {
                        let title_txt = p.title.clone();
                        let sub_txt = p.subtitle.clone();
                        let price_str = format!("${:.2}", p.price);
                        let quota_str = if p.quota <= 0.0 { "无限制".to_string() } else { format!("{}", p.quota) };
                        let period_str = format!("{} {}", p.period_val, p.period_unit);
                        rsx! {
                            div {
                                key: "{p.id}",
                                class: "group flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-700 hover:bg-zinc-900/90 shadow-md",

                                // 卡片头部行: ID + 标题 + 状态/分组徽标 + 操作按钮
                                div { class: "flex flex-wrap items-start justify-between gap-2.5",
                                    div { class: "flex items-center gap-2.5 min-w-0 flex-1",
                                        span { class: "shrink-0 rounded-md border border-zinc-700/80 bg-zinc-800 px-2 py-0.5 text-xs font-mono font-bold text-zinc-300",
                                            "#{p.id}"
                                        }
                                        h3 { class: "truncate text-base font-bold text-zinc-100", "{title_txt}" }
                                        span {
                                            class: if p.enabled { "rounded-full border border-emerald-500/30 bg-emerald-500/10 px-2.5 py-0.5 text-[11px] font-medium text-emerald-400" } else { "rounded-full border border-zinc-700 bg-zinc-800/80 px-2.5 py-0.5 text-[11px] font-medium text-zinc-500" },
                                            if p.enabled { "启用" } else { "禁用" }
                                        }
                                        if !p.group.is_empty() && p.group != "不升级" {
                                            span { class: "rounded-full border border-sky-500/30 bg-sky-500/10 px-2.5 py-0.5 text-[11px] font-medium text-sky-400 uppercase",
                                                "分组: {p.group}"
                                            }
                                        }
                                    }
                                    div { class: "flex items-center gap-2 shrink-0",
                                        ToggleSwitch {
                                            on: p.enabled,
                                            on_toggle: move |_| {
                                                let mut w = plans.write();
                                                w[i].enabled = !w[i].enabled;
                                            },
                                        }
                                        button {
                                            class: "rounded-lg border border-zinc-700 bg-zinc-800 px-2.5 py-1 text-xs text-zinc-200 transition-colors hover:bg-zinc-700 hover:text-white",
                                            onclick: move |_| open_edit(i),
                                            "编辑"
                                        }
                                        button {
                                            class: "rounded-lg border border-red-900/50 bg-red-950/20 px-2 py-1 text-xs text-red-400 transition-colors hover:bg-red-900/30 hover:text-red-300",
                                            onclick: move |_| { plans.write().remove(i); },
                                            "✕"
                                        }
                                    }
                                }

                                // 副标题
                                if !sub_txt.is_empty() {
                                    p { class: "mt-1.5 text-xs text-zinc-400 leading-relaxed", "{sub_txt}" }
                                }

                                // 关键指标条 (对标 Image #5 字段)
                                div { class: "mt-3.5 grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3 pt-3 border-t border-zinc-800/70 text-xs",
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "价格" }
                                        span { class: "font-mono font-bold text-sm text-emerald-400", "{price_str}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "有效期" }
                                        span { class: "font-medium text-zinc-200", "{period_str}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "套餐额度" }
                                        span { class: "font-mono font-semibold text-amber-300 flex items-center gap-1",
                                            span { "🧀" }
                                            span { "{quota_str}" }
                                        }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "站内支付 / 渠道" }
                                        span { class: "text-zinc-300 font-medium", "{p.payment_method}" }
                                    }
                                    div {
                                        span { class: "text-[11px] text-zinc-500 block", "额度重置" }
                                        span { class: "text-zinc-400", "{p.reset_cycle}" }
                                    }
                                }

                                // 第三方配置徽标展示
                                if !p.stripe_price_id.is_empty() || !p.creem_product_id.is_empty() {
                                    div { class: "mt-2.5 flex flex-wrap gap-2 text-[10px] text-zinc-500 font-mono",
                                        if !p.stripe_price_id.is_empty() {
                                            span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Stripe: {p.stripe_price_id}" }
                                        }
                                        if !p.creem_product_id.is_empty() {
                                            span { class: "rounded bg-zinc-950 px-1.5 py-0.5 border border-zinc-800", "Creem: {p.creem_product_id}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // ============ 多 Tab 编辑/新建弹窗 (对标 Image #6, #7, #8) ============
        if show_modal() {
            div {
                class: "fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4 backdrop-blur-sm",
                onclick: move |_| show_modal.set(false),
                div {
                    class: "w-full max-w-2xl rounded-2xl border border-zinc-800 bg-zinc-900 p-6 shadow-2xl space-y-5 max-h-[90vh] overflow-y-auto scroll-subtle",
                    onclick: move |e| e.stop_propagation(),

                    // 弹窗头部
                    div { class: "flex items-start justify-between",
                        div {
                            h3 { class: "text-lg font-bold text-zinc-100",
                                if editing_idx().is_some() { "更新套餐信息" } else { "新建订阅套餐" }
                            }
                            p { class: "mt-0.5 text-xs text-zinc-400", "修改现有订阅套餐的配置" }
                        }
                        button {
                            class: "rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                            onclick: move |_| show_modal.set(false),
                            "✕"
                        }
                    }

                    // 弹窗内部 Tab 切换条
                    div { class: "flex items-center gap-2 border-b border-zinc-800 pb-2 text-xs",
                        button {
                            class: if modal_tab() == 0 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(0),
                            "🔑 基本信息"
                        }
                        button {
                            class: if modal_tab() == 1 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(1),
                            "📅 规则与周期"
                        }
                        button {
                            class: if modal_tab() == 2 { "rounded-lg bg-zinc-800 px-3 py-1.5 font-semibold text-zinc-100" } else { "rounded-lg px-3 py-1.5 text-zinc-400 hover:text-zinc-200" },
                            onclick: move |_| modal_tab.set(2),
                            "💳 第三方支付配置"
                        }
                    }

                    // ---- Tab 0: 基本信息 (Image #6) ----
                    if modal_tab() == 0 {
                        div { class: "space-y-4 pt-1",
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "套餐标题" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                    value: "{f_title()}",
                                    placeholder: "例如：开拓的封赏",
                                    oninput: move |e| f_title.set(e.value()),
                                }
                            }
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "套餐副标题" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                    value: "{f_subtitle()}",
                                    placeholder: "向你们致敬，向外开拓的勇士们！",
                                    oninput: move |e| f_subtitle.set(e.value()),
                                }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "套餐价格 ($)" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_price()}",
                                        oninput: move |e| f_price.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "用户购买该套餐需支付的金额，具体币种由支付渠道决定" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "额度 (🧀)" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_quota()}",
                                        oninput: move |e| f_quota.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "套餐包含的总额度，每个计费周期可用；0 表示不限量" }
                                }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "套餐价格（菌种）" }
                                    input {
                                        r#type: "number",
                                        step: "0.1",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_currency_price()}",
                                        oninput: move |e| f_currency_price.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "最小单位 0.1。仅当支付方式包含它时才生效。" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "站内支付方式" }
                                    select {
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_payment_method()}",
                                        oninput: move |e| f_payment_method.set(e.value()),
                                        option { value: "仅扣菌种", "仅扣菌种" }
                                        option { value: "允许余额兑换", "允许余额兑换" }
                                        option { value: "无限制", "无限制" }
                                    }
                                    p { class: "text-[11px] text-zinc-500", "只影响站内货币，不影响第三方支付渠道。" }
                                }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "升级分组" }
                                    select {
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_group()}",
                                        oninput: move |e| f_group.set(e.value()),
                                        option { value: "不升级", "不升级" }
                                        for g in groups.read().iter() {
                                            option { value: "{g.name}", "{g.name}" }
                                        }
                                    }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "降级分组" }
                                    select {
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_downgrade_group()}",
                                        oninput: move |e| f_downgrade_group.set(e.value()),
                                        option { value: "降级到购买前分组", "降级到购买前分组" }
                                        option { value: "默认分组", "默认分组" }
                                    }
                                    p { class: "text-[11px] text-zinc-500", "订阅过期后降级到该分组" }
                                }
                            }
                            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "限购" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_limit()}",
                                        oninput: move |e| f_limit.set(e.value()),
                                    }
                                    p { class: "text-[11px] text-zinc-500", "0 表示不限" }
                                }
                                label { class: "block space-y-1",
                                    span { class: "text-xs font-medium text-zinc-300", "排序" }
                                    input {
                                        r#type: "number",
                                        class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                        value: "{f_sort()}",
                                        oninput: move |e| f_sort.set(e.value()),
                                    }
                                }
                            }
                        }
                    }

                    // ---- Tab 1: 规则与周期 (Image #7) ----
                    if modal_tab() == 1 {
                        div { class: "space-y-4 pt-1",
                            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                                span { class: "text-sm text-zinc-200 font-medium", "启用状态" }
                                ToggleSwitch { on: f_enabled(), on_toggle: move |_| f_enabled.set(!f_enabled()) }
                            }
                            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                                span { class: "text-sm text-zinc-200 font-medium", "允许余额兑换" }
                                ToggleSwitch { on: f_allow_redeem(), on_toggle: move |_| f_allow_redeem.set(!f_allow_redeem()) }
                            }
                            div { class: "flex items-center justify-between py-2 border-b border-zinc-800/80",
                                span { class: "text-sm text-zinc-200 font-medium", "额度用尽后允许使用钱包余额" }
                                ToggleSwitch { on: f_allow_wallet(), on_toggle: move |_| f_allow_wallet.set(!f_allow_wallet()) }
                            }

                            // 有效期设置
                            div { class: "pt-2 space-y-2",
                                h4 { class: "text-xs font-semibold text-amber-400 flex items-center gap-1.5", "📅 有效期设置" }
                                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                    label { class: "block space-y-1",
                                        span { class: "text-xs text-zinc-400", "有效期数值" }
                                        input {
                                            r#type: "number",
                                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                            value: "{f_period_val()}",
                                            oninput: move |e| f_period_val.set(e.value()),
                                        }
                                    }
                                    label { class: "block space-y-1",
                                        span { class: "text-xs text-zinc-400", "有效期单位" }
                                        select {
                                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                            value: "{f_period_unit()}",
                                            oninput: move |e| f_period_unit.set(e.value()),
                                            option { value: "小时", "小时" }
                                            option { value: "天", "天" }
                                            option { value: "个月", "个月" }
                                            option { value: "年", "年" }
                                            option { value: "秒", "秒" }
                                        }
                                    }
                                }
                            }

                            // 额度重置
                            div { class: "pt-2 space-y-2",
                                h4 { class: "text-xs font-semibold text-emerald-400 flex items-center gap-1.5", "🔄 额度重置" }
                                div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                                    label { class: "block space-y-1",
                                        span { class: "text-xs text-zinc-400", "重置周期" }
                                        select {
                                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 focus:border-zinc-500 outline-none",
                                            value: "{f_reset_cycle()}",
                                            oninput: move |e| f_reset_cycle.set(e.value()),
                                            option { value: "不重置", "不重置" }
                                            option { value: "每天", "每天" }
                                            option { value: "每周", "每周" }
                                            option { value: "每月", "每月" }
                                            option { value: "自定义", "自定义" }
                                        }
                                    }
                                    label { class: "block space-y-1",
                                        span { class: "text-xs text-zinc-400", "自定义秒数" }
                                        input {
                                            r#type: "number",
                                            disabled: f_reset_cycle() != "自定义",
                                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 disabled:opacity-40 focus:border-zinc-500 outline-none",
                                            value: "0",
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ---- Tab 2: 第三方支付配置 (Image #8) ----
                    if modal_tab() == 2 {
                        div { class: "space-y-4 pt-1",
                            div { class: "rounded-xl border border-amber-500/20 bg-amber-500/5 p-3 text-xs text-amber-300 leading-relaxed",
                                "使用此套餐的标题和价格，在已保存的店铺中创建 Pancake 产品。需要先在支付设置中完整配置 Waffo Pancake。"
                            }
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "Stripe Price ID" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                                    value: "{f_stripe_id()}",
                                    placeholder: "price_1M...",
                                    oninput: move |e| f_stripe_id.set(e.value()),
                                }
                            }
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "Creem Product ID" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                                    value: "{f_creem_id()}",
                                    placeholder: "prod_...",
                                    oninput: move |e| f_creem_id.set(e.value()),
                                }
                            }
                            label { class: "block space-y-1",
                                span { class: "text-xs font-medium text-zinc-300", "Waffo Pancake Product ID" }
                                input {
                                    class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-sm text-zinc-100 font-mono focus:border-zinc-500 outline-none",
                                    value: "{f_waffo_id()}",
                                    placeholder: "选择产品或输入 ID",
                                    oninput: move |e| f_waffo_id.set(e.value()),
                                }
                            }
                        }
                    }

                    // 弹窗底部操作按钮
                    div { class: "flex items-center justify-end gap-3 pt-3 border-t border-zinc-800",
                        button {
                            class: "rounded-xl border border-zinc-700 px-4 py-2 text-xs font-medium text-zinc-400 hover:bg-zinc-800 hover:text-white transition-colors",
                            onclick: move |_| show_modal.set(false),
                            "关闭"
                        }
                        button {
                            class: "rounded-xl bg-amber-400 px-5 py-2 text-xs font-bold text-zinc-950 hover:bg-amber-300 transition-colors shadow-lg shadow-amber-500/10",
                            onclick: commit,
                            "保存更改"
                        }
                    }
                }
            }
        }
    }
}

// ============ 兑换码页 ============

/// 兑换码:3 个面板 = 生成(名称/数量/额度/过期)、列表(搜索/停用/删除)、统计。
#[component]
pub fn RedemptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut reds = store.redemptions;
    let mut name = use_signal(String::new);
    let mut count = use_signal(String::new);
    let mut quota = use_signal(String::new);
    let mut expire_days = use_signal(String::new);
    let search = use_signal(String::new);
    let mut created_flash = use_signal(|| 0u32);

    let generate = move |_| {
        let n = name.peek().trim().to_string();
        if n.is_empty() {
            return;
        }
        let cnt = count
            .peek()
            .trim()
            .parse::<u32>()
            .unwrap_or(1)
            .clamp(1, 100);
        let q = quota.peek().trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let days = expire_days.peek().trim().parse::<i64>().unwrap_or(0);
        let base = reds.read().len() as u32;
        let upper = n.to_uppercase().replace(' ', "-");
        for k in 0..cnt {
            let code = (base.wrapping_add(k).wrapping_mul(2654435761) >> 4) % 65536;
            reds.write().push(RedRow {
                name: n.clone(),
                key: format!("{upper}-{code:04X}"),
                quota: q,
                status: 1,
                created: "2026-09-01".into(),
                expired: if days > 0 {
                    format!("{days} 天后")
                } else {
                    "永不过期".into()
                },
            });
        }
        created_flash.set(cnt);
        name.set(String::new());
        count.set(String::new());
        quota.set(String::new());
        expire_days.set(String::new());
    };

    let q = search().to_lowercase();
    let all = reds.read().clone();
    let total = all.len();
    let used = all.iter().filter(|r| r.status == 3).count();
    let unused = all.iter().filter(|r| r.status == 1).count();
    let visible: Vec<(usize, RedRow)> = all
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            q.is_empty() || r.name.to_lowercase().contains(&q) || r.key.to_lowercase().contains(&q)
        })
        .map(|(i, r)| (i, r.clone()))
        .collect();

    rsx! {
        GridShell {
            Panel {
                title: "生成兑换码",
                hint: "数量 1-100;过期天数空 = 永不过期",
                div { class: "space-y-2",
                    InputCell { label: "名称", value: name, placeholder: "活动码", grow: true }
                    InputCell { label: "数量", value: count, placeholder: "1" }
                    InputCell { label: "每张额度 ¥", value: quota, placeholder: "10" }
                    InputCell { label: "过期天数", value: expire_days, placeholder: "空 = 不过期" }
                    PushBtn { label: "生成", on_click: generate }
                    if created_flash() > 0 {
                        p { class: "text-[11px] text-emerald-400", "已生成 {created_flash()} 张" }
                    }
                }
            }

            Panel {
                title: "兑换码列表",
                hint: "搜索名称或码;停用/启用切换状态",
                InputCell { label: "搜索", value: search, placeholder: "名称 / 码", grow: true }
                if visible.is_empty() {
                    p { class: "text-[11px] text-zinc-600", "没有匹配项" }
                } else {
                    div { class: "space-y-1.5",
                        for (i, r) in visible {
                            {
                                let key = r.key.clone();
                                rsx! {
                                    div { class: "rounded-lg border border-zinc-800 bg-zinc-950/60 p-2",
                                        div { class: "flex items-center justify-between gap-2",
                                            span { class: "truncate font-mono text-xs text-zinc-200", "{key}" }
                                            span { class: "shrink-0 text-[11px] text-zinc-500", "¥{r.quota}" }
                                        }
                                        div { class: "mt-1 flex items-center justify-between gap-2",
                                            span { class: "text-[11px] text-zinc-500",
                                                "{redemption_status_label(r.status)} · {r.expired}"
                                            }
                                            div { class: "flex items-center gap-2",
                                                if r.status != 3 {
                                                    ToggleSwitch {
                                                        on: r.status == 1,
                                                        on_toggle: move |_| {
                                                            let mut w = reds.write();
                                                            w[i].status = if w[i].status == 1 { 2 } else { 1 };
                                                        },
                                                    }
                                                }
                                                button {
                                                    class: "text-[11px] text-red-500 hover:text-red-400",
                                                    onclick: move |_| { reds.write().remove(i); },
                                                    "✕"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Panel {
                title: "统计",
                hint: "对齐 new-api 的列表摘要",
                div { class: "space-y-1.5 text-xs text-zinc-400",
                    p { "总数 {total}" }
                    p { "未用 {unused}" }
                    p { "已用 {used}" }
                }
            }
        }
    }
}

// ============ 系统设置页 ============

/// 系统设置:分区开关 + 文本设置,全部本地 mock 持久(接入时换 kv_store)。
#[component]
pub fn SystemPage() -> Element {
    let store = use_context::<EntityStore>();
    let groups = store.groups;
    let mut toggles = use_signal(|| {
        let mut m = std::collections::HashMap::<&'static str, bool>::new();
        m.insert(KEY_ANNOUNCEMENT, true);
        m.insert(KEY_SIGNIN_BONUS, true);
        m.insert("全局模型配置", true);
        m.insert(KEY_SENSITIVE_WORDS, true);
        m
    });
    let site_name = use_signal(|| "New API".to_string());
    let announcement = use_signal(|| "欢迎使用新 API 控制台".to_string());
    let mut save_flash = use_signal(|| false);
    let mut def_group = use_signal(|| "default".to_string());

    const THEMES: [(&str, &[&str]); 6] = [
        ("站点", &[KEY_ANNOUNCEMENT, "页头导航", "货币与展示"]),
        (
            "认证",
            &["基础认证", "OAuth 集成", "自定义 OAuth", "Passkey"],
        ),
        ("计费", &["支付网关", KEY_SIGNIN_BONUS, "路由单位"]),
        (
            "安全",
            &["机器人防护", "频率限制", KEY_SENSITIVE_WORDS, "SSRF 防护"],
        ),
        ("内容", &[KEY_NOTICE, "FAQ", "绘画", "侧边栏模块"]),
        ("运维", &["日志维护", "监控告警", "性能", "Worker 代理"]),
    ];

    rsx! {
        GridShell {
            Panel {
                title: "分区开关",
                hint: "对齐 new-api 的主题开关;状态仅存本页",
                div { class: "space-y-3",
                    for (theme, items) in THEMES {
                        div { class: "space-y-1.5",
                            p { class: "text-[11px] font-medium text-zinc-400", "{theme}" }
                            for item in items {
                                {
                                    let on = toggles.read().get(item).copied().unwrap_or(false);
                                    rsx! {
                                        div { class: "flex items-center justify-between gap-2",
                                            span { class: "text-xs text-zinc-300", "{item}" }
                                            ToggleSwitch {
                                                on: on,
                                                on_toggle: move |_| {
                                                    let mut m = toggles.write();
                                                    let cur = *m.get(item).unwrap_or(&false);
                                                    m.insert(item, !cur);
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Panel {
                title: "站点信息",
                hint: "示例文本设置;保存只闪提示",
                div { class: "space-y-2",
                    InputCell { label: "站点名", value: site_name, placeholder: "New API", grow: true }
                    InputCell { label: KEY_NOTICE, value: announcement, placeholder: "…", grow: true }
                    PushBtn {
                        label: BTN_SAVE,
                        on_click: move |_| {
                            save_flash.set(true);
                            spawn(async move {
                                gloo_timers::future::TimeoutFuture::new(900).await;
                                save_flash.set(false);
                            });
                        },
                    }
                    if save_flash() {
                        p { class: "text-[11px] text-emerald-400", "已保存(mock)" }
                    }
                }
            }

            Panel {
                title: DEFAULT_GROUP,
                hint: "新用户注册落入的分组(示例)",
                label { class: "block space-y-1",
                    span { class: "text-[11px] text-zinc-500", "{DEFAULT_GROUP}" }
                    select {
                        class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none focus:border-zinc-500",
                        value: "{def_group()}",
                        oninput: move |e| def_group.set(e.value()),
                        for g in groups.read().iter() {
                            option { value: "{g.name}", selected: g.name == def_group(), "{g.name}" }
                        }
                    }
                }
                p { class: "text-[11px] text-zinc-600", "当前:{def_group()}" }
            }
        }
    }
}
/// 渠道导入面板:5/3/1 栏 GridShell 里的第 1 栏。粘贴 URL + Key 自动解析。
#[component]
fn ChannelImportBar() -> Element {
    let mut store = use_context::<EntityStore>();
    let mut url = use_signal(String::new);
    let mut key = use_signal(String::new);
    let mut alias = use_signal(String::new);
    let mut done = use_signal(|| false);
    let mut raw = use_signal(String::new);
    // 解析状态:None=没尝试/空文本,Some(true)=命中,Some(false)=尝试但失败
    let mut parsed_ok = use_signal(|| None::<bool>);

    let can_import = !url.read().trim().is_empty() && !key.read().trim().is_empty();

    rsx! {
        Panel {
            title: "渠道导入",
            hint: "粘贴 URL+Key 自动解析",
            if done() {
                p { class: "rounded-md border border-emerald-800/40 bg-emerald-950/40 px-3 py-1.5 text-[11px] text-emerald-300",
                    "已加入渠道列表,去下方「编辑渠道」补充类型/分组"
                }
            }
            textarea {
                class: "min-h-[60px] w-full resize-y rounded-md border border-dashed border-zinc-800 bg-zinc-950 px-3 py-2 font-mono text-xs text-zinc-200 outline-none placeholder:text-zinc-600 focus:border-zinc-500",
                placeholder: "粘贴 URL + Key(每行一对,或 key=value 形式)",
                oninput: move |e| {
                    // e.value() 返回 owned String,e 的 borrow 在这里就释放;
                    // 后续 set 多个 signal 时不再持有任何 RefCell 借用。
                    let text = e.value();
                    raw.set(text.clone());
                    // ponytail: 同 oninput 内对 url/key/done 多次 set 在 dioxus 0.7
                    // 的 reactive update 中可能嵌套触发。改成仅在 parse 命中时
                    // set url+key+done 三者;text 始终先 set,保证 textarea 自身回显。
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        parsed_ok.set(None);
                    } else if let Some((u, k)) = parse_url_key(&text) {
                        url.set(u);
                        key.set(k);
                        done.set(false);
                        parsed_ok.set(Some(true));
                    } else {
                        parsed_ok.set(Some(false));
                    }
                },
            }
            if parsed_ok() == Some(false) {
                p { class: "text-[11px] text-amber-400", "未识别到 URL + Key,可手动填下面三个字段" }
            }
            div { class: "grid grid-cols-1 gap-2",
                label { class: "block space-y-1",
                    input {
                        class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none focus:border-zinc-500",
                        placeholder: "渠道名(可选)",
                        value: "{alias.read()}",
                        oninput: move |e| alias.set(e.value()),
                    }
                }
                label { class: "block space-y-1",
                    span { class: "text-[11px] text-zinc-500", "Base URL" }
                    input {
                        class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none focus:border-zinc-500",
                        placeholder: "https://…",
                        value: "{url.read()}",
                        oninput: move |e| url.set(e.value()),
                    }
                }
                label { class: "block space-y-1",
                    span { class: "text-[11px] text-zinc-500", "API Key" }
                    input {
                        class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 font-mono text-sm text-zinc-200 outline-none focus:border-zinc-500",
                        placeholder: "sk-…",
                        value: "{key.read()}",
                        oninput: move |e| key.set(e.value()),
                    }
                }
            }
            button {
                class: "w-full rounded-md border border-zinc-100 bg-zinc-100 px-4 py-1.5 text-xs font-medium text-zinc-900 transition-colors hover:bg-zinc-300 disabled:cursor-not-allowed disabled:opacity-50",
                disabled: !can_import,
                onclick: move |_| {
                    let raw_name = alias.peek().trim().to_string();
                    let name = if raw_name.is_empty() { "新渠道".into() } else { raw_name };
                    store.channels.write().push(crate::state::ChannelRow {
                        name,
                        url: url.peek().trim().to_string(),
                        keys: key.peek().trim().to_string(),
                        ctype: "openai".into(),
                        status: 1,
                        group: "default".into(),
                        latency_ms: None,
                        candidates: vec![],
                        dispatch: vec![],
                    });
                    alias.set(String::new());
                    url.set(String::new());
                    key.set(String::new());
                    raw.set(String::new());
                    parsed_ok.set(None);
                    done.set(true);
                },
                "导入渠道"
            }
        }
    }
}

/// 粘贴文本里抽出 (Base URL, API Key)。支持多种形式:
/// - 每行一对:`https://api.openai.com/v1\nsk-xxx`
/// - `|` / 空白 分隔:`https://x | sk-xxx`
/// - `url=https://x\nkey=sk-xxx`(或 base_url / api_key)
/// 仅在能同时拿到 URL 和 Key 时返回 Some,否则 None(让用户继续手动填)。
pub fn parse_url_key(text: &str) -> Option<(String, String)> {
    let mut url = None::<String>;
    let mut key = None::<String>;
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        // key=value 形式
        if let Some((k, v)) = line.split_once('=') {
            let k = k.trim().to_ascii_lowercase();
            let v = v.trim();
            if k == "url" || k == "base_url" || k == "endpoint" || k == "api_base" {
                url = Some(v.to_string());
                continue;
            }
            if k == "key" || k == "api_key" || k == "apikey" || k == "token" {
                key = Some(v.to_string());
                continue;
            }
            continue;
        }
        let parts: Vec<&str> = line.split(|c: char| c.is_whitespace() || c == '|' || c == ',' || c == ';')
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2 {
            let first = parts[0];
            if first.starts_with("http://") || first.starts_with("https://") {
                url.get_or_insert_with(|| first.to_string());
                for p in &parts[1..] {
                    if p.starts_with("sk-") || p.starts_with("rk-") || p.len() >= 20 {
                        key.get_or_insert_with(|| p.to_string());
                        break;
                    }
                }
                continue;
            }
        }
        // 裸 key 行:sk-/rk- 前缀 + 至少 20 字符,降低误匹配短串的概率
        if (line.starts_with("sk-") || line.starts_with("rk-")) && line.len() >= 20 {
            key.get_or_insert_with(|| line.to_string());
            continue;
        }
        // 裸 URL 行
        if line.starts_with("http://") || line.starts_with("https://") {
            url.get_or_insert_with(|| line.to_string());
        }
    }
    match (url, key) {
        (Some(u), Some(k)) => Some((u, k)),
        _ => None,
    }
}
