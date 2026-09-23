use crate::ui::Button;
use crate::ui::Card;
use crate::ui::CardContent;
use crate::ui::CardGrid;
use crate::ui::CardHeader;
use crate::ui::CardTitle;
use crate::ui::Input;
use crate::ui::Label;
use leptos::prelude::*;

/// 设置页 — 账号与偏好，使用静态数据演示
#[component]
pub fn SettingsPage() -> impl IntoView {
    // 偏好设置状态
    let language = RwSignal::new("zh".to_string());
    let notifications = RwSignal::new(true);
    let theme = RwSignal::new("dark".to_string());
    let compact_mode = RwSignal::new(false);

    // 账号状态
    let display_name = RwSignal::new("Demo User".to_string());
    let email = RwSignal::new("demo@example.com".to_string());
    let current_password = RwSignal::new(String::new());
    let new_password = RwSignal::new(String::new());
    let confirm_password = RwSignal::new(String::new());

    // 状态反馈
    let save_flash = RwSignal::new(None::<String>);
    let save_error = RwSignal::new(String::new());
    let is_saving = RwSignal::new(false);

    let save_preferences = move |_| {
        if is_saving.get() {
            return;
        }
        is_saving.set(true);
        save_flash.set(None);
        save_error.set(String::new());

        // 静态演示：仅更新本地状态
        save_flash.set(Some("偏好设置已保存".into()));
        is_saving.set(false);
    };

    let save_account = move |_| {
        if is_saving.get() {
            return;
        }
        is_saving.set(true);
        save_flash.set(None);
        save_error.set(String::new());

        if !current_password.get().is_empty()
            && (new_password.get() != confirm_password.get() || new_password.get().len() < 8)
        {
            save_error.set("新密码需至少 8 位且两次一致".into());
            is_saving.set(false);
            return;
        }

        // 静态演示：仅更新本地状态
        save_flash.set(Some("账号信息已保存".into()));
        current_password.set(String::new());
        new_password.set(String::new());
        confirm_password.set(String::new());
        is_saving.set(false);
    };

    view! {
        <div class="flex flex-col gap-6">
            <CardGrid>
                // 偏好设置卡片
                <Card class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-6" attr:role="group" attr:aria-label="偏好设置" attr:data-testid="settings-preferences-card">
                    <CardHeader>
                        <CardTitle>"偏好设置"</CardTitle>
                    </CardHeader>
                    <CardContent class="flex flex-col gap-4">
                        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                            <div class="flex flex-col gap-2">
                                <Label label_for="language">"语言"</Label>
                                <select id="language" class="rounded-md border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" prop:value=move || language.get() on:change=move |ev| language.set(event_target_value(&ev))>
                                    <option value="zh">"中文"</option>
                                    <option value="en">"English"</option>
                                    <option value="ja">"日本語"</option>
                                </select>
                            </div>

                            <div class="flex flex-col gap-2">
                                <Label label_for="theme">"主题"</Label>
                                <select id="theme" class="rounded-md border border-zinc-700 bg-zinc-900 px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500" prop:value=move || theme.get() on:change=move |ev| theme.set(event_target_value(&ev))>
                                    <option value="dark">"深色"</option>
                                    <option value="light">"浅色"</option>
                                    <option value="system">"跟随系统"</option>
                                </select>
                            </div>

                            <div class="flex items-center gap-3">
                                <input type="checkbox" id="notifications" class="h-4 w-4 rounded border-zinc-700 bg-zinc-900 text-blue-600 focus:ring-blue-500" prop:checked=move || notifications.get() on:change=move |ev| notifications.set(event_target_checked(&ev)) />
                                <Label label_for="notifications" class="cursor-pointer">"启用通知"</Label>
                            </div>

                            <div class="flex items-center gap-3">
                                <input type="checkbox" id="compact_mode" class="h-4 w-4 rounded border-zinc-700 bg-zinc-900 text-blue-600 focus:ring-blue-500" prop:checked=move || compact_mode.get() on:change=move |ev| compact_mode.set(event_target_checked(&ev)) />
                                <Label label_for="compact_mode" class="cursor-pointer">"紧凑模式"</Label>
                            </div>
                        </div>

                        <div class="flex items-center gap-3 pt-2 border-t border-zinc-800">
                            <Button
                                variant="primary"
                                button_type="button"
                                on:click=save_preferences
                                disabled=is_saving
                            >
                                {move || if is_saving.get() { "保存中..." } else { "保存偏好设置" }}
                            </Button>
                        </div>
                    </CardContent>
                </Card>

                // 账号与密码卡片
                <Card class="rounded-xl border border-zinc-800 bg-zinc-900/60 p-6" attr:role="group" attr:aria-label="账号与密码" attr:data-testid="settings-account-card">
                    <CardHeader>
                        <CardTitle>"账号与密码"</CardTitle>
                    </CardHeader>
                    <CardContent class="flex flex-col gap-4">
                        <div class="grid grid-cols-1 gap-4 sm:grid-cols-2">
                            <div class="flex flex-col gap-2">
                                <Label label_for="display_name">"显示名"</Label>
                                <Input
                                    id="display_name"
                                    input_type="text"
                                    placeholder="请输入显示名"
                                    prop:value=move || display_name.get()
                                    on:input=move |ev| display_name.set(event_target_value(&ev))
                                />
                            </div>

                            <div class="flex flex-col gap-2">
                                <Label label_for="email">"邮箱"</Label>
                                <Input
                                    id="email"
                                    input_type="email"
                                    placeholder="请输入邮箱"
                                    prop:value=move || email.get()
                                    on:input=move |ev| email.set(event_target_value(&ev))
                                />
                            </div>
                        </div>

                        <div class="pt-2 border-t border-zinc-800">
                            <h4 class="mb-3 text-sm font-medium text-zinc-300">"修改密码"</h4>
                            <div class="grid grid-cols-1 gap-4 sm:grid-cols-3">
                                <div class="flex flex-col gap-2">
                                    <Label label_for="current_password">"当前密码"</Label>
                                    <Input
                                        id="current_password"
                                        input_type="password"
                                        placeholder="输入当前密码以确认"
                                        prop:value=move || current_password.get()
                                        on:input=move |ev| current_password.set(event_target_value(&ev))
                                    />
                                </div>

                                <div class="flex flex-col gap-2">
                                    <Label label_for="new_password">"新密码"</Label>
                                    <Input
                                        id="new_password"
                                        input_type="password"
                                        placeholder="至少 8 位"
                                        prop:value=move || new_password.get()
                                        on:input=move |ev| new_password.set(event_target_value(&ev))
                                    />
                                </div>

                                <div class="flex flex-col gap-2">
                                    <Label label_for="confirm_password">"确认新密码"</Label>
                                    <Input
                                        id="confirm_password"
                                        input_type="password"
                                        placeholder="再次输入新密码"
                                        prop:value=move || confirm_password.get()
                                        on:input=move |ev| confirm_password.set(event_target_value(&ev))
                                    />
                                </div>
                            </div>
                        </div>

                        <div class="flex items-center gap-3 pt-2 border-t border-zinc-800">
                            <Button
                                variant="primary"
                                button_type="button"
                                on:click=save_account
                                disabled=is_saving
                            >
                                {move || if is_saving.get() { "保存中..." } else { "保存账号信息" }}
                            </Button>
                        </div>
                    </CardContent>
                </Card>
            </CardGrid>

            // 保存反馈
            {move || save_flash.get().map(|msg| view! {
                <div class="rounded-md bg-green-900/30 border border-green-700 text-green-300 px-4 py-3 text-sm" role="alert">{msg}</div>
            })}
            {move || (!save_error.get().is_empty()).then(|| view! {
                <div class="rounded-md bg-red-900/30 border border-red-700 text-red-300 px-4 py-3 text-sm" role="alert">{save_error.get()}</div>
            })}
        </div>
    }
}
