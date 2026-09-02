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

/// 1/3/5 栏响应式网格(手机 1 / 平板 3 / 桌面 5)。
#[component]
pub fn GridShell(children: Element) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 xl:grid-cols-5", {children} }
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

/// 订阅周期文字。
fn plan_period_label(period: &str) -> &'static str {
    match period {
        "quarter" => "季度",
        "year" => "年度",
        _ => "月度",
    }
}

// ============ 渠道页 ============

/// 渠道管理:4 个面板 = 状态速览(启用/测速/停启/删除)、编辑渠道、模型调度、批量绑定。
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
            // 面板 1:状态速览(new-api 列表行的状态/测速/操作)
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

/// 订阅设置:3 个面板 = 套餐列表(启停/删除)、新增/编辑套餐、用户订阅占位。
#[component]
pub fn SubscriptionsPage() -> Element {
    let store = use_context::<EntityStore>();
    let mut plans = store.plans;
    let groups = store.groups;
    let mut title = use_signal(String::new);
    let mut subtitle = use_signal(String::new);
    let mut price = use_signal(String::new);
    let mut quota = use_signal(String::new);
    let mut period = use_signal(|| String::from("month"));
    let mut group = use_signal(|| String::from("default"));
    let mut max_per_user = use_signal(String::new);
    let mut editing = use_signal(|| None::<usize>);

    let commit = move |_| {
        let t = title.peek().trim().to_string();
        if t.is_empty() {
            return;
        }
        let parse_pos = |s: &str| s.trim().parse::<f64>().unwrap_or(0.0).max(0.0);
        let max_u = max_per_user.peek().trim().parse::<u32>().unwrap_or(0);
        let row = PlanRow {
            title: t,
            subtitle: subtitle.peek().trim().to_string(),
            price: parse_pos(&price.peek()),
            period: period.peek().clone(),
            quota: parse_pos(&quota.peek()),
            group: group.peek().clone(),
            enabled: true,
            max_per_user: max_u,
        };
        match *editing.peek() {
            Some(i) => {
                plans.write()[i] = row;
            }
            None => plans.write().push(row),
        }
        title.set(String::new());
        subtitle.set(String::new());
        price.set(String::new());
        quota.set(String::new());
        max_per_user.set(String::new());
        editing.set(None);
    };

    rsx! {
        GridShell {
            Panel {
                title: "套餐列表",
                hint: "开关=是否在售;点击行载入编辑",
                if plans.read().is_empty() {
                    p { class: "text-[11px] text-zinc-600", "还没有套餐" }
                } else {
                    div { class: "space-y-1.5",
                        for (i, p) in plans.read().iter().enumerate() {
                            {
                                let title_txt = p.title.clone();
                                let sub = if p.subtitle.is_empty() { plan_period_label(&p.period).to_string() } else { format!("{} · {}", p.subtitle, plan_period_label(&p.period)) };
                                let tone = if editing() == Some(i) { "border-zinc-600 bg-zinc-900" } else { "border-zinc-800 bg-zinc-950/60 hover:border-zinc-700" };
                                rsx! {
                                    div { class: "rounded-lg border p-2 {tone}",
                                        button {
                                            class: "flex w-full items-baseline justify-between gap-2 text-left",
                                            onclick: move |_| {
                                                let p = plans.read()[i].clone();
                                                title.set(p.title);
                                                subtitle.set(p.subtitle);
                                                price.set(format!("{}", p.price));
                                                quota.set(format!("{}", p.quota));
                                                period.set(p.period);
                                                group.set(p.group);
                                                max_per_user.set(format!("{}", p.max_per_user));
                                                editing.set(Some(i));
                                            },
                                            span { class: "truncate text-xs text-zinc-200", "{title_txt}" }
                                            span { class: "shrink-0 text-[11px] text-zinc-500", "{sub}" }
                                        }
                                        div { class: "mt-1 flex items-center justify-between gap-2",
                                            span { class: "text-[11px] text-zinc-500", "¥{p.price} · 额度 ¥{p.quota} · {p.group}" }
                                            div { class: "flex items-center gap-2",
                                                ToggleSwitch {
                                                    on: p.enabled,
                                                    on_toggle: move |_| {
                                                        let mut w = plans.write();
                                                        w[i].enabled = !w[i].enabled;
                                                    },
                                                }
                                                button {
                                                    class: "text-[11px] text-red-500 hover:text-red-400",
                                                    onclick: move |_| {
                                                        plans.write().remove(i);
                                                        if editing() == Some(i) { editing.set(None); }
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

            Panel {
                title: "新增 / 编辑套餐",
                hint: "标题必填;数字空 = 0(不限量/免费)",
                div { class: "space-y-2",
                    InputCell { label: "标题", value: title, placeholder: "进阶版", grow: true }
                    InputCell { label: "副标题", value: subtitle, placeholder: "Claude 主力(可选)", grow: true }
                    InputCell { label: "售价 ¥", value: price, placeholder: "128" }
                    InputCell { label: "内含额度 ¥", value: quota, placeholder: "100" }
                    SelectCell {
                        label: "周期",
                        value: period(),
                        options: crate::state::PLAN_PERIODS,
                        oninput: move |v: String| period.set(v),
                    }
                    label { class: "block space-y-1",
                        span { class: "text-[11px] text-zinc-500", "生效分组" }
                        select {
                            class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none focus:border-zinc-500",
                            value: "{group()}",
                            oninput: move |e| group.set(e.value()),
                            for g in groups.read().iter() {
                                option { value: "{g.name}", selected: g.name == group(), "{g.name}" }
                            }
                        }
                    }
                    InputCell { label: "每人限购(0=不限)", value: max_per_user, placeholder: "0" }
                    div { class: "flex gap-2 pt-1",
                        PushBtn { label: BTN_SAVE, on_click: commit }
                        if editing().is_some() {
                            GhostBtn {
                                label: BTN_CANCEL,
                                on_click: move |_| {
                                    editing.set(None);
                                    title.set(String::new());
                                    subtitle.set(String::new());
                                    price.set(String::new());
                                    quota.set(String::new());
                                    max_per_user.set(String::new());
                                },
                            }
                        }
                    }
                }
            }

            Panel {
                title: "用户订阅",
                hint: "占位:接入 API 后列 user_subscriptions",
                p { class: "text-[11px] text-zinc-600", "暂无订阅记录" }
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
