//! 模型页面 — 移植自 dioxus admin-page-overview 的 tab-page-models
//! 使用 singlestage 组件：Card、CardGrid、Button，静态数据，RwSignal 交互

use super::card::{ModelCard, ModelCardView};
use super::data::static_models;
use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

// 常量（对齐 Dioxus shared.rs）
const MODELS_TITLE: &str = "模型";
const MODELS_COUNT_HEAD: &str = "共 ";
const MODELS_COUNT_TAIL: &str = " 个";
const MODELS_ERR: &str = "加载模型列表失败";
const BTN_RETRY: &str = "重试";
const MODELS_LOADING: &str = "正在加载模型列表…";
const MODELS_EMPTY: &str = "暂无模型";
const MODELS_EMPTY_HINT: &str = "/api/models 返回空列表 —— 配置模型后这里会展示真实卡片";
const DASH: &str = "—";
const CARD_HEADLINE: &str = "累计调用";
const CARD_TYPE: &str = "类型";
const CARD_CONTEXT: &str = "最大上下文";
const CARD_STATUS: &str = "状态";
const CARD_ENABLED: &str = "启用";
const CARD_DISABLED: &str = "停用";

#[component]
pub fn ModelsPage() -> impl IntoView {
    // 静态演示数据（对齐 Dioxus 引用）
    let models = static_models();

    // 交互状态
    let loading = RwSignal::new(false);
    let err = RwSignal::new(None::<String>);
    let reload = RwSignal::new(0u32);

    view! {
        <section class="space-y-4" data-testid="models-page" role="region" aria-label=MODELS_TITLE>
            <div class="flex items-baseline justify-between">
                <h2 class="text-base font-semibold text-zinc-100">{MODELS_TITLE}</h2>
                <span class="text-xs text-zinc-600">
                    {MODELS_COUNT_HEAD}{models.len()}{MODELS_COUNT_TAIL}
                </span>
            </div>
            {move || {
                let list = models.clone();
                if let Some(e) = err.get() {
                    view! {
                        <div class="rounded-2xl border border-red-800/60 bg-red-950/40 px-4 py-6 text-center">
                            <p class="text-sm text-red-300">{MODELS_ERR}</p>
                            <p class="mt-1 text-xs text-red-400/70">{e}</p>
                            <button
                                class="mt-3 rounded-xl border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-xs text-zinc-300 hover:bg-zinc-700"
                                on:click=move |_| reload.set(reload.get_untracked() + 1)
                            >
                                {BTN_RETRY}
                            </button>
                        </div>
                    }.into_view()
                } else if loading.get() {
                    view! {
                        <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center">
                            <p class="text-zinc-400">{MODELS_LOADING}</p>
                        </div>
                    }.into_view()
                } else if list.is_empty() {
                    view! {
                        <div class="rounded-2xl border border-dashed border-zinc-700 bg-zinc-900/50 py-10 text-center">
                            <p class="text-zinc-400">{MODELS_EMPTY}</p>
                            <p class="mt-1 text-xs text-zinc-500">{MODELS_EMPTY_HINT}</p>
                        </div>
                    }.into_view()
                } else {
                    view! {
                        <CardGrid>
                            {list.into_iter().map(|model| view! { <ModelCard model=model/> }).collect_view()}
                        </CardGrid>
                    }.into_view()
                }
            }}
        </section>
    }
}
