use leptos::prelude::*;
use singlestage::*;

use crate::ui::CardGrid;

use super::card::UsageCard;
use super::data::{UsageLog, demo_usage_logs, fmt_num};

/// 用量日志页。统计和列表用演示数据，点卡片打开详情。
#[component]
pub fn UsagePage() -> impl IntoView {
    let logs = demo_usage_logs();
    let requests = logs.len();
    let quota: i64 = logs.iter().map(|l| l.quota).sum();
    let open = RwSignal::new(false);
    let selected = RwSignal::new(None::<UsageLog>);

    view! {
        <div class="flex flex-col gap-6">
            <section class="space-y-3">
                <h2 class="text-lg font-medium text-zinc-100">"用量统计"</h2>
                <div class="grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5">
                    <Stat label="今日请求" value=requests.to_string()/>
                    <Stat label="今日消耗" value=quota.to_string()/>
                    <Stat label="RPM" value="0".to_string()/>
                    <Stat label="TPM" value="0".to_string()/>
                    <Stat label="成功率" value="—".to_string()/>
                </div>
            </section>

            <section class="space-y-3">
                <h2 class="text-lg font-medium text-zinc-100">"请求日志"</h2>
                <CardGrid>
                    {logs.into_iter().map(|log| {
                        let pick = log;
                        view! {
                            <button type="button" class="text-left"
                                on:click=move |_| { selected.set(Some(pick)); open.set(true); }>
                                <UsageCard entry=pick/>
                            </button>
                        }
                    }).collect_view()}
                </CardGrid>
            </section>

            <Dialog
                open=open
                title="日志详情".to_string()
            >
                <DialogTrigger slot>
                    <span class="hidden">"日志详情"</span>
                </DialogTrigger>
                {move || selected.get().map(|log| view! {
                    <div class="space-y-3">
                        <Row k="模型" v=log.model_name.to_string()/>
                        <Row k="时间" v=log.created_at.to_string()/>
                        <Row k="密钥" v=log.token_name.to_string()/>
                        <Row k="渠道" v=log.channel_name.to_string()/>
                        <Row k="Tokens" v=format!("{} / {}", fmt_num(log.prompt_tokens), fmt_num(log.completion_tokens))/>
                        <Row k="流式" v=if log.is_stream { "是" } else { "否" }.to_string()/>
                    </div>
                })}
            </Dialog>
        </div>
    }
}

#[component]
fn Stat(label: &'static str, value: String) -> impl IntoView {
    view! {
        <Card class="px-5 py-4">
            <div class="text-2xl font-mono font-semibold text-zinc-100">{value}</div>
            <div class="text-sm text-zinc-400">{label}</div>
        </Card>
    }
}

#[component]
fn Row(k: &'static str, v: String) -> impl IntoView {
    view! {
        <div class="flex justify-between">
            <span class="text-zinc-500">{k}</span>
            <span class="font-mono text-zinc-200">{v}</span>
        </div>
    }
}
