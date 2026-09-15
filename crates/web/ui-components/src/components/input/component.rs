//! Input — dsh（deepseek-harness）风格输入框基元。
//!
//! 视觉基准已由 shadcn new-york-v4 逐字串切换为 dsh 组件 CSS：几何与状态取自
//! dsh `packages/client/ui-*/src/Pill.module.css:5-14`（1px border-l2 边框、r8、
//! focus-within 仅边框变色无 ring 无 shadow :12-14），映射与决策见
//! `todo/web-ui-reference/dsh-visual-spec.md` §3.2/§4.3。高度保留 h-9 36px（spec
//! 建议的 h-8 需 Select 系同步降高，留后续 PR，D12 改判）。
//! 输入相关的全部事件 handler 与其余属性原样透传给底层 `<input>`。

use dioxus::prelude::*;

/// dsh 基准的 Input 基础 class（Pill.module.css:5-14；高度保留 h-9，见模块注释）。
///
/// 相比 shadcn ui/input.tsx:10-12 的改动：bg-input/30 常驻（去 dark: 前缀，D13）、
/// shadow-xs→shadow-none（dsh 无阴影）、text-base md:text-sm 合并为单段 text-sm
/// （fs14，iOS 防缩放对 wasm/webview 无意义）、transition-[color,box-shadow]→
/// transition-[color,border-color]、focus-visible 删 ring 三段只留 border 变色（D15，
/// 对应 dsh focus-within :12-14）、aria-invalid 删 ring 段、disabled:opacity-40（dsh
/// 全组件统一 0.4）；file:/selection 段保留 shadcn 原样（dsh 无对应）。
const INPUT_BASE_CLASS: &str = "h-9 w-full min-w-0 rounded-md border border-input bg-input/30 px-3 py-1 text-sm shadow-none transition-[color,border-color] outline-none selection:bg-primary selection:text-primary-foreground file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-40 focus-visible:border-ring aria-invalid:border-destructive";

/// dsh 风格输入框：事件 handler 与属性全部原样透传。
#[component]
pub fn Input(
    oninput: Option<EventHandler<FormEvent>>,
    onchange: Option<EventHandler<FormEvent>>,
    oninvalid: Option<EventHandler<FormEvent>>,
    onselect: Option<EventHandler<SelectionEvent>>,
    onselectionchange: Option<EventHandler<SelectionEvent>>,
    onfocus: Option<EventHandler<FocusEvent>>,
    onblur: Option<EventHandler<FocusEvent>>,
    onfocusin: Option<EventHandler<FocusEvent>>,
    onfocusout: Option<EventHandler<FocusEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    onkeypress: Option<EventHandler<KeyboardEvent>>,
    onkeyup: Option<EventHandler<KeyboardEvent>>,
    onwheel: Option<EventHandler<WheelEvent>>,
    oncompositionstart: Option<EventHandler<CompositionEvent>>,
    oncompositionupdate: Option<EventHandler<CompositionEvent>>,
    oncompositionend: Option<EventHandler<CompositionEvent>>,
    oncopy: Option<EventHandler<ClipboardEvent>>,
    oncut: Option<EventHandler<ClipboardEvent>>,
    onpaste: Option<EventHandler<ClipboardEvent>>,
    #[props(extends=GlobalAttributes)]
    #[props(extends=input)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    rsx! {
        input {
            class: INPUT_BASE_CLASS,
            "data-slot": "input",
            oninput: move |e| _ = oninput.map(|callback| callback(e)),
            onchange: move |e| _ = onchange.map(|callback| callback(e)),
            oninvalid: move |e| _ = oninvalid.map(|callback| callback(e)),
            onselect: move |e| _ = onselect.map(|callback| callback(e)),
            onselectionchange: move |e| _ = onselectionchange.map(|callback| callback(e)),
            onfocus: move |e| _ = onfocus.map(|callback| callback(e)),
            onblur: move |e| _ = onblur.map(|callback| callback(e)),
            onfocusin: move |e| _ = onfocusin.map(|callback| callback(e)),
            onfocusout: move |e| _ = onfocusout.map(|callback| callback(e)),
            onkeydown: move |e| _ = onkeydown.map(|callback| callback(e)),
            onkeypress: move |e| _ = onkeypress.map(|callback| callback(e)),
            onkeyup: move |e| _ = onkeyup.map(|callback| callback(e)),
            onwheel: move |e| _ = onwheel.map(|callback| callback(e)),
            oncompositionstart: move |e| _ = oncompositionstart.map(|callback| callback(e)),
            oncompositionupdate: move |e| _ = oncompositionupdate.map(|callback| callback(e)),
            oncompositionend: move |e| _ = oncompositionend.map(|callback| callback(e)),
            oncopy: move |e| _ = oncopy.map(|callback| callback(e)),
            oncut: move |e| _ = oncut.map(|callback| callback(e)),
            onpaste: move |e| _ = onpaste.map(|callback| callback(e)),
            ..attributes,
            {children}
        }
    }
}
