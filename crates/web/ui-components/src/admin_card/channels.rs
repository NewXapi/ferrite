use super::card::{AdminCard, short_key};
use super::editable::{DangerActionRow, EditableRow};
use contract::api::admin::ChannelDto;
use dioxus::prelude::*;

/// Masks a key by Unicode scalar value without splitting UTF-8 characters.
fn mask_key(key: &str) -> String {
    const EDGE_CHARS: usize = 4;

    let char_count = key.chars().count();
    if char_count <= EDGE_CHARS * 2 {
        return key.to_string();
    }

    let head: String = key.chars().take(EDGE_CHARS).collect();
    let tail: String = key.chars().skip(char_count - EDGE_CHARS).collect();
    format!("{head}…{tail}")
}

/// 渠道卡可被行内 Popover 编辑的字段。
///
/// 后端 PUT `/api/channel/{key}` 的最小 diff 体（`UpdateChannelBody`）只覆盖
/// name / base_url / remark / test_model 四个可编辑列；priority / weight / status /
/// keys / models 不在其列（COALESCE 保持或走专用端点），因此卡上对它们只读。
/// 值为用户提交的**原始字符串**，校验与请求体构造在页面层完成。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChannelEditField {
    /// 渠道名称。
    Name,
    /// API 基址。
    BaseUrl,
    /// 备注（空串 = 显式清空）。
    Remark,
    /// 测速模型（空 = 置 NULL）。
    TestModel,
}

/// Renders a three-tab card for a [`ChannelDto`].
///
/// 页签按 UI 决策记录 §2.3 收敛为 3 个：基本信息（名称 / 基址 / 状态 / 类型）、
/// 密钥与模型（只读：密钥掩码与调度模型）、调度（优先级 / 权重 / 分组 / 测速模型 /
/// 备注 / 删除入口）。
///
/// 单字段编辑走行内 Popover（`EditableRow`，§2.2），保存经 `on_edit` 抛回页面，
/// 由页面构造 `UpdateChannelBody` 最小 diff PUT。状态启停是开关类操作，保留为
/// 行内切换按钮（旧卡同款交互，`on_toggle`）。密钥 / 模型 / 分组等多字段能力只在
/// 弹窗，卡上以「完整编辑」行入口（`on_full_edit`）跳转，不塞进 Popover。
/// 删除是危险操作，Dialog 确认后经 `on_delete` 抛回 key（§2.4）。
#[component]
pub fn ChannelCard(
    /// The channel DTO displayed by this card.
    channel: ChannelDto,
    /// 行内 Popover 保存回调：(字段, 原始字符串)。页面负责构造最小 diff 请求体。
    #[props(default)]
    on_edit: Option<EventHandler<(ChannelEditField, String)>>,
    /// 「完整编辑」行回调（开弹窗：密钥 / 模型 / 分组）；未传时不渲染该行。
    #[props(default)]
    on_full_edit: Option<EventHandler<()>>,
    /// 启停切换回调；未传时不渲染切换按钮。
    #[props(default)]
    on_toggle: Option<EventHandler<()>>,
    /// 删除确认回调（Dialog 确认后触发）；未传时不渲染删除行。
    #[props(default)]
    on_delete: Option<EventHandler<String>>,
) -> Element {
    let mut tab = use_signal(|| 0usize);
    let tabs = vec!["基本信息", "密钥与模型", "调度"];

    let channel_status_str = if channel.status == 1 {
        "启用"
    } else {
        "停用"
    }
    .to_string();
    let status_tone = if channel.status == 1 {
        "border-emerald-500/30 bg-emerald-500/20 text-emerald-400"
    } else {
        "border-zinc-700 bg-zinc-800/80 text-zinc-400"
    };
    let test_model = channel
        .test_model
        .as_deref()
        .filter(|model| !model.trim().is_empty())
        .unwrap_or("未配置")
        .to_string();
    let remark = if channel.remark.trim().is_empty() {
        "未填写".to_string()
    } else {
        channel.remark.clone()
    };
    let short_k = short_key(&channel.key);

    let dispatch_models: Vec<String> = channel
        .models
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| {
                    m.as_str().map(|s| s.to_string()).or_else(|| {
                        m.get("alias")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let masked_keys: Vec<String> = channel
        .keys
        .as_ref()
        .map(|v| v.iter().map(|k| mask_key(k)).collect())
        .unwrap_or_default();

    // —— 行内 Popover 的受控开合 signal（每字段一个）——
    let mut open_name = use_signal(|| false);
    let mut open_url = use_signal(|| false);
    let mut open_test_model = use_signal(|| false);
    let mut open_remark = use_signal(|| false);

    let commit_name = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((ChannelEditField::Name, v));
        }
        open_name.set(false);
    };
    let commit_url = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((ChannelEditField::BaseUrl, v));
        }
        open_url.set(false);
    };
    let commit_test_model = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((ChannelEditField::TestModel, v));
        }
        open_test_model.set(false);
    };
    let commit_remark = move |v: String| {
        if let Some(cb) = on_edit {
            cb.call((ChannelEditField::Remark, v));
        }
        open_remark.set(false);
    };
    let confirm_delete = move |_| {
        if let Some(cb) = on_delete {
            cb.call(channel.key.clone());
        }
    };

    // 三个页签内容各自为独立 Element，传给 AdminCard 同格叠加渲染。
    let groups_joined = channel.groups.join(", ");
    let panel_basic = rsx! {
        div { class: "space-y-2.5",
            if on_edit.is_some() {
                EditableRow {
                    label: "名称".to_string(),
                    value: channel.name.clone(),
                    open: open_name,
                    testid: "channel-edit-name".to_string(),
                    placeholder: "渠道名称".to_string(),
                    on_commit: commit_name,
                }
            } else {
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "名称" }
                    span { class: "font-medium text-zinc-200", "{channel.name}" }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "基址".to_string(),
                    value: channel.base_url.clone(),
                    open: open_url,
                    testid: "channel-edit-base-url".to_string(),
                    placeholder: "https://api.openai.com/v1".to_string(),
                    on_commit: commit_url,
                }
            } else {
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "基址" }
                    span { class: "font-medium text-zinc-200 break-all", "{channel.base_url}" }
                }
            }
            // 状态行：开关类操作，保留旧卡「行内一点即切换」交互（不走 Popover）
            div { class: "flex items-center justify-between gap-2 rounded-lg px-2 py-1.5 text-xs",
                span { class: "text-zinc-400", "状态" }
                span { class: "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 {status_tone}",
                    "{channel_status_str}"
                }
                if let Some(cb) = on_toggle {
                    button {
                        class: "rounded-lg border border-zinc-700 px-2 py-0.5 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800",
                        "data-testid": "channel-toggle-status",
                        onclick: move |_| cb.call(()),
                        if channel.status == 1 { "停用" } else { "启用" }
                    }
                }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "类型" }
                span { class: "font-medium text-zinc-200", "{channel.channel_type}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "标识" }
                span { class: "font-mono text-zinc-200", "{short_k}" }
            }
            // 完整编辑入口：密钥 / 模型 / 分组等弹窗专属能力（§2.4：多字段表单走 Modal）
            if let Some(cb) = on_full_edit {
                button {
                    class: "mt-1 w-full rounded-lg border border-zinc-700 px-2 py-1.5 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800",
                    "data-testid": "channel-full-edit",
                    onclick: move |_| cb.call(()),
                    "完整编辑（密钥 / 模型 / 分组）"
                }
            }
        }
    };
    let panel_keys = rsx! {
        div { class: "space-y-2",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "密钥" }
                span { class: "font-medium text-zinc-200", "{channel.key_count} 个" }
            }
            if masked_keys.is_empty() {
                span { class: "text-[11px] text-zinc-500", "列表不返回密钥明文，点「完整编辑」管理" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for k in masked_keys {
                        span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 font-mono text-[11px] text-zinc-400",
                            "{k}"
                        }
                    }
                }
            }
            p { class: "pt-1 text-[11px] text-zinc-400", "调度模型" }
            if dispatch_models.is_empty() {
                span { class: "text-[11px] text-zinc-500", "未配置" }
            } else {
                div { class: "flex flex-wrap gap-1.5",
                    for m in dispatch_models.iter().take(6) {
                        span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-[11px] text-zinc-300",
                            "{m}"
                        }
                    }
                    if dispatch_models.len() > 6 {
                        span { class: "rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-[11px] text-zinc-400",
                            "+{dispatch_models.len() - 6}"
                        }
                    }
                }
            }
        }
    };
    let panel_dispatch = rsx! {
        div { class: "space-y-2",
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "优先级" }
                span { class: "font-medium text-zinc-200", "{channel.priority}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "权重" }
                span { class: "font-medium text-zinc-200", "{channel.weight}" }
            }
            div { class: "flex justify-between gap-2 text-xs",
                span { class: "text-zinc-400", "绑定分组" }
                span { class: "font-medium text-zinc-200", "{groups_joined}" }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "测速模型".to_string(),
                    value: test_model.clone(),
                    open: open_test_model,
                    testid: "channel-edit-test-model".to_string(),
                    placeholder: "留空 = 清除".to_string(),
                    on_commit: commit_test_model,
                }
            } else {
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "测速模型" }
                    span { class: "font-medium text-zinc-200", "{test_model}" }
                }
            }
            if on_edit.is_some() {
                EditableRow {
                    label: "备注".to_string(),
                    value: remark.clone(),
                    open: open_remark,
                    testid: "channel-edit-remark".to_string(),
                    placeholder: "渠道用途说明".to_string(),
                    on_commit: commit_remark,
                }
            } else {
                div { class: "flex justify-between gap-2 text-xs",
                    span { class: "text-zinc-400", "备注" }
                    span { class: "font-medium text-zinc-200", "{remark}" }
                }
            }
            if on_delete.is_some() {
                DangerActionRow {
                    label: "删除渠道".to_string(),
                    confirm_title: "删除渠道".to_string(),
                    confirm_detail: format!("删除后 {channel_name} 及其密钥、调度配置将不可恢复。", channel_name = channel.name),
                    testid: "channel-delete".to_string(),
                    on_confirm: confirm_delete,
                }
            }
        }
    };

    rsx! {
        AdminCard {
            title: "{channel.name}",
            subtitle: channel.channel_type.clone(),
            tabs: tabs,
            active_tab: tab(),
            on_tab_change: move |t| tab.set(t),
            testid: Some("channel-card-new".to_string()),
            panel_0: panel_basic,
            panel_1: panel_keys,
            panel_2: panel_dispatch,
        }
    }
}
