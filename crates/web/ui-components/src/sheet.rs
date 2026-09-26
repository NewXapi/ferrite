//! Sheet — shadcn new-york-v4 风格抽屉（受控扁平 API）。
//!
//! class 契约逐字取自 [shadcn ui/sheet.tsx]（data-slot / data-state 属性照抄）；
//! Radix 的 Root/Trigger/Close/Portal 组合收敛成单个受控组件——`open` +
//! `on_open_change` + `side` + `title`/`description`/`footer` 插槽。
//!
//! 与上游的机制性差异（均不引入新依赖）：
//! - **动画**：tw-animate-css 的 `data-[state=open]:animate-in ... slide-in-from-*`
//!   段换成 transform class 直写——Content **常驻渲染**，关闭态平移出视口、开启态
//!   归位，过渡由 Content 自身的 `transition ease-in-out duration-300` 承担（见
//!   [`side_parts`]）；Overlay 无退场动画，`open` 才渲染。
//! - **Escape 关闭**：挂载时用 `document::eval` 给 `window` 挂 keydown 监听（与
//!   `dropdown_menu` 的 guard 模式一致），JS 侧按 Content 根节点 `data-state=open`
//!   判定后再回传关闭，卸载时 `use_drop` 拆掉监听。已知取舍：监听捕获首次渲染的
//!   `on_open_change`，回调只应捕获 Signal 等 Copy 稳定状态（Signal 跨渲染稳定）。
//! - **id**：Content 根元素 id 由组件分配（`dioxus-sheet-N`，Escape 判断锚点），
//!   调用方勿传 `id`。
//! - **可达性**：关闭态 Content 仍挂在 DOM（仅平移出视口，未加 inert），Tab 焦点
//!   仍可能落到关态内容；Overlay 充当点击关闭层。role="dialog" + aria-modal +
//!   title/description 的 labelledby/describedby 已照 Radix 语义补齐。
//!
//! [shadcn ui/sheet.tsx]: https://github.com/shadcn-ui/ui

use std::sync::atomic::{AtomicUsize, Ordering};

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// Sheet 抽屉弹出方向（shadcn `side` prop；上游默认 right，本 API 必填无默认）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetSide {
    /// 顶部滑入（`inset-x-0 top-0 h-auto border-b`）。
    Top,
    /// 右侧滑入（shadcn 默认方向）。
    Right,
    /// 底部滑入（`inset-x-0 bottom-0 h-auto border-t`）。
    Bottom,
    /// 左侧滑入。
    Left,
}

/// shadcn ui/sheet.tsx:38 SheetOverlay class（去掉 data-[state] 动画段——Overlay
/// open 才渲染，无退场过程可动画）。
const OVERLAY_CLASS: &str = "fixed inset-0 z-50 bg-black/50";

/// shadcn ui/sheet.tsx:62 SheetContent class（`data-[state=closed]:duration-300` /
/// `data-[state=open]:duration-500` 的 tw-animate 段收敛为 `duration-300`，side
/// 定位与开合位移由 [`side_parts`] 追加）。
const CONTENT_BASE_CLASS: &str = "fixed z-50 flex flex-col gap-4 bg-background p-6 shadow-lg transition ease-in-out duration-300";

/// shadcn ui/sheet.tsx:91 SheetHeader class（逐字）。
const HEADER_CLASS: &str = "flex flex-col gap-1.5 p-4";

/// shadcn ui/sheet.tsx:101 SheetFooter class（逐字）。
const FOOTER_CLASS: &str = "mt-auto flex flex-col gap-2 p-4";

/// shadcn ui/sheet.tsx:114 SheetTitle class（逐字）。
const TITLE_CLASS: &str = "font-semibold text-foreground";

/// shadcn ui/sheet.tsx:127 SheetDescription class（逐字）。
const DESCRIPTION_CLASS: &str = "text-sm text-muted-foreground";

/// shadcn ui/sheet.tsx:77 内置 Close 按钮的 class（逐字；`data-[state=open]` 命中
/// 依赖按钮自身的 data-state 属性，跟随 Content 状态）。
const CLOSE_CLASS: &str = "absolute top-4 right-4 rounded-xs opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:ring-2 focus:ring-ring focus:ring-offset-2 focus:outline-hidden data-[state=open]:bg-secondary";

/// side → (定位 class, 关态位移 class, 开态 class)。
///
/// 定位段取自 shadcn ui/sheet.tsx SheetContent 的 side 分支（逐字）；关态位移
/// class 替代上游 `data-[state=closed]:slide-out-to-*` / `slide-in-from-*` 动画段
/// （tw-animate-css 不可用，改为常驻元素的 transform 直写，开态归位
/// `translate-*-0`）。抽成纯函数供契约测试锁定视觉契约。
pub fn side_parts(side: SheetSide) -> (&'static str, &'static str, &'static str) {
    match side {
        SheetSide::Top => (
            "inset-x-0 top-0 h-auto border-b",
            "-translate-y-full",
            "translate-y-0",
        ),
        SheetSide::Right => (
            "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm",
            "translate-x-full",
            "translate-x-0",
        ),
        SheetSide::Bottom => (
            "inset-x-0 bottom-0 h-auto border-t",
            "translate-y-full",
            "translate-y-0",
        ),
        SheetSide::Left => (
            "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm",
            "-translate-x-full",
            "translate-x-0",
        ),
    }
}

/// 把调用方传入的 `class` 追加到组件 shadcn 基串之后，其余属性原样保留。
///
/// 与 button/badge 的 `with_class` 同签名同语义（各文件私有副本，见 button 的注释）。
fn with_class(attributes: Vec<Attribute>, extra: &str) -> Vec<Attribute> {
    let mut caller_class = String::new();
    let mut rest = Vec::with_capacity(attributes.len() + 1);
    for attribute in attributes {
        match (&attribute.value, attribute.name) {
            (AttributeValue::Text(value), "class") if !value.trim().is_empty() => {
                if !caller_class.is_empty() {
                    caller_class.push(' ');
                }
                caller_class.push_str(value.trim());
            }
            _ => rest.push(attribute),
        }
    }
    let class = if caller_class.is_empty() {
        extra.to_string()
    } else {
        format!("{extra} {caller_class}")
    };
    rest.push(Attribute::new("class", class, None, false));
    rest
}

/// shadcn new-york-v4 风格抽屉（受控扁平 API）。
///
/// ```ignore
/// let mut open = use_signal(|| false);
/// Sheet {
///     open: open(),
///     on_open_change: move |v| open.set(v),
///     side: SheetSide::Right,
///     title: Some("编辑资料".into()),
///     description: Some("修改后点击保存".into()),
///     footer: Some(rsx! { Button { "保存" } }),
///     p { "表单内容放 children" }
/// }
/// ```
///
/// 关闭路径（Radix 等价行为自研）：Overlay 点击、内置 Close 按钮、Escape 键，
/// 三者均回调 `on_open_change(false)`。
#[component]
pub fn Sheet(
    /// 受控开合：true 渲染 Overlay 并把 Content 平移回视口，false 仅保留滑出的
    /// Content（常驻渲染以承载 transition）。
    open: bool,
    /// 开合变化回调：Overlay 点击 / Close 按钮 / Escape 都回调 `false`。
    on_open_change: EventHandler<bool>,
    /// 弹出方向，决定 Content 的定位与位移 class（见 [`side_parts`]）。
    side: SheetSide,
    /// 标题文本（`h2.font-semibold.text-foreground`）；与 description 都为 None
    /// 时不渲染 SheetHeader。
    #[props(default)]
    title: Option<String>,
    /// 描述文本（`p.text-sm.text-muted-foreground`），渲染在标题下方。
    #[props(default)]
    description: Option<String>,
    /// 底部插槽（SheetFooter 容器，`mt-auto` 推到底部），None 不渲染。
    #[props(default)]
    footer: Option<Element>,
    /// 追加到 Content class 之后的调用方 class（宽度、内边距等定制）。
    #[props(default)]
    content_class: Option<String>,
    /// 透传到 Content 根元素的属性；勿传 `id`（由组件分配，Escape 判断依赖它）。
    #[props(extends = GlobalAttributes)]
    attributes: Vec<Attribute>,
    /// 抽屉主体内容：渲染在 SheetHeader 之后、SheetFooter 之前。
    children: Element,
) -> Element {
    // Content 根唯一 id：既是 Escape JS 判断的锚点，也是监听守卫的命名空间；
    // 实例间互不干扰靠自增序号（wasm 单线程，Relaxed 足够）。
    let root_id = use_hook(|| {
        static SEQ: AtomicUsize = AtomicUsize::new(0);
        format!("dioxus-sheet-{}", SEQ.fetch_add(1, Ordering::Relaxed))
    });

    // document keydown 监听（挂载一次）：仅当本实例 Content 处于 open 态时，
    // Escape 回传关闭；eval 通道随组件卸载而死，recv 出错即退出循环，监听本体
    // 由 use_drop 拆除（与 dropdown_menu 的 guard 收尾模式一致）。
    use_hook({
        let root_id = root_id.clone();
        move || {
            let guard = format!("__dioxusSheetEscape_{}", root_id.replace('-', "_"));
            let js = format!(
                r#"
                {{
                    if (window.{guard} === undefined) {{
                        const fn = (event) => {{
                            if (event.key !== 'Escape') {{ return; }}
                            const root = document.getElementById('{root_id}');
                            if (root && root.getAttribute('data-state') === 'open') {{
                                dioxus.send(1);
                            }}
                        }};
                        window.addEventListener('keydown', fn);
                        window.{guard} = fn;
                    }}
                }}
                "#
            );
            spawn(async move {
                let mut ev = document::eval(&js);
                // 任何回传都代表「open 态下按了 Escape」。
                while let Ok(_pressed) = ev.recv::<f64>().await {
                    on_open_change.call(false);
                }
            });
        }
    });

    // 卸载时拆掉 JS 侧监听，防止对已死通道持续 dioxus.send（同 scroll_spy 的收尾约定）。
    use_drop({
        let guard = format!("__dioxusSheetEscape_{}", root_id.replace('-', "_"));
        move || {
            let _ = document::eval(&format!(
                r#"
                if (window.{guard}) {{
                    window.removeEventListener('keydown', window.{guard});
                    delete window.{guard};
                }}
                "#
            ));
        }
    });

    // Content class：shadcn 基串 + side 定位 + 开合位移 + 调用方定制。
    let (position, closed_shift, open_shift) = side_parts(side);
    let state = if open { "open" } else { "closed" };
    let state_class = if open { open_shift } else { closed_shift };
    let mut content_class_full = format!("{CONTENT_BASE_CLASS} {position} {state_class}");
    if let Some(extra) = content_class
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
    {
        content_class_full.push(' ');
        content_class_full.push_str(extra);
    }
    let attrs = with_class(attributes, &content_class_full);

    // 内置 Close 按钮：内联 lucide X 图标（shadcn 用 lucide-react 的 XIcon）。
    let close_button = rsx! {
        button {
            class: CLOSE_CLASS,
            "data-slot": "sheet-close",
            "data-state": "{state}",
            "aria-label": "Close",
            onclick: move |_| on_open_change.call(false),
            svg {
                class: "size-4",
                "viewBox": "0 0 24 24",
                fill: "none",
                stroke: "currentColor",
                "stroke-width": "2",
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                "aria-hidden": "true",
                path { d: "M18 6 6 18" }
                path { d: "m6 6 12 12" }
            }
            span { class: "sr-only", "Close" }
        }
    };

    // title/description 存在才渲染 SheetHeader（Radix 语义：h2 标题 + p 描述）。
    let has_header = title.is_some() || description.is_some();

    rsx! {
        if open {
            div {
                class: OVERLAY_CLASS,
                "data-slot": "sheet-overlay",
                "data-state": "open",
                "aria-hidden": "true",
                onclick: move |_| on_open_change.call(false),
            }
        }
        div {
            "data-slot": "sheet-content",
            "data-state": "{state}",
            role: "dialog",
            "aria-modal": "true",
            "aria-labelledby": title.as_ref().map(|_| format!("{root_id}-title")),
            "aria-describedby": description.as_ref().map(|_| format!("{root_id}-description")),
            id: "{root_id}",
            ..attrs,
            if has_header {
                div {
                    class: HEADER_CLASS,
                    "data-slot": "sheet-header",
                    if let Some(text) = &title {
                        h2 {
                            id: "{root_id}-title",
                            class: TITLE_CLASS,
                            "data-slot": "sheet-title",
                            "{text}"
                        }
                    }
                    if let Some(text) = &description {
                        p {
                            id: "{root_id}-description",
                            class: DESCRIPTION_CLASS,
                            "data-slot": "sheet-description",
                            "{text}"
                        }
                    }
                }
            }
            {children}
            if let Some(footer) = footer {
                div {
                    class: FOOTER_CLASS,
                    "data-slot": "sheet-footer",
                    {footer}
                }
            }
            {close_button}
        }
    }
}
