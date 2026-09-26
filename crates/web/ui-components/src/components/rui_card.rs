//! Card — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum CardSize {
    #[default]
    Default,
    Sm,
}

#[component]
pub fn Card(
    #[props(into, optional)] class: Option<String>,
    #[props(default = CardSize::default())] size: CardSize,
    children: Element,
) -> Element {
    let size_classes = match size {
        CardSize::Default => "px-6 py-5 gap-4",
        CardSize::Sm => "px-4 py-3 gap-1",
    };
    let data_size = match size {
        CardSize::Default => "default",
        CardSize::Sm => "sm",
    };
    let merged_class = tw_merge!(
        "flex flex-col rounded-xl border border-zinc-800 bg-zinc-900/60 text-zinc-100 transition-colors hover:border-zinc-600",
        size_classes,
        class.as_deref().unwrap_or("")
    );

    rsx! {
        div { "data-name": "Card", "data-size": data_size, class: "{merged_class}", {children} }
    }
}

#[component]
pub fn CardHeader(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!(
        "@container/card-header flex flex-col items-start gap-1.5 px-6 [[data-size=sm]_&]:px-4 [.border-b]:pb-6 sm:grid sm:auto-rows-min sm:grid-rows-[auto_auto] has-data-[slot=card-action]:sm:grid-cols-[1fr_auto]",
        class.as_deref().unwrap_or("")
    );

    rsx! { div { "data-name": "CardHeader", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!("leading-none font-semibold", class.as_deref().unwrap_or(""));

    rsx! { h2 { "data-name": "CardTitle", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardContent(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!(
        "px-6 [[data-size=sm]_&]:px-4",
        class.as_deref().unwrap_or("")
    );

    rsx! { div { "data-name": "CardContent", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged_class = tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );

    rsx! { p { "data-name": "CardDescription", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardFooter(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!(
        "flex items-center gap-2 px-6 [[data-size=sm]_&]:px-4 [.border-t]:pt-6",
        class.as_deref().unwrap_or("")
    );

    rsx! { footer { "data-name": "CardFooter", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardAction(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!(
        "self-start sm:col-start-2 sm:row-span-2 sm:row-start-1 sm:justify-self-end",
        class.as_deref().unwrap_or("")
    );

    rsx! { div { "data-name": "CardAction", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardList(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!("flex flex-col gap-4", class.as_deref().unwrap_or(""));

    rsx! { ul { "data-name": "CardList", class: "{merged_class}", {children} } }
}

#[component]
pub fn CardItem(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged_class = tw_merge!(
        "flex items-center [&_svg:not([class*='size-'])]:size-4 [&_svg]:shrink-0",
        class.as_deref().unwrap_or("")
    );

    rsx! { li { "data-name": "CardItem", class: "{merged_class}", {children} } }
}
