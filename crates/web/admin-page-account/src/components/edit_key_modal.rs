//! 编辑密钥弹窗 — 走 PUT /api/token/{key}。
//! 契约 UpdateTokenRequest 全 Option, 缺省字段不随请求发出 (skip_serializing_if),
//! 后端按「缺省 = 不改」处理 (admin-catalog tokens.rs svc.update 逐字段 if let Some)。
//! 注意: 后端 group / expires_at 是双层 Option (Some(None) = 跟随用户组 / 永不过期),
//! 契约层不表达「清空」语义, 所以这里的留空只能 = 保持不变。

use contract::api::token::{TokenDto, UpdateTokenRequest};
use dioxus::prelude::*;

use crate::api;
use crate::usage_support::{date_input_to_rfc3339, rfc3339_to_date_input};

/// 【是什么】编辑密钥弹窗，允许修改名称、分组、额度、无限额度开关、过期时间。
///
/// 【做什么】负责渲染编辑表单，含名称/分组/额度/无限额度/过期时间五个字段；提交走 PUT /api/token/{key}。
/// 不负责状态切换（启用/停用由 KeyCard 的启用/停用按钮单独处理）。
///
/// 【交互逻辑】
/// - 取消: 点击遮罩或「取消」按钮，调用 on_cancel。
/// - 保存: 校验名称非空、额度为 ≥0 整数；通过后提交 API，成功则 on_saved，失败则显示错误。
/// - 无限额度勾选时：输入框禁用，不解析输入值，直接发 quota=0 + unlimited=true。
/// - 分组/过期时间为可选修改，留空 = 保持不变（不发字段）。
///
/// 【样式】固定居中弹窗 (fixed inset-0 z-50)，白字标题，内边距 p-5，最大宽度 max-w-md；
/// 字段区内部可滚动 (max-h-[60vh] overflow-y-auto)。按钮为原始 button 而非 ui::button 组件。
///
/// 【子组件组成】纯无外部组件依赖，仅使用 dioxus 原生 input/button/label/p。
///
/// 【数据流】
/// - 对内（入）：token (TokenDto) 提供预填充值；on_cancel/on_saved EventHandler 由页面传入。
/// - 对外（出）：保存成功时 on_saved.call(()); 失败仅更新本地 err Signal 显示错误，不回调页面。
#[component]
pub fn EditKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_saved: EventHandler<()>,
) -> Element {
    let mut name = use_signal(|| token.name.clone());
    // 分组 prefill: None (跟随用户组) 显示空串; 编辑语义下空串 = 不发字段 = 保持现状
    let mut group = use_signal(|| token.group.clone().unwrap_or_default());
    let mut unlimited = use_signal(|| token.unlimited_quota);
    let mut quota = use_signal(|| token.quota.to_string());
    // 过期时间 prefill: RFC3339 → UTC 日期段 (与提交方向同口径, 见 usage_support);
    // None (永不过期) 显示空 = 保持不变
    let mut expiry =
        use_signal(|| rfc3339_to_date_input(token.expires_at.as_deref().unwrap_or("")));
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let submit = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("名称不能为空".into());
            return;
        }
        // 分组: 空串 → 不发字段 (保持不变); 非空 → Some(g) 改分组
        let g = group().trim().to_string();
        // 额度: 提交总是发 Some(quota) + Some(unlimitedQuota) (两项一体生效)
        let q_raw = quota().trim().to_string();
        let (quota_v, unlimited_v) = if unlimited() {
            // 无限额度时限额输入禁用: 明确发 0 占位, 不再 parse 输入框旧文本
            // (先输非法值再勾选无限时, 旧输入的 parse 结果无意义);
            // quota 数值此时无意义, 后端以 unlimitedQuota = true 为准
            (0, true)
        } else {
            match q_raw.parse::<i64>() {
                Ok(v) if v >= 0 => (v, false),
                _ => {
                    err.set("额度限制必须是不小于 0 的整数".into());
                    return;
                }
            }
        };
        // 过期时间: 空 → 不发字段 (保持不变); 有值 → UTC RFC3339 (所选日期 → UTC 当天末尾)
        let expires_at = date_input_to_rfc3339(expiry().trim());
        let req = UpdateTokenRequest {
            name: Some(n),
            group: if g.is_empty() { None } else { Some(g) },
            quota: Some(quota_v),
            unlimited_quota: Some(unlimited_v),
            expires_at,
            ..Default::default()
        };
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::update_token_api(&client, &key_id, &req).await {
                Ok(_) => on_saved.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "{ui::MODAL_BACKDROP}",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "{ui::MODAL_CARD}",
                onclick: move |e| e.stop_propagation(),

                div { class: "{ui::MODAL_HEADER}",
                    h3 { class: "{ui::T_text_base} {ui::T_font_semibold} {ui::T_text_zinc_100}", "编辑密钥" }
                    p { class: "truncate font-mono {ui::TYPE_DESC}", "{token.key_preview}" }
                }

                // 字段较多, 弹窗保持 max-w-md 视觉, 字段区超高内部滚动
                div { class: "max-h-[60vh] space-y-4 overflow-y-auto",
                    div {
                        label { class: "mb-1.5 block {ui::T_text_xs} {ui::T_text_zinc_400}", "密钥名称" }
                        input {
                            class: "{ui::INPUT}",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block {ui::T_text_xs} {ui::T_text_zinc_400}", "分组 (可选)" }
                        input {
                            class: "{ui::INPUT}",
                            placeholder: "留空 = 保持不变",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                        p { class: "mt-1 {ui::T_text_11px} {ui::T_text_zinc_500}",
                            "分组决定计费与模型可见范围; 跟随用户默认分组的密钥此处显示为空"
                        }
                    }
                    div {
                        label { class: "mb-1.5 block {ui::T_text_xs} {ui::T_text_zinc_400}", "额度限制 (额度单位)" }
                        label { class: "mb-1.5 flex cursor-pointer items-center gap-2 {ui::T_text_xs} {ui::T_text_zinc_400}",
                            input {
                                r#type: "checkbox",
                                class: "h-4 w-4 accent-emerald-500",
                                checked: "{unlimited}",
                                onchange: move |e| unlimited.set(e.checked()),
                            }
                            "无限额度"
                        }
                        input {
                            class: "w-full rounded-xl border {ui::T_border_zinc_700} {ui::T_bg_zinc_950} px-4 py-2.5 font-mono {ui::T_text_sm} focus:{ui::T_border_zinc_500} focus:outline-none disabled:cursor-not-allowed disabled:opacity-40",
                            r#type: "text",
                            placeholder: "额度单位, 500,000 ≈ $1",
                            value: "{quota}",
                            disabled: unlimited(),
                            oninput: move |e| quota.set(e.value()),
                        }
                        p { class: "mt-1 {ui::T_text_11px} {ui::T_text_zinc_500}", "额度单位: 500,000 ≈ $1" }
                    }
                    div {
                        label { class: "mb-1.5 block {ui::T_text_xs} {ui::T_text_zinc_400}", "过期时间" }
                        input {
                            class: "w-full rounded-xl border {ui::T_border_zinc_700} {ui::T_bg_zinc_950} px-4 py-2.5 {ui::T_text_sm} {ui::T_text_zinc_200} focus:{ui::T_border_zinc_500} focus:outline-none",
                            r#type: "date",
                            value: "{expiry}",
                            oninput: move |e| expiry.set(e.value()),
                        }
                        p { class: "mt-1 {ui::T_text_11px} {ui::T_text_zinc_500}",
                            "留空 = 保持不变; 所选日期当日 (UTC) 结束后失效"
                        }
                    }
                    if !err().is_empty() {
                        p { class: "{ui::T_text_xs} {ui::STATE_DANGER_TEXT}", "{err()}" }
                    }
                }

                div { class: "mt-6 flex gap-3",
                    button {
                        class: "flex-1 rounded-xl border {ui::T_border_zinc_700} py-2.5 {ui::T_text_sm} {ui::T_text_zinc_400} transition-colors hover:{ui::T_bg_zinc_800}",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    button {
                        class: "flex-1 rounded-xl {ui::T_bg_white} py-2.5 {ui::T_text_sm} {ui::T_font_medium} {ui::T_text_zinc_900} transition-colors hover:{ui::T_bg_zinc_200} disabled:opacity-40",
                        disabled: busy(),
                        onclick: submit,
                        "保存"
                    }
                }
            }
        }
    }
}
