//! Card — shadcn new-york-v4 风格卡片基元（七件套）。
//!
//! 样式不再走 dxc 的 css_module，而是把 shadcn ui/card.tsx 七个导出组件的
//! Tailwind class 逐字写进 `class`；shadcn 源码里的 `data-slot` 原样保留
//! （CardHeader 的 `has-data-[slot=card-action]` variant 依赖它命中）。
//! 调用方传入的 `class` 由 [`with_class`] 追加到组件基串之后，其余属性原样透传。

use dioxus::core::AttributeValue;
use dioxus::prelude::*;

/// shadcn new-york-v4 Card 基础 class（ui/card.tsx:9，逐字）。
const CARD_BASE_CLASS: &str =
    "flex flex-col gap-6 rounded-xl border bg-card py-6 text-card-foreground shadow-sm";

/// shadcn new-york-v4 CardHeader 基础 class（ui/card.tsx:22，逐字）。
const CARD_HEADER_BASE_CLASS: &str = "@container/card-header grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6 has-data-[slot=card-action]:grid-cols-[1fr_auto] [.border-b]:pb-6";

/// shadcn new-york-v4 CardTitle 基础 class（ui/card.tsx:34，逐字）。
const CARD_TITLE_BASE_CLASS: &str = "leading-none font-semibold";

/// shadcn new-york-v4 CardDescription 基础 class（ui/card.tsx:44，逐字）。
const CARD_DESCRIPTION_BASE_CLASS: &str = "text-sm text-muted-foreground";

/// shadcn new-york-v4 CardAction 基础 class（ui/card.tsx:55，逐字）。
const CARD_ACTION_BASE_CLASS: &str =
    "col-start-2 row-span-2 row-start-1 self-start justify-self-end";

/// shadcn new-york-v4 CardContent 基础 class（ui/card.tsx:67，逐字）。
const CARD_CONTENT_BASE_CLASS: &str = "px-6";

/// shadcn new-york-v4 CardFooter 基础 class（ui/card.tsx:77，逐字）。
const CARD_FOOTER_BASE_CLASS: &str = "flex items-center px-6 [.border-t]:pt-6";

/// 把调用方传入的 `class` 追加到组件 shadcn 基串之后，其余属性原样保留。
///
/// 取代 dxc 的 `dioxus_primitives::merge_attributes`：只摘出 `class` 属性做字符串
/// 拼接（先组件基串、后调用方串；Tailwind 语义下拼接顺序不影响命中），非 `class`
/// 属性原样透传，保证调用方传的 class 与其他属性都不丢。
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

/// shadcn new-york-v4 风格卡片容器。
#[component]
pub fn Card(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片头部。
#[component]
pub fn CardHeader(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_HEADER_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-header",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片标题。
#[component]
pub fn CardTitle(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_TITLE_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-title",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片描述文本。
#[component]
pub fn CardDescription(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_DESCRIPTION_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-description",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片右上角动作区。
#[component]
pub fn CardAction(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_ACTION_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-action",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片内容区。
#[component]
pub fn CardContent(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_CONTENT_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-content",
            ..class,
            {children}
        }
    }
}

/// shadcn new-york-v4 风格卡片底部。
#[component]
pub fn CardFooter(
    #[props(extends=GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let class = with_class(attributes, CARD_FOOTER_BASE_CLASS);

    rsx! {
        div {
            "data-slot": "card-footer",
            ..class,
            {children}
        }
    }
}
