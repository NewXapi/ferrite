//! 状态/属性小徽标(圆角胶囊,配色由调用方按语义传入)。
//!
//! 【是什么】一枚 `rounded-full` 的胶囊小标签,用于卡片徽标行(倍率 / 内置 / 状态 /
//! 面值)与兑换码卡片。
//!
//! 【做什么】渲染 `text` 并套用调用方给的 `tone` 配色串。不负责决定语义配色
//! (由调用方按 status 判定)、不负责点击、不含任何状态。
//!
//! 【交互逻辑】纯展示,无交互:无 `EventHandler`,无网络请求。
//!
//! 【样式】`rounded-full border px-2 py-0.5 text-[11px] font-medium` 为固定部分,
//! 具体色系(emerald 折扣 / amber 溢价 / zinc 基准 / blue 内置)完全由 `tone`
//! 注入,本组件不自带任何色彩倾向。
//!
//! 【子组件组成】无(单个原生 `span`)。
//!
//! 【数据流】
//! - 对内(入):`text` 徽标文案(状态名、`面值 ¥N` 等)、`tone` 语义配色 class 串
//!   (调用方在 `GroupCard` / `RedemptionCard` / `AliasCard` 内按 status 算好)。
//! - 对外(出):无。

use dioxus::prelude::*;

/// 状态/属性小徽标。
#[component]
pub fn Badge(text: String, tone: &'static str) -> Element {
    let class = match tone {
        "emerald" => "rounded-full border border-emerald-600 bg-emerald-900/40 px-2 py-0.5 text-[11px] font-medium text-emerald-300",
        "amber" => "rounded-full border border-amber-600 bg-amber-900/40 px-2 py-0.5 text-[11px] font-medium text-amber-300",
        "zinc" => "rounded-full border border-zinc-600 bg-zinc-900/40 px-2 py-0.5 text-[11px] font-medium text-zinc-400",
        "blue" => "rounded-full border border-blue-600 bg-blue-900/40 px-2 py-0.5 text-[11px] font-medium text-blue-300",
        _ => "rounded-full border border-zinc-600 bg-zinc-900/40 px-2 py-0.5 text-[11px] font-medium text-zinc-400",
    };

    rsx! {
        span { class, "{text}" }
    }
}