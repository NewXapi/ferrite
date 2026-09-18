//! 模型 tab 页面层:状态 + 拉取 effect + 四态分支与卡片网格组合。

use dioxus::prelude::*;

use client::ApiClient;

use super::card::ModelCard;
use crate::api::{self, ModelCardView};

/// 模型页入口: 卡网格(页头 + ModelCard), 数据经 [`api::list_models_api`] 取自
/// `/api/models`(loading / 错误 / 空态走与 `health.rs` 同一的 Signal 取数模式)。
/// 遵循页面响应式约定 (手机 1 栏 / 平板 md 3 栏 / Web lg 5 栏)。
#[component]
pub fn ModelsPanel() -> Element {
    let mut models = use_signal(Vec::<ModelCardView>::new);
    let mut total = use_signal(|| 0i64);
    let mut loading = use_signal(|| true);
    let mut err = use_signal(|| None::<String>);
    let mut reload = use_signal(|| 0u32);

    use_effect(move || {
        let _ = reload();
        loading.set(true);
        err.set(None);
        spawn(async move {
            let client = ApiClient::shared().clone();
            match api::list_models_api(&client).await {
                Ok((items, n)) => {
                    models.set(items);
                    total.set(n);
                    loading.set(false);
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    loading.set(false);
                }
            }
        });
    });

    let list = models();
    let total = total();
    let loading = loading();
    let err = err();

    rsx! {
        div { class: "space-y-4",
            "data-testid": "models-panel",
            role: "region",
            "aria-label": "模型",
            div { class: "flex items-baseline justify-between",
                h2 { class: "text-base font-semibold text-zinc-100", "模型" }
                span { class: "text-xs text-zinc-600", "共 {total} 个" }
            }
            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "text-sm text-red-300", "加载模型列表失败" }
                    p { class: "mt-1 text-xs text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-border px-3 py-1.5 text-xs text-muted-foreground hover:bg-accent",
                        onclick: move |_| reload.set(reload() + 1),
                        "重试"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "正在加载模型列表…" }
                }
            } else if list.is_empty() {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "暂无模型" }
                    p { class: "mt-1 text-xs text-muted-foreground/70", "/api/models 返回空列表 —— 配置模型后这里会展示真实卡片" }
                }
            } else {
                div { class: "grid grid-cols-1 gap-3 md:grid-cols-3 md:gap-4 lg:grid-cols-5",
                    for m in &list {
                        ModelCard { model: m.clone() }
                    }
                }
            }
        }
    }
}
