//! 密钥·资料面板 — 资料/密钥区全部走真实 API:
//! - 个人资料: GET/PUT /api/user/self
//! - 我的密钥: GET /api/token (owner 模式) + create/update/delete
//! - 个人数据卡: 密钥总数 (列表长度) / 剩余额度 (/self quota-used_quota)
//!   / 成功率与近 30 天聚合暂无数据源 (daily 端点待 overview 会话合入后接回), 显示「—」

use dioxus::prelude::*;
use ui::StatCard;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

use contract::api::token::{CreateTokenResult, TokenDto};
use contract::api::user::{UserDto, role_label};

use crate::api;
use crate::tab_page_keys::{
    CreatedKeyView, DeleteKeyModal, EditKeyModal, KeyCard, NewKeyForm, ProfileItem,
};
use crate::usage_support::{fmt_quota, short_key};

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
    // —— 区段标题 (ScrollSpyNav + h2 同用) ——
    const SEC_STATS: &str = "个人数据";
    const SEC_PROFILE: &str = "个人资料";
    const SEC_KEYS: &str = "我的密钥";

    let mut show_new_form = use_signal(|| false);
    let mut new_name = use_signal(String::new);
    let mut new_group = use_signal(String::new);
    let mut new_quota = use_signal(String::new);

    // ---- 真实用户信息 (GET /api/user/self) ----
    let self_user = use_signal(|| None::<UserDto>);
    let self_err = use_signal(String::new);

    // ---- 我的密钥 (GET /api/token) + 近 30 天按天统计 ----
    let keys = use_signal(Vec::<TokenDto>::new);
    let keys_loaded = use_signal(|| false);
    let keys_err = use_signal(String::new);
    // 可选分组名 (GET /api/token/auto-groups) — 卡牌分组切换菜单数据源
    let groups = use_signal(Vec::<String>::new);
    // 本次会话创建时的一次性明文 (后端只存 sha256, 刷新即失):
    // key UUID → plaintext, 仅这些卡牌渲染复制按钮
    let mut plain_keys = use_signal(std::collections::HashMap::<String, String>::new);

    // 新建成功后的明文展示 (只出现一次)
    let mut created_key = use_signal(|| None::<CreateTokenResult>);
    // 编辑 / 删除目标
    let mut edit_key = use_signal(|| None::<TokenDto>);
    let mut del_key = use_signal(|| None::<TokenDto>);

    use_hook(move || {
        let client = client::ApiClient::shared().clone();

        // self 资料
        let mut su = self_user;
        let mut se = self_err;
        spawn(async move {
            match api::get_self_api(&client).await {
                Ok(u) => {
                    // 同步刷新共享缓存 (UserBadge / 顶栏读取 ferrite_current_user)
                    if let Ok(s) = serde_json::to_string(&u) {
                        ui::set_storage_item("ferrite_current_user", &s);
                    }
                    su.set(Some(u));
                }
                Err(e) => se.set(e.to_string()),
            }
        });

        // 可选分组名首载 (分组切换菜单用; 失败留空 = 卡牌分组只读)
        let mut g = groups;
        let gclient = client::ApiClient::shared().clone();
        spawn(async move {
            if let Ok(v) = api::list_auto_groups_api(&gclient).await {
                g.set(v);
            }
        });

        // 密钥列表首载 (create/update/delete 成功后同入口刷新)
        load_keys(keys, keys_loaded, keys_err);
    });

    let remaining: Option<i64> = self_user().as_ref().map(|u| u.quota - u.used_quota);

    let pending = String::from("…");
    let none_v = String::from("—");

    rsx! {
        div { class: "flex flex-col gap-6",
            // 1. 统计区
            section {
                id: "keys-sec-stats",
                class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_STATS}" }
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                    StatCard {
                        value: if keys_loaded() { keys().len().to_string() } else { pending.clone() },
                        label: "密钥总数",
                    }
                    StatCard { value: none_v.clone(), label: "近 30 天消耗 (暂无数据)" }
                    StatCard { value: none_v.clone(), label: "近 30 天请求 (暂无数据)" }
                    StatCard {
                        value: match remaining {
                            // 剩余额度 $ 口径展示 (QUOTA_PER_USD = 500_000 ≈ $1, 同 usage_support::fmt_quota);
                            // 内部裸数 (如 2500000) 对用户无意义
                            Some(v) => fmt_quota(v),
                            None if self_err().is_empty() => pending.clone(),
                            None => none_v.clone(),
                        },
                        label: "剩余额度 (≈$)",
                    }
                    StatCard {
                        value: none_v,
                        label: "成功率 (暂无数据)",
                    }
                }
            }

            // 2. 个人资料区
            section {
                id: "keys-sec-profile",
                class: "scroll-mt-8 space-y-3",
                h2 { class: "text-lg font-medium text-zinc-100", "{SEC_PROFILE}" }
                div { class: "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6",
                    div { class: "flex items-start justify-between gap-4",
                        div { class: "min-w-0",
                            if let Some(user) = self_user() {
                                div { class: "mb-4 flex items-center gap-2",
                                    span { class: "truncate text-sm font-medium text-zinc-100", "{user.username}" }
                                    span { class: "shrink-0 rounded-full bg-zinc-800 px-2 py-0.5 text-[10px] font-medium text-zinc-400", "{role_label(user.role)}" }
                                }
                                // 资料项横向流式排布, 一行放不下自动换行。
                                // 无「分组」行: group 是创建密钥时的分组语义, 不属于用户资料。
                                div { class: "flex flex-wrap items-baseline gap-x-14 gap-y-4 text-sm",
                                    ProfileItem { label: "显示名", value: user.display_name.clone(), copyable: false }
                                    ProfileItem { label: "邮箱", value: if user.email.is_empty() { "—".to_string() } else { user.email.clone() }, copyable: false }
                                    // 用户 ID 短显 (前4…后4), 复制按钮复制完整 UUID (copy_value),
                                    // 悬停 title 也有全值
                                    ProfileItem { label: "用户ID", value: short_key(&user.key), copy_value: Some(user.key.clone()), copyable: true }
                                    ProfileItem { label: "注册时间", value: user.created_at.chars().take(10).collect::<String>(), copyable: false }
                                }
                            } else if !self_err().is_empty() {
                                p { class: "text-sm text-amber-400", "无法加载用户信息 (未登录或请求失败): {self_err()}" }
                            } else {
                                p { class: "text-sm text-zinc-500", "加载中…" }
                            }
                        }
                    }
                }
            }

            // 3. 我的密钥区
            section {
                id: "keys-sec-keys",
                class: "scroll-mt-8",
                div { class: "space-y-4",
                    div { class: "flex items-center justify-between gap-3",
                        div { class: "flex items-center gap-2",
                            h2 { class: "text-lg font-medium text-zinc-100", "{SEC_KEYS}" }
                            span { class: "text-xs px-3 py-1 rounded-full bg-zinc-800 text-zinc-400",
                                if keys_loaded() { "{keys().len()} 个" } else { "…" }
                            }
                        }
                        Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            onclick: move |_| show_new_form.set(true),
                            "✚ 新建密钥"
                        }
                    }

                    if !keys_err().is_empty() {
                        p { class: "text-sm text-amber-400", "无法加载密钥 (未登录或请求失败): {keys_err()}" }
                    } else if !keys_loaded() {
                        p { class: "text-sm text-zinc-500", "加载中…" }
                    } else if keys().is_empty() {
                        p { class: "text-sm text-zinc-500", "还没有密钥,点「✚ 新建密钥」签发第一个" }
                    } else {
                        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5",
                            for t in keys() {
                                // key 按密钥 UUID 绑定卡牌: 无 key 时 Dioxus 按位置 diff,
                                // 重取列表顺序变化会让卡牌串位——「点一张卡的按钮, 其他卡的
                                // 按钮跟着变」的根因 (维护者批注 2026-09-21 15:50)。
                                KeyCard {
                                    key: "{t.key}",
                                    entry: t.clone(),
                                    groups: groups(),
                                    plain_key: plain_keys().get(&t.key).cloned(),
                                    on_edit: move |tk: TokenDto| {
                                        edit_key.set(Some(tk));
                                    },
                                    on_toggle: move |tk: TokenDto| {
                                        let mut k = keys;
                                        let mut kl = keys_loaded;
                                        let mut ke = keys_err;
                                        // 乐观更新: 点击立即翻转本卡状态 (批注 15:50:
                                        // 按钮反馈要卡牌内部即时生效), 随后重取以服务端
                                        // 为准对齐 (含 PUT 失败回滚)。
                                        k.write().iter_mut().for_each(|t| {
                                            if t.key == tk.key {
                                                t.status = if t.status == 1 { 2 } else { 1 };
                                            }
                                        });
                                        let client = client::ApiClient::shared().clone();
                                        spawn(async move {
                                            let next_status = if tk.status == 1 { 2 } else { 1 };
                                            let req = contract::api::token::UpdateTokenRequest {
                                                status: Some(next_status),
                                                ..Default::default()
                                            };
                                            let _ = api::update_token_api(&client, &tk.key, &req).await;
                                            match api::list_tokens_api(&client).await {
                                                Ok(v) => { ke.set(String::new()); k.set(v); kl.set(true); }
                                                Err(e) => ke.set(e.to_string()),
                                            }
                                        });
                                    },
                                    on_group_change: move |(tk, g): (TokenDto, String)| {
                                        let mut k = keys;
                                        let mut kl = keys_loaded;
                                        let mut ke = keys_err;
                                        let client = client::ApiClient::shared().clone();
                                        spawn(async move {
                                            let req = contract::api::token::UpdateTokenRequest {
                                                group: Some(g),
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
                                    on_delete: move |tk: TokenDto| {
                                        del_key.set(Some(tk));
                                    },
                                }
                            }
                        }
                    }
                }
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
                    // 会话明文入表: 该卡牌随后带复制按钮 (后端只存 sha256, 刷新即失)
                    if !res.plaintext.is_empty() {
                        plain_keys.write().insert(res.token.key.clone(), res.plaintext.clone());
                    }
                    created_key.set(Some(res));
                    load_keys(keys, keys_loaded, keys_err);
                },
            }
        }

        // 新建成功: 一次性明文展示
        if let Some(res) = created_key() {
            CreatedKeyView {
                result: res,
                on_close: move || {
                    created_key.set(None);
                },
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
