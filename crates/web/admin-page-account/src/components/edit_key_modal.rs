//! 编辑密钥弹窗 — 走 PUT /api/token/{key}（rust-ui Dialog 重构）。
//! 契约 UpdateTokenRequest 全 Option, 缺省字段不随请求发出 (skip_serializing_if),
//! 后端按「缺省 = 不改」处理 (admin-catalog tokens.rs svc.update 逐字段 if let Some)。
//! 注意: 后端 group / expires_at 是双层 Option (Some(None) = 跟随用户组 / 永不过期),
//! 契约层不表达「清空」语义, 所以这里的留空只能 = 保持不变。

use contract::api::token::{TokenDto, UpdateTokenRequest};
use dioxus::prelude::*;

use ui::components::rui_alert::{Alert, AlertDescription, AlertVariant};
use ui::components::rui_button::{Button, ButtonSize, ButtonVariant};
use ui::components::rui_dialog::{
    Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};
use ui::components::rui_input::{Input, InputType};
use ui::components::rui_label::Label;
use ui::components::rui_switch::{Switch, SwitchLabel};

use crate::api;
use crate::usage_support::{date_input_to_rfc3339, rfc3339_to_date_input};

/// 【是什么】编辑密钥弹窗，允许修改名称、分组、额度、无限额度开关、过期时间。
///
/// 【做什么】负责渲染编辑表单，含名称/分组/额度/无限额度/过期时间五个字段；提交走 PUT /api/token/{key}。
/// 不负责状态切换（启用/停用由 KeyCard 的启用/停用按钮单独处理）。
///
/// 【交互逻辑】
/// - 取消: 点击遮罩、「取消」按钮或 X，调用 on_cancel（弹窗条件挂载，父层卸载）。
/// - 保存: 校验名称非空、额度为 ≥0 整数；通过后提交 API，成功则 on_saved，失败则以 Destructive Alert 显示错误。
/// - 无限额度开关打开时：输入框禁用，不解析输入值，直接发 quota=0 + unlimited=true。
/// - 分组/过期时间为可选修改，留空 = 保持不变（不发字段）。
///
/// 【样式】rust-ui Dialog 承载（max-w-md，字段区超高内部滚动）；Input + Label；无限额度 Switch；错误 Destructive Alert；按钮 Outline/Default。
///
/// 【子组件组成】rui Dialog 族 + Input ×4 + Label + Switch + Alert + Button ×2
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

    let submit = move |_: MouseEvent| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("名称不能为空".into());
            return;
        }
        // 分组: 空串 → 不发字段 (保持不变); 非空 → Some(g) 改分组
        let g = group().trim().to_string();
        // 额度: 提交总是发 Some(quota) + Some(unlimitedQuota) (两项一体生效);
        // 无限额度时明确发 0 占位, 不再 parse 输入框旧文本
        let q_raw = quota().trim().to_string();
        let (quota_v, unlimited_v) = if unlimited() {
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
        Dialog { class: "w-full max-w-md",
            DialogContent { open: true, on_close: move |_| on_cancel.call(()),
                DialogHeader {
                    DialogTitle { "编辑密钥" }
                    DialogDescription { class: "truncate font-mono", "{token.key_preview}" }
                }

                // 字段较多, 弹窗保持 max-w-md 视觉, 字段区超高内部滚动
                div { class: "max-h-[60vh] space-y-4 overflow-y-auto",
                    div { class: "space-y-1",
                        Label { "密钥名称" }
                        Input {
                            value: "{name}",
                            oninput: move |e: FormEvent| name.set(e.value()),
                        }
                    }
                    div { class: "space-y-1",
                        Label { "分组 (可选)" }
                        Input {
                            placeholder: "留空 = 保持不变",
                            value: "{group}",
                            oninput: move |e: FormEvent| group.set(e.value()),
                        }
                        p { class: "text-[11px] text-zinc-500",
                            "分组决定计费与模型可见范围; 跟随用户默认分组的密钥此处显示为空"
                        }
                    }
                    div { class: "space-y-1",
                        Label { "额度限制 (额度单位)" }
                        Input {
                            placeholder: "额度单位, 500,000 ≈ $1",
                            class: "font-mono",
                            value: "{quota}",
                            disabled: unlimited(),
                            oninput: move |e: FormEvent| quota.set(e.value()),
                        }
                        div { class: "flex items-center gap-2",
                            Switch {
                                checked: unlimited(),
                                aria_label: "无限额度",
                                on_change: Some(EventHandler::new(move |v: bool| unlimited.set(v))),
                            }
                            SwitchLabel { class: "text-xs text-zinc-400", "无限额度" }
                        }
                        p { class: "text-[11px] text-zinc-500", "额度单位: 500,000 ≈ $1" }
                    }
                    div { class: "space-y-1",
                        Label { "过期时间" }
                        Input {
                            r#type: InputType::Date,
                            value: "{expiry}",
                            oninput: move |e: FormEvent| expiry.set(e.value()),
                        }
                        p { class: "text-[11px] text-zinc-500",
                            "留空 = 保持不变; 所选日期当日 (UTC) 结束后失效"
                        }
                    }
                    if !err().is_empty() {
                        Alert { variant: AlertVariant::Destructive,
                            AlertDescription { "{err()}" }
                        }
                    }
                }

                DialogFooter {
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        size: ButtonSize::Sm,
                        disabled: busy(),
                        onclick: submit,
                        "保存"
                    }
                }
            }
        }
    }
}
