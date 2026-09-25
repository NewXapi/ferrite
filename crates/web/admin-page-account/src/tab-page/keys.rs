//! keys 面板 — 个人资料、密钥列表、统计卡片、新建/编辑/删除/明文展示弹窗。
//! 组件组成：
//!   - KeysStatsSection   统计卡片区 (5 张 StatCard)
//!   - KeysProfileSection 个人资料区 (横向 ProfileItem ×4)
//!   - KeysListSection    密钥列表区 (KeyCard 循环 + 新建按钮)
//!   - NewKeyForm         新建密钥弹窗
//!   - EditKeyModal       编辑密钥弹窗
//!   - DeleteKeyModal     删除确认弹窗
//!   - CreatedKeyView     一次性明文展示
//!
//! 数据来源：api::{get_self_api, list_tokens_api, create_token_api, update_token_api, delete_token_api}
//!
//! 刷新时机：首次加载、create/update/delete 成功后、弹窗关闭回调里统一调用 load_keys()。

use dioxus::prelude::*;

use contract::api::token::{CreateTokenResult, TokenDto};
use contract::api::user::UserDto;

use crate::api;
use crate::components::{
    CreatedKeyView, DeleteKeyModal, EditKeyModal, KeysListSection, KeysProfileSection,
    KeysStatsSection, NewKeyForm,
};

// 跨组件共享状态块（页面层持有：初始化由跨组件动作触发、或需多组件读取）
// 弹窗表单状态：弹窗内部消费，但由「打开弹窗」这一跨组件动作初始化，
// 提升到页面层持有，以 Signal<T> prop 传入弹窗。
// 列表数据与错误：KeysListSection 与弹窗回调共享，故放页面层。

/// 拉取当前用户的密钥列表 (GET /api/token, owner 模式) 并写回三个 Signal。
/// 首次加载与 create/update/delete 成功后刷新共用此入口。
/// `Signal` 为 `Rc` 句柄 (Copy), 直接按值传入即可共享同一底层值。
fn load_keys(mut k: Signal<Vec<TokenDto>>, mut kl: Signal<bool>, mut ke: Signal<String>) {
    let client = client::ApiClient::shared().clone();
    spawn(async move {
        match api::list_tokens_api(&client).await {
            Ok(v) => {
                ke.set(String::new());
                k.set(v);
                kl.set(true);
            }
            Err(e) => ke.set(e.to_string()),
        }
    });
}

#[component]
pub fn KeysPanel() -> Element {
    // 新建表单临时状态
    let mut show_new_form = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_group = use_signal(String::new);
    let mut new_quota = use_signal(String::new);

    // 用户信息 (GET /api/user/self)
    let self_user = use_signal(|| None::<UserDto>);
    let self_err = use_signal(String::new);

    // 密钥列表 (GET /api/token)
    let keys = use_signal(Vec::<TokenDto>::new);
    let keys_loaded = use_signal(|| false);
    let keys_err = use_signal(String::new);

    // 新建成功后的明文展示 (只出现一次)
    let mut created_key = use_signal(|| None::<CreateTokenResult>);
    // 编辑 / 删除目标
    let mut edit_key = use_signal(|| None::<TokenDto>);
    let mut del_key = use_signal(|| None::<TokenDto>);

    // 初始化加载
    use_hook(move || {
        let client = client::ApiClient::shared().clone();
        // 用户信息
        spawn({
            let mut su = self_user;
            let mut se = self_err;
            async move {
                match api::get_self_api(&client).await {
                    Ok(u) => su.set(Some(u)),
                    Err(e) => se.set(e.to_string()),
                }
            }
        });
        // 密钥列表
        load_keys(keys, keys_loaded, keys_err);
    });

    let remaining: Option<i64> = self_user().as_ref().map(|u| u.quota - u.used_quota);
    let pending = String::from("…");
    let none_v = String::from("—");

    rsx! {
        div { class: "flex flex-col gap-6",
            // 1. 统计区
            KeysStatsSection {
                keys_loaded,
                keys_len: keys().len(),
                remaining,
                pending: pending.clone(),
                none_v: none_v.clone(),
            }

            // 2. 个人资料区
            KeysProfileSection {
                user: self_user(),
                self_err: self_err(),
                pending: pending.clone(),
            }

            // 3. 我的密钥区
            KeysListSection {
                keys: keys(),
                keys_loaded,
                keys_err,
                on_new: move |_| show_new_form.set(true),
                on_edit: move |tk: TokenDto| edit_key.set(Some(tk)),
                on_delete: move |tk: TokenDto| del_key.set(Some(tk)),
                on_toggle: move |tk: TokenDto| {
                    let mut k = keys;
                    let mut kl = keys_loaded;
                    let mut ke = keys_err;
                    let client = client::ApiClient::shared().clone();
                    spawn(async move {
                        let next_status = if tk.status == 1 { 2 } else { 1 };
                        let req = contract::api::token::UpdateTokenRequest {
                            status: Some(next_status),
                            ..Default::default()
                        };
                        if api::update_token_api(&client, &tk.key, &req).await.is_ok() {
                            match api::list_tokens_api(&client).await {
                                Ok(v) => { ke.set(String::new()); k.set(v); kl.set(true); }
                                Err(e) => ke.set(e.to_string()),
                            }
                        }
                    });
                },
            }
        }

        // 新建密钥弹窗
        if show_new_form() {
            NewKeyForm {
                name: new_name,
                group: new_group,
                quota: new_quota,
                on_cancel: move || show_new_form.set(false),
                on_created: move |res: CreateTokenResult| {
                    new_name.set(String::new());
                    new_group.set(String::new());
                    new_quota.set(String::new());
                    show_new_form.set(false);
                    created_key.set(Some(res));
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }

        // 新建成功: 一次性明文展示
        if let Some(res) = created_key() {
            CreatedKeyView {
                result: res,
                on_close: move || created_key.set(None),
            }
        }

        // 编辑密钥弹窗
        if let Some(t) = edit_key() {
            EditKeyModal {
                token: t,
                on_cancel: move || edit_key.set(None),
                on_saved: move |_| {
                    edit_key.set(None);
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }

        // 删除确认弹窗
        if let Some(t) = del_key() {
            DeleteKeyModal {
                token: t,
                on_cancel: move || del_key.set(None),
                on_confirmed: move |_| {
                    del_key.set(None);
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }
    }
}
