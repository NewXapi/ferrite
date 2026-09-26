//! 资料与密码区 — PUT /api/user/self 改显示名 / 改密码
//! (改密须原密码 + 新密码成对提供)。

use contract::api::user::UpdateSelfRequest;
use dioxus::prelude::*;
use ui::button::{Button, ButtonVariant};

use crate::api;

/// 【是什么】资料与密码区段组件，修改显示名与登录密码 (同一端点 PUT /api/user/self)。
///
/// 【做什么】挂载时拉取当前显示名预填；渲染显示名输入 + 保存按钮、原密码/新密码两输入 + 修改密码按钮，成功/失败以绿字 flash 与红字错误反馈；改显示名成功后同步 ferrite_current_user 本地缓存，顶栏头像菜单立即读到新名。
///
/// 【交互逻辑】保存显示名前先 trim 并拒绝空值；改密两框都空则不发请求，只填一框则提示须成对提供；两个动作共用 saving 信号串行防重入，成功后清空密码框。
///
/// 【样式】卡片 rounded-xl bg-zinc-900/60 p-6 + hover:border-zinc-600；输入 rounded-xl 边框 + focus:border-zinc-500，密码框等宽字体；按钮 Primary 置于输入右侧 (items-end 对齐)。
///
/// 【子组件组成】ui::button::Button (保存显示名 / 修改密码) × 2
///
/// 【数据流】无 props：显示名与密码草稿、保存状态全为本组件私有，经 api 取用。
#[component]
pub fn AccountSection() -> Element {
    // —— 资料与密码 (PUT /api/user/self) ——
    let mut display_name = use_signal(String::new);
    let mut original_password = use_signal(String::new);
    let mut new_password = use_signal(String::new);
    // 保存动作状态 (显示名 / 密码共用: 同一端点, 串行防重入)
    let mut saving = use_signal(|| false);
    let mut save_err = use_signal(String::new);
    let mut save_flash = use_signal(|| None::<String>);

    use_hook(move || {
        // 预填当前显示名 (GET /api/user/self)
        let client = client::ApiClient::shared().clone();
        let mut dn = display_name;
        spawn(async move {
            if let Ok(u) = api::get_self_api(&client).await {
                dn.set(u.display_name);
            }
        });
    });

    rsx! {
        div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6 transition-colors hover:border-zinc-600",
            role: "group",
            "aria-label": "资料与密码",
            "data-testid": "settings-account-card",
            div { class: "mb-3 flex items-center justify-between",
                h3 { class: "{ui::TYPE_CARD_TITLE}", "资料与密码" }
                span { class: "{ui::TYPE_DESC}", "改密须原密码 + 新密码成对提供" }
            }

            div { class: "grid grid-cols-1 gap-4 sm:grid-cols-2",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "显示名" }
                    input {
                        class: "{ui::INPUT}",
                        "data-testid": "settings-display-name",
                        placeholder: "对外展示的名称",
                        value: "{display_name}",
                        oninput: move |e| display_name.set(e.value()),
                    }
                }
                div { class: "flex items-end",
                    Button {
                        variant: ButtonVariant::Primary,
                        disabled: saving(),
                        "data-testid": "save-display-name",
                        "aria-label": "保存显示名",
                        onclick: move |_| {
                            if saving() {
                                return;
                            }
                            let name = display_name().trim().to_string();
                            if name.is_empty() {
                                save_err.set("显示名不能为空".into());
                                save_flash.set(None);
                                return;
                            }
                            saving.set(true);
                            save_err.set(String::new());
                            save_flash.set(None);
                            let client = client::ApiClient::shared().clone();
                            let req = UpdateSelfRequest {
                                display_name: Some(name),
                                original_password: None,
                                new_password: None,
                            };
                            let mut f = save_flash;
                            let mut e2 = save_err;
                            let mut sv = saving;
                            spawn(async move {
                                match api::update_self_api(&client, &req).await {
                                    Ok(u) => {
                                        // 同步共享缓存, 顶栏 UserBadge 立即读到新显示名
                                        if let Ok(s) = serde_json::to_string(&u) {
                                            ui::set_storage_item("ferrite_current_user", &s);
                                        }
                                        f.set(Some("显示名已更新".into()));
                                    }
                                    Err(err2) => e2.set(err2.to_string()),
                                }
                                sv.set(false);
                            });
                        },
                        "保存显示名"
                    }
                }
            }

            div { class: "mt-4 grid grid-cols-1 gap-4 sm:grid-cols-2",
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "原密码" }
                    input {
                        class: "{ui::INPUT_MONO}",
                        r#type: "password",
                        "data-testid": "settings-original-password",
                        placeholder: "当前密码",
                        value: "{original_password}",
                        oninput: move |e| original_password.set(e.value()),
                    }
                }
                div {
                    label { class: "mb-1.5 block text-xs text-zinc-400", "新密码" }
                    input {
                        class: "{ui::INPUT_MONO}",
                        r#type: "password",
                        "data-testid": "settings-new-password",
                        placeholder: "留空 = 不修改密码",
                        value: "{new_password}",
                        oninput: move |e| new_password.set(e.value()),
                    }
                }
            }

            div { class: "mt-4 flex items-center gap-3",
                Button {
                    variant: ButtonVariant::Primary,
                    disabled: saving(),
                    "data-testid": "save-password",
                    "aria-label": "修改密码",
                    onclick: move |_| {
                        if saving() {
                            return;
                        }
                        let orig = original_password();
                        let next = new_password();
                        // 后端语义: 改密必须原密码 + 新密码成对; 两者都空 = 不改密, 不发请求
                        if orig.is_empty() && next.is_empty() {
                            return;
                        }
                        if orig.is_empty() || next.is_empty() {
                            save_err.set("修改密码须同时填写原密码与新密码".into());
                            save_flash.set(None);
                            return;
                        }
                        saving.set(true);
                        save_err.set(String::new());
                        save_flash.set(None);
                        let client = client::ApiClient::shared().clone();
                        let req = UpdateSelfRequest {
                            display_name: None,
                            original_password: Some(orig),
                            new_password: Some(next),
                        };
                        let mut op = original_password;
                        let mut np = new_password;
                        let mut f = save_flash;
                        let mut e2 = save_err;
                        let mut sv = saving;
                        spawn(async move {
                            match api::update_self_api(&client, &req).await {
                                Ok(_) => {
                                    op.set(String::new());
                                    np.set(String::new());
                                    f.set(Some("密码已修改".into()));
                                }
                                Err(err2) => e2.set(err2.to_string()),
                            }
                            sv.set(false);
                        });
                    },
                    "修改密码"
                }
                if let Some(m) = save_flash() {
                    span { class: "text-xs {ui::STATE_SUCCESS_TEXT}", "{m}" }
                }
            }

            if !save_err().is_empty() {
                p { class: "mt-3 text-xs {ui::STATE_DANGER_TEXT}", "操作失败: {save_err()}" }
            }
        }
    }
}
