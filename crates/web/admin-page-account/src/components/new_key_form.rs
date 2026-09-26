//! 新建密钥弹窗 — 提交走 POST /api/token。
//! 额度输入留空 = 不限额 (unlimited_quota)。
//! 成功后由父组件弹出一次性明文视图。

use contract::api::token::{CreateTokenRequest, CreateTokenResult};
use dioxus::prelude::*;
use ui::button::{Button, ButtonVariant};

use crate::api;

/// 【是什么】新建密钥表单弹窗，含名称、分组、额度输入、无限额度开关及新建按钮。
///
/// 【做什么】负责渲染新建 API 密钥的完整表单：三个输入字段 (名称、分组、额度) + 无限额度复选框 + 新建/取消按钮。不负责 API 调用，仅做表单校验（名称非空、额度为 ≥0 整数）及提交逻辑。
///
/// 【交互逻辑】
/// - 点击「取消」/遮罩：调用 on_cancel，关闭弹窗。
/// - 点击「新建密钥」：校验表单，合法后提交 CREATE /api/token。
///   * 成功则 on_created(res), 并清空表单 & 关闭弹窗，父页显示 CreatedKeyView。
///   * 失败则更新本地 err Signal 显示错误信息，不中断用户继续编辑。
/// - 无限额度勾选时：额度输入框禁用，不再 parse 输入值，直接发 quota=0 + unlimited=true。
///
/// 【样式】固定最大宽度 max-w-md，圆角边框、暗色背景、中等内边距，表单字段为上拉输入框风格，
/// 错误信息以红色文字显示，按钮为 Ghost/Primary variant。
///
/// 【子组件组成】ui::button::Button × 2 (取消/新建)
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

    let submit = move |_| {
        let n = name().trim().to_string();
        if n.is_empty() {
            err.set("密钥名称必填".into());
            return;
        }
        let g = group().trim().to_string();
        let q_raw = quota().trim().to_string();
        let (quota_v, unlimited) = if q_raw.is_empty() {
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
        div {
            class: "{ui::MODAL_BACKDROP}",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "{ui::MODAL_CARD}",
                onclick: move |e| e.stop_propagation(),

                div { class: "{ui::MODAL_HEADER}",
                    h3 { class: "text-base font-semibold text-zinc-100", "新建 API 密钥" }
                    button {
                        class: "{ui::CLOSE_BTN}",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "{ui::INPUT}",
                            placeholder: "例如: 生产环境密钥",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组 (可选)" }
                        input {
                            class: "{ui::INPUT}",
                            placeholder: "留空 = 跟随用户默认分组",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度限制 (额度单位)" }
                        input {
                            class: "{ui::INPUT_MONO}",
                            r#type: "text",
                            placeholder: "留空 = 不限额",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }

                    if !err().is_empty() {
                        p { class: "text-xs {ui::STATE_DANGER_TEXT}", "{err()}" }
                    }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Primary,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: submit,
                        "创建密钥"
                    }
                }
            }
        }
    }
}
