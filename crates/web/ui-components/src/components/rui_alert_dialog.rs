//! AlertDialog — rust-ui (github.com/rust-ui/ui) registry 原版拷贝（shadcn copy-paste 层）。
//!
//! 与 crate 内同名自研组件并存，rust-ui 线走 `rui_` 前缀；仅按需去上游依赖。
use dioxus::prelude::*;

use crate::components::rui_dialog::{
    Dialog, DialogAction, DialogBody, DialogClose, DialogContent, DialogDescription, DialogFooter,
    DialogHeader, DialogTitle, DialogTrigger,
};

#[component]
pub fn AlertDialog(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    rsx! { Dialog { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogTrigger(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogTrigger { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogContent(
    #[props(into, optional)] class: Option<String>,
    #[props(default)] open: bool,
    #[props(default)] on_close: Option<EventHandler<()>>,
    children: Element,
) -> Element {
    rsx! {
        DialogContent {
            class: class.unwrap_or_default(),
            close_on_backdrop_click: false,
            open,
            on_close,
            {children}
        }
    }
}

#[component]
pub fn AlertDialogBody(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogBody { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogHeader(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogHeader { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogTitle(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogTitle { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogDescription(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogDescription { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogFooter(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogFooter { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogClose(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogClose { class: class.unwrap_or_default(), {children} } }
}

#[component]
pub fn AlertDialogAction(
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    rsx! { DialogAction { class: class.unwrap_or_default(), {children} } }
}
