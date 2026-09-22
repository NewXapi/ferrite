use leptos::prelude::*;
use singlestage::*;
use crate::ui::CardGrid;
use crate::ui::components::button::ButtonVariant;
use serde_json::json;

/// 系统设置页 — 基于 dioxus 源码里的静态数据
/// 使用 Card + CardHeader + CardTitle + CardContent, CardGrid 包装列表
/// Dialog 使用 RwSignal 控制 open, button_type="button"
#[component]
pub fn SystemPage() -> impl IntoView {
    let mut dialog_open = RwSignal::new(false);

    let static_data = RwSignal::new(json!({
        "title": "系统设置",
        "version": "v2.4.1",
        "env": "production",
        "uptime": "17天 4小时",
        "users": 1248,
        "models": 42,
        "tokens": 987654,
        "options": {
            "debug_mode": false,
            "log_level": "info",
            "max_connections": 5000
        }
    }));

    view! {
        <div class="space-y-6 p-6">
            <h1 class="text-2xl font-semibold">"系统设置"</h1>

            <CardGrid>
                <Card>
                    <CardHeader>
                        <CardTitle>"系统概览"</CardTitle>
                    </CardHeader>
                    <CardContent class="space-y-4">
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <p class="text-sm text-zinc-500">"版本"</p>
                                <p class="font-mono text-lg">{move || static_data().get("version").and_then(|v| v.as_str()).unwrap_or_default().to_string()}</p>
                            </div>
                            <div>
                                <p class="text-sm text-zinc-500">"环境"</p>
                                <p class="font-medium">{move || static_data().get("env").and_then(|v| v.as_str()).unwrap_or_default().to_string()}</p>
                            </div>
                            <div>
                                <p class="text-sm text-zinc-500">"运行时间"</p>
                                <p class="font-medium">{move || static_data().get("uptime").and_then(|v| v.as_str()).unwrap_or_default().to_string()}</p>
                            </div>
                            <div>
                                <p class="text-sm text-zinc-500">"注册用户"</p>
                                <p class="font-medium">{move || static_data().get("users").and_then(|v| v.as_i64()).unwrap_or(0)}</p>
                            </div>
                        </div>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle>"资源统计"</CardTitle>
                    </CardHeader>
                    <CardContent class="space-y-4">
                        <div class="grid grid-cols-2 gap-4">
                            <div>
                                <p class="text-sm text-zinc-500">"模型数量"</p>
                                <p class="text-2xl font-bold text-blue-600">{move || static_data().get("models").and_then(|v| v.as_i64()).unwrap_or(0)}</p>
                            </div>
                            <div>
                                <p class="text-sm text-zinc-500">Token "总数"</p>
                                <p class="text-2xl font-bold text-emerald-600">{move || static_data().get("tokens").and_then(|v| v.as_i64()).unwrap_or(0)}</p>
                            </div>
                        </div>
                    </CardContent>
                </Card>

                <Card>
                    <CardHeader>
                        <CardTitle>"配置选项"</CardTitle>
                    </CardHeader>
                    <CardContent>
                        <div class="space-y-3 text-sm">
                            <div class="flex justify-between">
                                <span class="text-zinc-500">"调试模式"</span>
                                <span class="font-medium">{"关闭"}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-zinc-500">"日志级别"</span>
                                <span class="font-mono">{"info"}</span>
                            </div>
                            <div class="flex justify-between">
                                <span class="text-zinc-500">"最大连接数"</span>
                                <span class="font-medium">{"5000"}</span>
                            </div>
                        </div>
                    </CardContent>
                </Card>
            </CardGrid>

            <div class="flex gap-3">
                <Button
                    button_type="button"
                    variant=ButtonVariant::Default
                    on_click=move |_| { dialog_open.set(true); }
                >
                    "运行诊断"
                </Button>
                <Button
                    button_type="button"
                    variant=ButtonVariant::Outline
                >
                    "刷新配置"
                </Button>
            </div>

            <Dialog open=dialog_open on_open_change=move |open| dialog_open.set(open)>
                <DialogTrigger>
                    <span class="hidden">"trigger"</span>
                </DialogTrigger>
                <DialogContent>
                    <DialogHeader>
                        <DialogTitle>"系统诊断工具"</DialogTitle>
                        <DialogDescription>
                            "当前系统状态摘要（静态演示数据）"
                        </DialogDescription>
                    </DialogHeader>
                    <div class="py-4 text-sm space-y-2">
                        <p>"CPU 负载: 34%"</p>
                        <p>"内存占用: 2.1GB / 15.6GB"</p>
                        <p>"数据库连接: 正常 (42/50)"</p>
                        <p>"最后健康检查: 刚刚"</p>
                    </div>
                    <DialogFooter>
                        <Button
                            button_type="button"
                            on_click=move |_| dialog_open.set(false)
                            variant=ButtonVariant::Outline
                        >
                            "关闭"
                        </Button>
                    </DialogFooter>
                </DialogContent>
            </Dialog>
        </div>
    }
}
