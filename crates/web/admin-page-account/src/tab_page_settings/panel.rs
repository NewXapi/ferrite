//! 用户设置面板 — 读写自由 JSONB 设置 (GET/PUT /api/user/self/setting)。
//! 后端对设置做 deep-merge, 故前端只提交本次改动的键即可。
//! 另含「资料与密码」编辑区: PUT /api/user/self 改显示名 / 改密码
//! (改密须原密码 + 新密码成对提供)。
//! 数据全部经 `api` 取用, 面板不认识数据怎么来。

use dioxus::prelude::*;

use crate::tab_page_settings::{AccountSection, PreferencesSection};

#[component]
pub fn SettingsPanel() -> Element {
    rsx! {
        div { class: "flex flex-col gap-6",
            PreferencesSection {}
            AccountSection {}
        }
    }
}