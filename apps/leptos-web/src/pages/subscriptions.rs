use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;

/// 订阅管理页面
/// 简化版：4个预设订阅套餐展示，新建套餐弹窗
#[component]
pub fn SubscriptionsPage() -> impl IntoView {
    let mut show_modal = use_signal(|| false);

    // 预设的 4 个订阅套餐数据
    const PLANS: &[(&str, &str, &str)] = &[
        ("开拓的封赏", "29.00 CNY/月", "启用"),
        ("先锋的勋章", "59.00 CNY/月", "启用"),
        ("领航的旗帜", "99.00 CNY/月", "启用"),
        ("传奇的王冠", "199.00 CNY/月", "禁用"),
    ];

    view! {
        <div class="space-y-6">
            <div class="flex justify-between items-center">
                <h1 class="text-2xl font-bold">套餐管理</h1>
                <button
                    type="button"
                    class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
                    on:click=move |_| show_modal.set(true)
                >
                    新建套餐
                </button>
            </div>

            <CardGrid>
                {PLANS.iter().enumerate().map(|(idx, (name, price, status))| {
                    let status_class = if *status == "启用" { "bg-green-100 text-green-800" } else { "bg-red-100 text-red-800" };
                    view! {
                        <Card class="p-4">
                            <h3 class="text-lg font-semibold mb-2">{name}</h3>
                            <p class="text-gray-600 mb-2">{price}</p>
                            <span class="inline-block px-2 py-1 text-xs rounded-full">{status_class} {status}</span>
                        </Card>
                    }
                }).collect_view()}
            </CardGrid>

            <Dialog open=show_modal() on_open_change=move |v| show_modal.set(v)>
                <DialogTrigger as std::fmt::Display = "button" button_type="button">
                    <span class="hidden">Dialog Trigger</span>
                </DialogTrigger>
                <DialogContent class="sm:max-w-2xl">
                    <DialogHeader>
                        <DialogTitle>"新建订阅套餐"</DialogTitle>
                        <DialogDescription>"创建一个新的订阅套餐。填写基本信息和规则。"</DialogDescription>
                    </DialogHeader>

                    <div class="grid gap-4 py-4">
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label class="text-right">"套餐标题"</Label>
                            <div class="col-span-3">
                                <Input placeholder="例如：开拓的封赏" />
                            </div>
                        </div>
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label class="text-right">"价格"</Label>
                            <div class="col-span-3">
                                <Input placeholder="0" />
                            </div>
                        </div>
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label class="text-right">"币种"</Label>
                            <div class="col-span-3">
                                <Select>
                                    <SelectTrigger>
                                        <SelectValue placeholder="选择币种" />
                                    </SelectTrigger>
                                    <SelectContent>
                                        <SelectItem value="CNY">"CNY"</SelectItem>
                                        <SelectItem value="USD">"USD"</SelectItem>
                                        <SelectItem value="EUR">"EUR"</SelectItem>
                                    </SelectContent>
                                </Select>
                            </div>
                        </div>
                        <div class="grid grid-cols-4 items-center gap-4">
                            <Label class="text-right">"额度"</Label>
                            <div class="col-span-3">
                                <Input placeholder="0" />
                            </div>
                        </div>
                    </div>

                    <DialogFooter>
                        <Button type="button" button_type="button" on:click=move |_| show_modal.set(false)>"取消"</Button>
                        <Button type="button" button_type="button" class="ml-2" on:click=move |_| show_modal.set(false)>"保存"</Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    }
}