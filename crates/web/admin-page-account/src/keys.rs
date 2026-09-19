use dioxus::prelude::*;

use contract::api::token::{CreateTokenRequest, CreateTokenResult, TokenDto, UpdateTokenRequest};
use contract::api::user::{UserDto, role_label};

use crate::api;
use crate::usage_support::{fmt_quota, short_key, used_pct};
use ui::StatCard;
use ui::components::button::{Button, ButtonSize, ButtonVariant};

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

/// 密钥·资料面板 — 资料/密钥区全部走真实 API:
/// - 个人资料: GET/PUT /api/user/self
/// - 我的密钥: GET /api/token (owner 模式) + create/update/delete
/// - 个人数据卡: 密钥总数 (列表长度) / 剩余额度 (/self quota-used_quota)
///   / 成功率与近 30 天聚合暂无数据源 (daily 端点待 overview 会话合入后接回), 显示「—」
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
                                KeyCard {
                                    entry: t,
                                    on_edit: move |tk: TokenDto| {
                                        edit_key.set(Some(tk));
                                    },
                                    on_toggle: move |tk: TokenDto| {
                                        let mut k = keys;
                                        let mut kl = keys_loaded;
                                        let mut ke = keys_err;
                                        let client = client::ApiClient::shared().clone();
                                        spawn(async move {
                                            let next_status = if tk.status == 1 { 2 } else { 1 };
                                            let req = UpdateTokenRequest {
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

/// 横向资料项: label 与 value 同行 (label 灰、value 等宽字体)。
/// 由父容器 flex-wrap 控制换行, 单项不自带换行逻辑。
/// `copyable` 时在 value 后挂复制小按钮。注意: value 可短显
/// (如用户 ID 显示 "0000…aa01"), 复制按钮实际复制的内容取 `copy_value`;
/// 未传 (None) 时回退复制 `value` (对非短显项 = 复制原值, 向后兼容)。
#[component]
fn ProfileItem(
    label: &'static str,
    value: String,
    copyable: bool,
    copy_value: Option<String>,
) -> Element {
    let copy_text = copy_value.unwrap_or_else(|| value.clone());
    rsx! {
        div { class: "flex items-baseline gap-2",
            span { class: "shrink-0 text-zinc-400", "{label}" }
            span {
                class: "min-w-0 break-all font-mono text-zinc-200",
                title: "{value}",
                "{value}"
            }
            if copyable {
                div { class: "shrink-0 self-center",
                    CopyPlaintextButton { text: copy_text, label: format_args!("复制完整{label}").to_string() }
                }
            }
        }
    }
}

/// 密钥卡片 — 数据来自 TokenDto (GET /api/token)。
/// 操作: 编辑 (改名) / 停用·启用 (status 1↔2) / 删除, 成功后由父面板刷新列表。
/// 已用额度走 $ 口径 (fmt_quota, 500_000 ≈ $1) + used_pct 进度条,
/// 无限额度显示「无限」徽标且不渲染进度条。
#[component]
fn KeyCard(
    entry: TokenDto,
    on_edit: EventHandler<TokenDto>,
    on_toggle: EventHandler<TokenDto>,
    on_delete: EventHandler<TokenDto>,
) -> Element {
    let enabled = entry.status == 1;
    let status_color = if enabled {
        "bg-emerald-500/20 text-emerald-400 border-emerald-500/30"
    } else {
        "bg-amber-500/20 text-amber-400 border-amber-500/30"
    };
    // 每个 handler 闭包各持一份 clone, 避免 3 个 move 闭包连环占用 entry
    let e_edit = entry.clone();
    let e_toggle = entry.clone();
    let e_del = entry.clone();
    // RFC3339 → 展示取日期段 (无数据时不渲染)
    let created: String = entry
        .created_at
        .chars()
        .take(10)
        .filter(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    // 已用额度 $ 口径 (fmt_quota: 500_000 ≈ $1), 内部裸数对用户无意义;
    // 无限额度显示「无限」徽标 (参照 admin-page-users 面板处理, 跨 crate 只看不引)
    let unlimited = entry.unlimited_quota;
    // 进度条: 0..=100; quota <= 0 (无限/未设限额) 时 0, 超用 clamp 100。
    // 配色随用量升高转告警, 样式抄 admin-page-users panel.rs 的 bar_tone 风格
    let pct = used_pct(entry.quota, entry.used_quota);
    let bar_tone = if pct >= 90 {
        "bg-red-500"
    } else if pct >= 70 {
        "bg-amber-500"
    } else {
        "bg-emerald-500"
    };
    rsx! {
        div {
            class: "group rounded-xl border border-zinc-800 bg-zinc-900/60 p-4 transition-all duration-200 hover:border-zinc-600 hover:bg-zinc-900/80",
            div { class: "mb-3 flex items-start justify-between gap-2",
                div { class: "min-w-0",
                    h3 { class: "truncate text-sm font-medium text-zinc-100", "{entry.name}" }
                    // 掩码预览仅作展示 (完整明文不可再获取), 不提供复制 ——
                    // 复制到的是 `sk-ab****ef` 这类废串, 粘贴必失败。
                    p { class: "min-w-0 truncate font-mono text-[11px] text-zinc-500", "{entry.key_preview}" }
                }
                span {
                    class: "shrink-0 rounded-full border px-2.5 py-0.5 text-xs font-medium {status_color}",
                    if enabled { "启用" } else { "停用" }
                }
            }

            div { class: "space-y-2 text-xs",
                div { class: "flex items-center justify-between gap-2",
                    span { class: "shrink-0 whitespace-nowrap text-zinc-400", "已用额度" }
                    if unlimited {
                        span {
                            class: "whitespace-nowrap rounded-full border border-sky-500/30 bg-sky-500/20 px-2 py-0.5 text-[11px] font-medium text-sky-300",
                            "无限"
                        }
                    } else {
                        span { class: "whitespace-nowrap font-medium text-zinc-200",
                            "{fmt_quota(entry.used_quota)} / {fmt_quota(entry.quota)}"
                        }
                    }
                }
                // 用量进度条: 无限额度不渲染 (无分母, 百分比无意义)
                if !unlimited {
                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-zinc-800",
                        div { class: "h-full rounded-full {bar_tone}", style: "width: {pct}%" }
                    }
                }
                if !created.is_empty() {
                    div { class: "flex justify-between gap-2",
                        span { class: "shrink-0 whitespace-nowrap text-zinc-400", "创建时间" }
                        span { class: "whitespace-nowrap font-mono text-zinc-400", "{created}" }
                    }
                }
            }

            div { class: "mt-4 flex items-center gap-2 border-t border-zinc-800 pt-3",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-zinc-400",
                    onclick: move |_| on_edit.call(e_edit.clone()),
                    "编辑"
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-zinc-400",
                    onclick: move |_| on_toggle.call(e_toggle.clone()),
                    if enabled { "停用" } else { "启用" }
                }
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Xs,
                    class: "flex-1 text-red-400",
                    onclick: move |_| on_delete.call(e_del.clone()),
                    "删除"
                }
            }
        }
    }
}

/// 新建密钥弹窗 — 提交走 POST /api/token。
/// 额度输入留空 = 不限额 (unlimited_quota)。
/// 成功后由父组件弹出一次性明文视图。
#[component]
fn NewKeyForm(
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
            (q_raw.parse::<i64>().unwrap_or(0), false)
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
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "新建 API 密钥" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_cancel.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                div { class: "space-y-4",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "例如: 生产环境密钥",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组 (可选)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "留空 = 跟随用户默认分组",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度限制 (额度单位)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none",
                            r#type: "text",
                            placeholder: "留空 = 不限额",
                            value: "{quota}",
                            oninput: move |e| quota.set(e.value()),
                        }
                    }

                    if !err().is_empty() {
                        p { class: "text-xs text-red-400", "{err()}" }
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

/// 新建成功视图 — 一次性明文 key 展示 (后端只在创建响应返回一次)。
/// 提供一键复制明文 (这是唯一值得复制的完整密钥), 关闭后无法再查看。
#[component]
fn CreatedKeyView(result: CreateTokenResult, on_close: EventHandler<()>) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            div {
                class: "w-full max-w-md rounded-2xl border border-emerald-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-4 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-emerald-400", "密钥创建成功" }
                    button {
                        class: "rounded-lg p-1.5 text-zinc-500 transition-colors hover:bg-zinc-800 hover:text-zinc-200",
                        onclick: move |_| on_close.call(()),
                        "aria-label": "关闭",
                        "✕"
                    }
                }

                p { class: "mb-2 text-xs text-amber-400", "明文密钥只显示这一次,关闭后无法再查看" }
                div { class: "flex items-center gap-2",
                    input {
                        class: "min-w-0 flex-1 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-3 font-mono text-sm text-emerald-300 focus:outline-none",
                        r#type: "text",
                        r#readonly: true,
                        value: "{result.plaintext}"
                    }
                    button {
                        class: if copied() {
                            "shrink-0 rounded-xl border border-emerald-500/40 bg-emerald-500/10 px-3 py-3 text-xs font-medium text-emerald-400"
                        } else {
                            "shrink-0 rounded-xl border border-zinc-700 bg-zinc-950 px-3 py-3 text-xs font-medium text-zinc-300 transition-colors hover:bg-zinc-800"
                        },
                        "aria-label": "复制明文密钥",
                        onclick: move |_| {
                            // fire-and-forget 写入剪贴板, 提交即视为发起成功
                            let ok = ui::copy_text_to_clipboard(&result.plaintext);
                            copied.set(ok);
                            let mut c = copied;
                            spawn(async move {
                                gloo_timers::future::TimeoutFuture::new(1500).await;
                                c.set(false);
                            });
                        },
                        if copied() { "已复制" } else { "复制" }
                    }
                }
                p { class: "mt-2 text-xs text-zinc-500", "名称: {result.token.name}" }

                div { class: "mt-5 flex justify-end",
                    Button {
                        variant: ButtonVariant::Primary,
                        onclick: move |_| on_close.call(()),
                        "完成"
                    }
                }
            }
        }
    }
}

/// 编辑密钥弹窗 — 走 PUT /api/token/{key}。
/// 契约 UpdateTokenRequest 全 Option, 缺省字段不随请求发出 (skip_serializing_if),
/// 后端按「缺省 = 不改」处理 (admin-catalog tokens.rs svc.update 逐字段 if let Some)。
/// 注意: 后端 group / expires_at 是双层 Option (Some(None) = 跟随用户组 / 永不过期),
/// 契约层不表达「清空」语义, 所以这里的留空只能 = 保持不变。
#[component]
fn EditKeyModal(
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
    let mut expiry = use_signal(|| {
        crate::usage_support::rfc3339_to_date_input(token.expires_at.as_deref().unwrap_or(""))
    });
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
        let expires_at = crate::usage_support::date_input_to_rfc3339(expiry().trim());
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
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                div { class: "mb-5 flex items-center justify-between",
                    h3 { class: "text-base font-semibold text-zinc-100", "编辑密钥" }
                    p { class: "truncate font-mono text-xs text-zinc-500", "{token.key_preview}" }
                }

                // 字段较多, 弹窗保持 max-w-md 视觉, 字段区超高内部滚动
                div { class: "max-h-[60vh] space-y-4 overflow-y-auto",
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "密钥名称" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            value: "{name}",
                            oninput: move |e| name.set(e.value()),
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "分组 (可选)" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none",
                            placeholder: "留空 = 保持不变",
                            value: "{group}",
                            oninput: move |e| group.set(e.value()),
                        }
                        p { class: "mt-1 text-[11px] text-zinc-500",
                            "分组决定计费与模型可见范围; 跟随用户默认分组的密钥此处显示为空"
                        }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "额度限制 (额度单位)" }
                        label { class: "mb-1.5 flex cursor-pointer items-center gap-2 text-xs text-zinc-400",
                            input {
                                r#type: "checkbox",
                                class: "h-4 w-4 accent-emerald-500",
                                checked: "{unlimited}",
                                onchange: move |e| unlimited.set(e.checked()),
                            }
                            "无限额度"
                        }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none disabled:cursor-not-allowed disabled:opacity-40",
                            r#type: "text",
                            placeholder: "额度单位, 500,000 ≈ $1",
                            value: "{quota}",
                            disabled: unlimited(),
                            oninput: move |e| quota.set(e.value()),
                        }
                        p { class: "mt-1 text-[11px] text-zinc-500", "额度单位: 500,000 ≈ $1" }
                    }
                    div {
                        label { class: "mb-1.5 block text-xs text-zinc-400", "过期时间" }
                        input {
                            class: "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm text-zinc-200 focus:border-zinc-500 focus:outline-none",
                            r#type: "date",
                            value: "{expiry}",
                            oninput: move |e| expiry.set(e.value()),
                        }
                        p { class: "mt-1 text-[11px] text-zinc-500",
                            "留空 = 保持不变; 所选日期当日 (UTC) 结束后失效"
                        }
                    }
                    if !err().is_empty() {
                        p { class: "text-xs text-red-400", "{err()}" }
                    }
                }

                div { class: "mt-6 flex gap-3",
                    button {
                        class: "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    button {
                        class: "flex-1 rounded-xl bg-white py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:opacity-40",
                        disabled: busy(),
                        onclick: submit,
                        "保存"
                    }
                }
            }
        }
    }
}

/// 删除确认弹窗 — 走 DELETE /api/token/{key}。
#[component]
fn DeleteKeyModal(
    token: TokenDto,
    on_cancel: EventHandler<()>,
    on_confirmed: EventHandler<()>,
) -> Element {
    let mut busy = use_signal(|| false);
    let mut err = use_signal(String::new);

    let confirm = move |_| {
        busy.set(true);
        err.set(String::new());
        let client = client::ApiClient::shared().clone();
        let key_id = token.key.clone();
        let mut b = busy;
        let mut er = err;
        spawn(async move {
            match api::delete_token_api(&client, &key_id).await {
                Ok(_) => on_confirmed.call(()),
                Err(e) => {
                    er.set(e.to_string());
                    b.set(false);
                }
            }
        });
    };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
            onclick: move |_| on_cancel.call(()),
            div {
                class: "w-full max-w-md rounded-2xl border border-red-500/40 bg-zinc-900 p-5 shadow-xl",
                onclick: move |e| e.stop_propagation(),

                h3 { class: "text-base font-semibold text-zinc-100", "删除密钥" }
                p { class: "mt-3 text-sm text-zinc-400",
                    "确认删除「{token.name}」({token.key_preview})？删除后使用该密钥的调用会立即失败, 且无法恢复。"
                }
                if !err().is_empty() {
                    p { class: "mt-3 text-xs text-red-400", "{err()}" }
                }

                div { class: "mt-6 flex gap-3",
                    Button {
                        variant: ButtonVariant::Outline,
                        class: "flex-1",
                        onclick: move |_| on_cancel.call(()),
                        "取消"
                    }
                    Button {
                        variant: ButtonVariant::Destructive,
                        class: "flex-1",
                        disabled: busy(),
                        onclick: confirm,
                        "确认删除"
                    }
                }
            }
        }
    }
}

/// 复制完整明文的小图标按钮: 点击写入剪贴板, 复制成功有可见反馈
/// (按钮文案瞬时变「已复制」/ 图标变 ✓)。
/// 只用于真正值得复制的完整值 —— 一次性明文密钥 / 完整用户 ID;
/// 掩码预览 (sk-ab****ef) 禁止用此组件, 复制掩码是功能错误。
/// 复制语义与 ui-components session::copy_text_to_clipboard 一致:
/// Clipboard API fire-and-forget, 提交即视为成功。
#[component]
fn CopyPlaintextButton(text: String, label: String) -> Element {
    let mut copied = use_signal(|| false);
    rsx! {
        Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::IconXs,
            title: "{label}",
            "aria-label": "{label}",
            class: if copied() { "text-emerald-400" } else { "" },
            onclick: move |_| {
                let ok = ui::copy_text_to_clipboard(text.as_str());
                copied.set(ok);
                let mut c = copied;
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(1500).await;
                    c.set(false);
                });
            },
            if copied() {
                span { class: "block h-3.5 w-3.5 text-center text-[11px] leading-[14px]", "✓" }
            } else {
                svg {
                    class: "h-3.5 w-3.5",
                    fill: "none",
                    stroke: "currentColor",
                    view_box: "0 0 24 24",
                    stroke_width: "2",
                    rect { x: "9", y: "9", width: "13", height: "13", rx: "2" }
                    path { d: "M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" }
                }
            }
        }
    }
}
