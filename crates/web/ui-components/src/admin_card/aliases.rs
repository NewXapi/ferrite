use super::card::{AdminCard, short_key};
use super::editable::{DangerActionRow, EditableRow};
use super::price_mode::{PriceMode, PriceModeToggle};
use dioxus::prelude::*;

fn fmt_price(v: f64) -> String {
    format!("¥{v:.4}")
}

/// 分组倍率胶囊的配色：1.0 灰、低于 1.0 绿（打折）、高于 1.0 琥珀（加价）。
///
/// 这是旧卡的信息设计核心（一眼看出一组定价是便宜还是贵），迁移时逐字保留。
fn ratio_tone(ratio: f64) -> &'static str {
    if (ratio - 1.0).abs() < 0.001 {
        "border-zinc-700 bg-zinc-800/60 text-zinc-300"
    } else if ratio < 1.0 {
        "border-emerald-800/60 bg-emerald-950/40 text-emerald-300"
    } else {
        "border-amber-800/60 bg-amber-950/40 text-amber-300"
    }
}

/// 别名卡可被行内 Popover 编辑的字段。
///
/// 值为用户在浮层里提交的**原始字符串**（未解析）：校验与类型转换在页面层
/// 的写回闭包里做，卡片只负责展示与收集输入。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AliasEditField {
    /// 别名（后端 models 域唯一可落地列，走 PUT）。
    Name,
    /// 展示名（后端无对应列，仅本地状态）。
    Display,
    /// 输入价（CNY / 1k tokens，本地）。
    InputPrice,
    /// 输出价（CNY / 1k tokens，本地）。
    OutputPrice,
    /// 倍率（本地）。
    Multiplier,
}

/// Renders a three-tab card for a model alias.
///
/// 页签按 UI 决策记录 §2.3 收敛为 3 个：基本信息（别名 / 展示名 / 序号 / 标识）、
/// 定价（模式 toggle + 输入 / 输出 / 倍率）、分组（可用分组倍率胶囊）。
///
/// 单字段编辑走行内 Popover（`EditableRow`，决策记录 §2.2 / §3.2），不再经过
/// Modal：点击行 → 浮层输入 → 保存经 `on_edit` 抛回页面（字段 + 原始字符串），
/// 由页面校验并写回。删除是危险操作，保留 Dialog 确认（`DangerActionRow`，
/// 决策记录 §2.4），确认后经 `on_delete` 抛回 key。
///
/// 卡片本身不发起网络请求：数据以值传入，提交以事件抛出；定价模式切换经
/// `on_mode_change` 抛回页面写回 `rows`。序号以 header badge 呈现。
#[component]
pub fn AliasCard(
    /// Alias used as the card title.
    alias: String,
    /// Optional display name; shown as subtitle only when non-empty and
    /// different from `alias`（与旧卡判定一致，避免标题副标题重复）.
    display: String,
    /// Page-supplied input price in CNY per 1k tokens, not a complete persisted
    /// backend pricing record.
    input_per_1k: f64,
    /// Page-supplied output price in CNY per 1k tokens, not a complete
    /// persisted backend pricing record.
    output_per_1k: f64,
    /// Per-call multiplier displayed on the pricing tab.
    multiplier: f64,
    /// Zero-based card position, displayed as a one-based sequence number.
    index: usize,
    /// Caller-computed list of groups usable by this alias: `(group name,
    /// group ratio)`. Empty when no group references the alias.
    usable_groups: Vec<(String, f64)>,
    /// Backend alias key (UUID); displayed truncated on the basic tab.
    alias_key: String,
    /// Per-card pricing mode（按量 / 按次）.
    #[props(default = PriceMode::PerToken)]
    price_mode: PriceMode,
    /// 模式切换回调（抛回页面写回 rows）；未传时不渲染 toggle。
    #[props(default)]
    on_mode_change: Option<EventHandler<PriceMode>>,
    /// 行内 Popover 保存回调：(字段, 原始字符串)。页面负责解析、校验与写回
    /// （Name 走 PUT /api/models/{key}，其余为本地状态）。未传时该行仍渲染，
    /// 但保存无出口 —— 调用方应二选一：要么传回调，要么不渲染编辑入口。
    #[props(default)]
    on_edit: Option<EventHandler<(AliasEditField, String)>>,
    /// 删除确认回调（危险操作，Dialog 确认后触发）；未传时不渲染删除行。
    #[props(default)]
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "定价", "分组"];

    let shown_groups = usable_groups.iter().take(4).collect::<Vec<_>>();
    let overflow_groups = usable_groups.len().saturating_sub(4);
    let short_k = short_key(&alias_key);
    // 卡片标题与内容区各持一份克隆,避免 `alias` 被 move 后仍被借用
    let title_alias = alias.clone();

    // 副标题仅在展示名非空且与别名不同时出现（与旧卡一致）。
    let subtitle = if display.is_empty() || display == alias {
        None
    } else {
        Some(display.clone())
    };

    // —— 行内 Popover 的受控开合 signal（每字段一个，保存后由 EditableRow 置 false）——
    let mut open_name = use_signal(|| false);
    let mut open_display = use_signal(|| false);
    let mut open_input = use_signal(|| false);
    let mut open_output = use_signal(|| false);
    let mut open_mult = use_signal(|| false);

    // 每个编辑行一套「开合 signal + 提交闭包」：保存 = 抛 (字段, 草稿) 给页面 + 收关。
    let commit_name = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((AliasEditField::Name, v));
        }
        open_name.set(false);
    };
    let commit_display = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((AliasEditField::Display, v));
        }
        open_display.set(false);
    };
    let commit_input = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((AliasEditField::InputPrice, v));
        }
        open_input.set(false);
    };
    let commit_output = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((AliasEditField::OutputPrice, v));
        }
        open_output.set(false);
    };
    let commit_mult = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((AliasEditField::Multiplier, v));
        }
        open_mult.set(false);
    };
    let confirm_delete = move |_| {
        if let Some(cb) = on_delete {
            cb.call(alias_key.clone());
        }
    };

    // 三个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            if on_edit.is_some() {
                EditableRow {
                    label: "别名".to_string(),
                    value: title_alias.clone(),
                    open: open_name,
                    testid: "alias-edit-name".to_string(),
                    on_commit: commit_name,
                }
            } else {
                div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                    span { class: "{crate::T_text_zinc_400}", "别名" }
                    span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", "{title_alias}" }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "展示名".to_string(),
                    value: if display.is_empty() { "未填写".to_string() } else { display.clone() },
                    open: open_display,
                    testid: "alias-edit-display".to_string(),
                    placeholder: "展示名（后端无此列，仅本地）".to_string(),
                    on_commit: commit_display,
                }
            } else {
                div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                    span { class: "{crate::T_text_zinc_400}", "展示名" }
                    span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", if display.is_empty() { "未填写" } else { "{display}" } }
                }
            }
            div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                span { class: "{crate::T_text_zinc_400}", "序号" }
                span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", "#{index + 1}" }
            }
            div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                span { class: "{crate::T_text_zinc_400}", "标识" }
                span { class: "font-mono {crate::T_text_zinc_200}", "{short_k}" }
            }
            if on_delete.is_some() {
                DangerActionRow {
                    label: "删除别名".to_string(),
                    confirm_title: "删除别名".to_string(),
                    confirm_detail: format!("删除后调用方将无法再解析 {title_alias}，该操作不可恢复。"),
                    testid: "alias-delete".to_string(),
                    on_confirm: confirm_delete,
                }
            }
        }
    };
    let panel_pricing = rsx! {
        div { class: "space-y-2",
            div { class: "flex items-center justify-between gap-2",
                p { class: "{crate::T_text_11px} {crate::T_font_medium} {crate::T_text_zinc_400}",
                    if price_mode == PriceMode::PerCall { "按次定价" } else { "按量定价" }
                }
                if let Some(cb) = on_mode_change {
                    PriceModeToggle { active: price_mode, on_change: move |m: PriceMode| cb.call(m) }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "输入".to_string(),
                    value: fmt_price(input_per_1k),
                    open: open_input,
                    testid: "alias-edit-input".to_string(),
                    on_commit: commit_input,
                }
            } else {
                div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                    span { class: "{crate::T_text_zinc_400}", "输入" }
                    span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", "{fmt_price(input_per_1k)} / 1k tokens" }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "输出".to_string(),
                    value: fmt_price(output_per_1k),
                    open: open_output,
                    testid: "alias-edit-output".to_string(),
                    on_commit: commit_output,
                }
            } else {
                div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                    span { class: "{crate::T_text_zinc_400}", "输出" }
                    span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", "{fmt_price(output_per_1k)} / 1k tokens" }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "倍率".to_string(),
                    value: format!("×{multiplier}"),
                    open: open_mult,
                    testid: "alias-edit-multiplier".to_string(),
                    on_commit: commit_mult,
                }
            } else {
                div { class: "flex justify-between gap-2 {crate::T_text_xs}",
                    span { class: "{crate::T_text_zinc_400}", "倍率" }
                    span { class: "{crate::T_font_medium} {crate::T_text_zinc_200}", "×{multiplier}" }
                }
            }
        }
    };
    let panel_groups = rsx! {
        div { class: "space-y-1.5",
            p { class: "{crate::T_text_11px} {crate::T_text_zinc_400}", "可用分组" }
            if shown_groups.is_empty() {
                span { class: "{crate::T_text_11px} {crate::T_text_zinc_500}", "无分组引用" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for (gname, gratio) in shown_groups {
                        {
                            let tone = ratio_tone(*gratio);
                            rsx! {
                                span { class: "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 {crate::T_text_11px} {tone}",
                                    "{gname}"
                                    span { class: "{crate::T_text_10px} font-mono opacity-70", "×{gratio:.1}" }
                                }
                            }
                        }
                    }
                    if overflow_groups > 0 {
                        span { class: "rounded-full border {crate::T_border_zinc_700} bg-zinc-800/60 px-2 py-0.5 {crate::T_text_11px} {crate::T_text_zinc_400}",
                            "+{overflow_groups}"
                        }
                    }
                }
            }
        }
    };

    // 序号 badge：与旧卡 header 右侧样式逐字一致。
    let index_badge = rsx! {
        span {
            class: "shrink-0 rounded {crate::T_bg_zinc_800} px-1.5 py-0.5 {crate::T_text_10px} font-mono {crate::T_text_zinc_400} border border-zinc-700/60",
            "#{index + 1}"
        }
    };

    rsx! {
        AdminCard {
            title: "{title_alias}",
            subtitle: subtitle,
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("alias-card-new".to_string()),
            header_action: index_badge,
            panel_0: panel_basic,
            panel_1: panel_pricing,
            panel_2: panel_groups,
        }
    }
}
