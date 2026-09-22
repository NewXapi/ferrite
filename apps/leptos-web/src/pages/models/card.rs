//! 模型页面 — 移植自 dioxus admin-page-overview 的 tab-page-models
//! 使用 singlestage 组件：Card、CardGrid、Button，静态数据，RwSignal 交互

use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;
#[derive(Clone)]
pub struct ModelCardView {
    name: String,
    owner: String,
    model_type: String,
    status: i32,
    usage_count: i64,
    max_tokens: i64,
    is_vision: bool,
    is_tool: bool,
}

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
pub fn ModelCard(model: ModelCardView) -> impl IntoView {
    let status_text = if model.status == 1 { CARD_ENABLED } else { CARD_DISABLED };
    let status_class = if model.status == 1 { "text-emerald-400" } else { "text-red-400" };

    view! {
        <Card class="h-full">
            <CardHeader class="pb-2">
                <div class="flex items-start justify-between gap-2">
                    <div class="min-w-0">
                        <CardTitle class="text-base font-medium text-zinc-100 truncate">{model.name}</CardTitle>
                        <p class="mt-0.5 text-xs text-zinc-500 truncate">{model.owner}</p>
                    </div>
                    <span class=format!("flex-shrink-0 px-2 py-0.5 rounded-full text-xs font-medium {}", status_class)>
                        {status_text}
                    </span>
                </div>
            </CardHeader>
            <CardContent class="space-y-4 pt-0">
                // Headline: 累计调用
                <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-4 py-3">
                    <div class="text-xs text-zinc-500">{CARD_HEADLINE}</div>
                    <div class="mt-1 text-2xl font-semibold tracking-tight text-zinc-100">{model.usage_count.to_string()}</div>
                </div>

                // Mini stats: 类型 / 最大上下文 / 状态
                <div class="grid grid-cols-3 gap-3 text-center">
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div class="text-xs text-zinc-500">{CARD_TYPE}</div>
                        <div class="mt-0.5 text-sm font-medium text-zinc-100 truncate">{model.model_type}</div>
                    </div>
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div class="text-xs text-zinc-500">{CARD_CONTEXT}</div>
                        <div class="mt-0.5 text-sm font-medium text-zinc-100">{model.max_tokens.to_string()}</div>
                    </div>
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div class="text-xs text-zinc-500">{CARD_STATUS}</div>
                        <div class=format!("mt-0.5 text-sm font-medium {}", status_class)>{status_text}</div>
                    </div>
                </div>

                // 预留：价格/趋势/热力/分组（当前静态数据无对应字段，用 DASH 占位）
                <div class="grid grid-cols-3 gap-3 text-center text-xs text-zinc-500">
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div>输入价</div>
                        <div class="font-mono">{DASH}</div>
                    </div>
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div>输出价</div>
                        <div class="font-mono">{DASH}</div>
                    </div>
                    <div class="rounded-xl border border-zinc-800 bg-zinc-900/60 px-3 py-2.5">
                        <div>缓存价</div>
                        <div class="font-mono">{DASH}</div>
                    </div>
                </div>
            </CardContent>
        </Card>
    }
}

// 静态演示数据（对齐 Dioxus static_models / 后端 ModelView 字段）
