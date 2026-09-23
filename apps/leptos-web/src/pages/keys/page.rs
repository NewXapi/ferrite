use crate::ui::{Card, CardContent, CardGrid, CardHeader, CardTitle, Dialog, DialogTrigger};
use leptos::prelude::*;
use singlestage::*;

#[derive(Clone, Copy)]
struct KeyItem {
    name: &'static str,
    key_preview: &'static str,
    status: i32,
    unlimited_quota: bool,
    used_quota: i64,
    quota: i64,
    created_at: &'static str,
}

use super::card::KeyCard;
use super::data::*;

#[component]
pub fn KeysPage() -> impl IntoView {
    let mut dialog_open = RwSignal::new(false);

    view! {
        div { class: "space-y-8 p-6",
            // 密钥卡片网格
            CardGrid { aria_label: "密钥列表".to_string(),
                {demo_keys().into_iter().map(|k| view! { KeyCard(entry=k) }).collect::<Vec<_>>()}
            }

            Dialog {
                dialog_trigger: DialogTrigger::None,
                open: dialog_open,
                title: "新建密钥".to_string(),
                div { class: "space-y-4",
                    div {
                        label { class: "block text-sm font-medium text-zinc-300 mb-1", "名称" }
                        input { class: "w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm", placeholder: "输入密钥名称" }
                    }
                    div {
                        label { class: "block text-sm font-medium text-zinc-300 mb-1", "分组" }
                        input { class: "w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm", placeholder: "输入分组名称" }
                    }
                    div {
                        label { class: "block text-sm font-medium text-zinc-300 mb-1", "额度" }
                        input { class: "w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm", placeholder: "输入额度（留空为无限）" }
                    }
                    div { class: "flex gap-2 justify-end",
                        Button { button_type: "button", variant: ButtonVariant::Secondary, "取消" }
                        Button { button_type: "button", variant: ButtonVariant::Primary, "创建" }
                    }
                }
            }
        }
    }
}
