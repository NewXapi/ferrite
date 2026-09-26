//! 模型 tab 页面层:状态 + 拉取 effect + 四态分支与卡片网格组合。

use dioxus::prelude::*;

use super::card::ModelCard;
use super::shared::{
    BTN_RETRY, MODELS_COUNT_HEAD, MODELS_COUNT_TAIL, MODELS_EMPTY, MODELS_EMPTY_HINT, MODELS_ERR,
    MODELS_LOADING, MODELS_TITLE,
};
use client::ApiClient;

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
            "aria-label": MODELS_TITLE,
            div { class: "flex items-baseline justify-between",
                h2 { class: "{ui::T_text_base} {ui::T_font_semibold} {ui::T_text_zinc_100}", "{MODELS_TITLE}" }
                span { class: "{ui::T_text_xs} {ui::T_text_zinc_600}", "{MODELS_COUNT_HEAD}{total}{MODELS_COUNT_TAIL}" }
            }
            if let Some(e) = err {
                div { class: "rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center",
                    p { class: "{ui::T_text_sm} {ui::T_text_red_300}", "{MODELS_ERR}" }
                    p { class: "mt-1 {ui::T_text_xs} text-red-400/70", "{e}" }
                    button {
                        class: "mt-3 rounded-xl border border-border px-3 py-1.5 {ui::T_text_xs} text-muted-foreground hover:bg-accent",
                        onclick: move |_| reload.set(reload() + 1),
                        "{BTN_RETRY}"
                    }
                }
            } else if loading {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "{MODELS_LOADING}" }
                }
            } else if list.is_empty() {
                div { class: "rounded-2xl border border-dashed border-border bg-card/50 py-10 text-center",
                    p { class: "text-muted-foreground", "{MODELS_EMPTY}" }
                    p { class: "mt-1 {ui::T_text_xs} text-muted-foreground/70", "{MODELS_EMPTY_HINT}" }
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
