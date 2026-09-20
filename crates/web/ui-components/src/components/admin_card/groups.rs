use super::card::{AdminCard, short_key};
use contract::api::admin::GroupDto;
use dioxus::prelude::*;

fn parse_whitelist(raw: &serde_json::Value) -> Vec<String> {
    if let Some(arr) = raw.as_array() {
        arr.iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect()
    } else if let Some(s) = raw.as_str() {
        s.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    }
}

fn fmt_ratio(r: f64) -> String {
    format!("×{r:.2}")
}

/// Renders a four-tab prototype card for a [`GroupDto`].
///
/// The tabs follow the group edit modal's fields: 基本信息 (name / status /
/// remark), 倍率与计费 (ratio with a 100-unit example price), 白名单 (model
/// whitelist chips, or "全模型可用" when empty), and 系统 (shortened key,
/// creation and update times). `is_default` labels the default group.
/// This card itself does not perform a write.
#[component]
pub fn GroupCard(
    /// The group DTO displayed by this card.
    group: GroupDto,
    /// Whether this group is the default group.
    is_default: bool,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "倍率与计费", "白名单", "系统"];

    let group_status_str = if group.status == 1 {
        "启用"
    } else {
        "停用"
    }
    .to_string();
    let example_cost = (100.0 * group.ratio).round() as i64;
    let whitelist = parse_whitelist(&group.model_whitelist);
    let short_k = short_key(&group.key);
    let remark = if group.remark.trim().is_empty() {
        "未填写".to_string()
    } else {
        group.remark.clone()
    };

    // 四个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染。
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "名称" }
                span { class: "font-medium text-zinc-200 truncate", "{group.name}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "状态" }
                span { class: "font-medium text-zinc-200", "{group_status_str}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "备注" }
                span { class: "font-medium text-zinc-200", "{remark}" }
            }
        }
    };
    let panel_ratio = rsx! {
        div { class: "space-y-2",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "倍率" }
                span { class: "font-medium text-zinc-200", "{fmt_ratio(group.ratio)}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "示例价格" }
                span { class: "font-medium text-zinc-200", "100 单位 = {example_cost}" }
            }
        }
    };
    let panel_whitelist = rsx! {
        div { class: "space-y-1.5",
            if whitelist.is_empty() {
                span { class: "text-[11px] text-zinc-500", "全模型可用" }
            } else {
                for m in &whitelist {
                    span {
                        class: "inline-flex rounded border border-zinc-700 bg-zinc-800/80 px-2 py-0.5 text-[11px] text-zinc-300 mr-1.5 mb-1",
                        "{m}"
                    }
                }
            }
        }
    };
    let panel_system = rsx! {
        div { class: "space-y-2 text-xs",
            div { class: "flex justify-between gap-2",
                span { class: "text-zinc-400", "Key" }
                span { class: "font-mono text-zinc-200", "{short_k}" }
            }
            div { class: "flex justify-between gap-2",
                span { class: "text-zinc-400", "创建" }
                span { class: "text-zinc-200", "{group.created_at}" }
            }
            div { class: "flex justify-between gap-2",
                span { class: "text-zinc-400", "更新" }
                span { class: "text-zinc-200", "{group.updated_at}" }
            }
        }
    };

    rsx! {
        AdminCard {
            title: "{group.name}",
            subtitle: if is_default { Some("默认分组".to_string()) } else { None },
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("group-card-new".to_string()),
            panel_0: panel_basic,
            panel_1: panel_ratio,
            panel_2: panel_whitelist,
            panel_3: Some(panel_system),
        }
    }
}
