//! 站点选项面板:列表来自 `list_options_api`(`/api/option` 注册表与数据库值),
//! 补三态 loading/error/empty;另提供选项值展示与可编辑判定两个纯函数。
//! 使用基本 dioxus 元素 (ponytail: 避免依赖未导出的 pub(crate) 组件)。

use dioxus::prelude::*;

use crate::api::{OptionView, list_options_api};
use client::ApiClient;

/// 纯函数:选项值 → 展示串(数字保持原样,字符串去掉引号,其余原样 JSON)。
/// 空值统一显示占位符「—」,不造数据。
pub fn format_option_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "—".to_string(),
        other => other.to_string(),
    }
}

/// 纯函数:判断选项是否可编辑(数值/布尔可编辑,字符串与嵌套结构只读)。
pub fn option_editable(v: &serde_json::Value) -> bool {
    matches!(v, serde_json::Value::Number(_) | serde_json::Value::Bool(_))
}

/// 站点选项面板:列表来自 `list_options_api`,补三态 loading/error/empty。
#[component]
pub fn SystemOptionsPanel() -> Element {
    let mut options = use_signal(Vec::<OptionView>::new);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);

    use_effect(move || {
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared();
            match list_options_api(&client).await {
                Ok(v) => options.set(v),
                Err(e) => err.set(Some(e.to_string())),
            }
            loading.set(false);
        });
    });

    let list = options();
    let err = err();
    let loading = loading();

    rsx! {
        section {
            id: "system-sec-options",
            "data-testid": "system-options-panel",
            role: "region",
            "aria-label": "站点选项",
            class: "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900/60 p-5 space-y-4",
            div {
                h2 { class: "text-sm font-medium text-zinc-200", "站点选项" }
                p { class: "text-xs text-zinc-500", "运行时选项 (key/value 平表),来自 /api/option 注册表与数据库值" }
            }
            if let Some(e) = err {
                div {
                    role: "alert",
                    "data-testid": "system-options-error",
                    class: "rounded-xl border border-red-500/30 bg-red-950/30 p-4 text-sm text-red-400",
                    "{e}"
                }
            } else if loading {
                div {
                    "data-testid": "system-options-loading",
                    class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-6 text-center",
                    p { class: "text-zinc-400", "正在加载站点选项…" }
                }
            } else if list.is_empty() {
                div {
                    "data-testid": "system-options-empty",
                    class: "rounded-xl border border-dashed border-zinc-700 bg-zinc-900/50 py-6 text-center",
                    p { class: "text-zinc-400", "暂无站点选项" }
                }
            } else {
                div { class: "divide-y divide-zinc-800/80",
                    for (i, o) in list.iter().enumerate() {
                        {
                            let display = format_option_value(&o.value);
                            let editable = option_editable(&o.value);
                            rsx! {
                                div {
                                    key: "{i}",
                                    class: "flex items-center justify-between gap-4 py-2.5",
                                    span { class: "shrink-0 text-xs text-zinc-500", "{o.key}" }
                                    span {
                                        "data-testid": "system-option-value",
                                        class: if editable { "break-all text-right text-xs font-mono text-zinc-300" } else { "break-all text-right text-xs font-mono text-zinc-500" },
                                        "{display}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
