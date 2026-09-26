//! rust-ui Input 原语拷贝（github.com/rust-ui/ui registry）。
//! 仅去掉上游 strum 依赖（`IntoStaticStr` → 手写 `as_str` match），其余原样。
use dioxus::prelude::*;
use tw_merge::tw_merge;

#[allow(dead_code)] // registry 原版:18 个 type 变体按需使用,未用的保留枚举完整性
#[derive(Default, Clone, PartialEq, Eq)]
pub enum InputType {
    #[default]
    Text,
    Email,
    Password,
    Number,
    Tel,
    Url,
    Search,
    Time,
    DatetimeLocal,
    Date,
    Month,
    Week,
    Hidden,
    File,
    Checkbox,
    Radio,
    Color,
    Range,
}

impl InputType {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Email => "email",
            Self::Password => "password",
            Self::Number => "number",
            Self::Tel => "tel",
            Self::Url => "url",
            Self::Search => "search",
            Self::Time => "time",
            Self::DatetimeLocal => "datetime-local",
            Self::Date => "date",
            Self::Month => "month",
            Self::Week => "week",
            Self::Hidden => "hidden",
            Self::File => "file",
            Self::Checkbox => "checkbox",
            Self::Radio => "radio",
            Self::Color => "color",
            Self::Range => "range",
        }
    }
}

#[component]
pub fn Input(
    #[props(into, optional)] class: Option<String>,
    #[props(default = InputType::default())] r#type: InputType,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(into, optional)] name: Option<String>,
    #[props(into, optional)] id: Option<String>,
    #[props(into, optional)] value: Option<String>,
    #[props(optional)] disabled: bool,
    #[props(optional)] readonly: bool,
    #[props(optional)] required: bool,
    #[props(into, optional)] oninput: Option<EventHandler<FormEvent>>,
) -> Element {
    let merged_class = tw_merge!(
        "text-foreground placeholder:text-zinc-500 w-full min-w-0 rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm transition-colors outline-none disabled:cursor-not-allowed disabled:opacity-40 focus:border-zinc-500",
        class.as_deref().unwrap_or("")
    );

    rsx! {
        input {
            "data-name": "Input",
            r#type: r#type.as_str(),
            class: "{merged_class}",
            placeholder: placeholder,
            value: value,
            name: name,
            id: id,
            disabled: disabled,
            readonly: readonly,
            required: required,
            oninput: move |e| {
                if let Some(handler) = &oninput {
                    handler.call(e);
                }
            },
        }
    }
}
