//! Empty — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Empty(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex flex-col items-center justify-center gap-4 rounded-lg border border-dashed p-8 text-center",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { class: "{merged}", {children} } }
}

#[component]
pub fn EmptyHeader(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex flex-col items-center gap-2",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { class: "{merged}", {children} } }
}

#[component]
pub fn EmptyTitle(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "text-lg font-semibold leading-none",
        class.as_deref().unwrap_or("")
    );
    rsx! { h3 { class: "{merged}", {children} } }
}

#[component]
pub fn EmptyDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!(
        "text-muted-foreground text-sm",
        class.as_deref().unwrap_or("")
    );
    rsx! { p { class: "{merged}", {children} } }
}

#[component]
pub fn EmptyContent(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!(
        "flex items-center justify-center gap-2",
        class.as_deref().unwrap_or("")
    );
    rsx! { div { class: "{merged}", {children} } }
}

#[derive(Default, Clone, PartialEq, Eq)]
pub enum EmptyMediaVariant {
    #[default]
    Default,
    Icon,
}

#[component]
pub fn EmptyMedia(
    #[props(into, optional)] class: Option<String>,
    #[props(default = EmptyMediaVariant::default())] variant: EmptyMediaVariant,
    children: Element,
) -> Element {
    let variant_class = match variant {
        EmptyMediaVariant::Default => "bg-transparent",
        EmptyMediaVariant::Icon => {
            "bg-muted text-foreground flex size-10 shrink-0 items-center justify-center rounded-lg [&_svg:not([class*='size-'])]:size-6"
        }
    };
    let merged = tw_merge!(
        "flex shrink-0 items-center justify-center mb-2 [&_svg]:pointer-events-none [&_svg]:shrink-0",
        variant_class,
        class.as_deref().unwrap_or("")
    );
    rsx! { div { class: "{merged}", {children} } }
}
