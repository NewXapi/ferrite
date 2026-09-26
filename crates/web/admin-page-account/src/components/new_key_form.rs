//! 新建密钥弹窗 — 提交走 POST /api/token（rust-ui Dialog 重构）。
//! 额度输入留空 = 不限额 (unlimited_quota)；「无限额度」开关勾选时禁用额度输入。
//! 成功后由父组件弹出一次性明文视图。

use contract::api::token::{CreateTokenRequest, CreateTokenResult};
use dioxus::prelude::*;

use ui::components::rui_alert::{Alert, AlertDescription, AlertVariant};
use ui::components::rui_button::{Button, ButtonSize, ButtonVariant};
use ui::components::rui_dialog::{
    Dialog, DialogBody, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle,
};
use ui::components::rui_input::{Input, InputType};
use ui::components::rui_label::Label;
use ui::components::rui_switch::{Switch, SwitchLabel};

use crate::api;

/// 【是什么】新建密钥表单弹窗，含名称、分组、额度输入、无限额度开关及新建按钮。
///
/// 【做什么】负责渲染新建 API 密钥的完整表单：三个输入字段 (名称、分组、额度) + 无限额度 Switch + 创建/取消按钮。不负责 API 调用，仅做表单校验（名称非空、额度为 ≥0 整数）及提交逻辑。
///
/// 【交互逻辑】
/// - 点击「取消」/遮罩/X：调用 on_cancel，关闭弹窗（弹窗条件挂载，父层卸载）。
/// - 点击「创建密钥」：校验表单，合法后提交 CREATE /api/token。
///   * 成功则 on_created(res), 并清空表单 & 关闭弹窗，父页显示 CreatedKeyView。
///   * 失败则更新本地 err Signal 以 Destructive Alert 显示，不中断用户继续编辑。
/// - 无限额度开关打开时：额度输入框禁用，不再 parse 输入值，直接发 quota=0 + unlimited=true。
///
/// 【样式】rust-ui Dialog 承载（max-w-md）；字段 Input + Label 上标签；错误 Destructive Alert；按钮 Outline/Default。
///
/// 【子组件组成】rui Dialog 族 + Input ×3 + Label + Switch + Alert + Button ×2
///
/// 【数据流】
/// - 对内（入）：三个 Signal (name/group/quota) 由父页面持有名，传值到组件内双向绑定；on_cancel/on_created EventHandler 由页面传入。
/// - 对外（出）：成功时 on_created.call(res) 透传 CreateTokenResult 给页面；失败时仅更新 err 状态，页面无需感知。
#[component]
pub fn NewKeyForm(
    name: Signal<String>,
    group: Signal<String>,
    quota: Signal<String>,
    on_cancel: EventHandler<()>,
    on_created: EventHandler<CreateTokenResult>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);
    let mut unlimited = use_signal(|| false);

    let submit = move |_: MouseEvent| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("密钥名称必填".into());
            return;
        }
        let g = group().trim().to_string();
        let q_raw = quota().trim().to_string();
        // 无限额度开关或留空额度 → (0, true); 开关打开时输入框已禁用, 这里统一兜底
        let (quota_v, unlimited) = if unlimited() || q_raw.is_empty() {
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
        let req = CreateTokenRequest {
            name: n,
            group: if g.is_empty() { None } else { Some(g) },
            quota: quota_v,
            unlimited_quota: unlimited,
            // 新建暂不设置过期 (MVP); 需要时由 CreateTokenRequest.expires_at 传入 RFC3339。
            expires_at: None,
        };

        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::create_token_api(&client, &req).await {
                Ok(res) => on_created.call(res),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        Dialog {
            DialogContent { open: true, on_close: move |_| on_cancel.call(()),
                DialogHeader {
                    DialogTitle { "新建 API 密钥" }
                    DialogDescription { "创建后明文只显示一次, 请立即保存" }
                }

                DialogBody {
                    div { class: "space-y-1",
                        Label { "密钥名称" }
                        Input {
                            placeholder: "例如: 生产环境密钥",
                            value: "{name}",
                            oninput: move |e: FormEvent| name.set(e.value()),
                        }
                    }
                    div { class: "space-y-1",
                        Label { "分组 (可选)" }
                        Input {
                            placeholder: "留空 = 跟随用户默认分组",
                            value: "{group}",
                            oninput: move |e: FormEvent| group.set(e.value()),
                        }
                    }
                    div { class: "space-y-1",
                        Label { "额度限制 (额度单位)" }
                        Input {
                            r#type: InputType::Text,
                            placeholder: "留空 = 不限额",
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
                        "创建密钥"
                    }
                }
            }
        }
    }
}
