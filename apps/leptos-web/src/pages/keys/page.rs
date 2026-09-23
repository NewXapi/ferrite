use crate::ui::{Button, CardGrid, Dialog, DialogTrigger};
use leptos::prelude::*;

use super::card::KeyCard;
use super::data::demo_keys;

#[component]
pub fn KeysPage() -> impl IntoView {
    let dialog_open = RwSignal::new(false);
    view! {
        <div class="space-y-8 p-6">
            // 密钥卡片网格
            <CardGrid>
                {demo_keys().into_iter().map(|k| view! { <KeyCard entry=k/> }).collect_view()}
            </CardGrid>

            <Dialog open=dialog_open title="新建密钥".to_string()>
                <DialogTrigger slot>
                    <span class="hidden">"新建密钥"</span>
                </DialogTrigger>
                <div class="space-y-4">
                    <div>
                        <label class="block text-sm font-medium text-zinc-300 mb-1">"名称"</label>
                        <input class="w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm" placeholder="输入密钥名称"/>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-zinc-300 mb-1">"分组"</label>
                        <input class="w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm" placeholder="输入分组名称"/>
                    </div>
                    <div>
                        <label class="block text-sm font-medium text-zinc-300 mb-1">"额度"</label>
                        <input class="w-full px-3 py-2 bg-zinc-800 rounded border border-zinc-700 text-sm" placeholder="输入额度（留空为无限）"/>
                    </div>
                    <div class="flex gap-2 justify-end">
                        <Button button_type="button" variant="outline">"取消"</Button>
                        <Button button_type="button" variant="primary">"创建"</Button>
                    </div>
                </div>
            </Dialog>
        </div>
    }
}
