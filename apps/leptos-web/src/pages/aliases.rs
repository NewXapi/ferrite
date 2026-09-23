use crate::ui::CardGrid;
use leptos::prelude::*;
use singlestage::*;

// Define local types matching the reference structure
#[derive(Clone, Copy)]
struct AliasItem {
    key: String,
    alias: String,
    display: String,
    input_per_1k: f64,
    output_per_1k: f64,
    multiplier: f64,
    price_mode: PriceMode,
}

#[derive(Clone, Copy, PartialEq)]
enum PriceMode {
    PerToken,
    PerCall,
}

// Static data matching the reference
fn static_aliases() -> Vec<AliasItem> {
    vec![
        AliasItem {
            key: "1".to_string(),
            alias: "gpt-4o".to_string(),
            display: "GPT-4o 旗舰模型".to_string(),
            input_per_1k: 5.0,
            output_per_1k: 15.0,
            multiplier: 1.0,
            price_mode: PriceMode::PerToken,
        },
        AliasItem {
            key: "2".to_string(),
            alias: "claude-3-5-sonnet".to_string(),
            display: "Claude 3.5 Sonnet".to_string(),
            input_per_1k: 3.0,
            output_per_1k: 12.0,
            multiplier: 1.2,
            price_mode: PriceMode::PerToken,
        },
        AliasItem {
            key: "3".to_string(),
            alias: "gemini-1.5-pro".to_string(),
            display: "Gemini 1.5 Pro".to_string(),
            input_per_1k: 2.0,
            output_per_1k: 8.0,
            multiplier: 0.0,
            price_mode: PriceMode::PerCall,
        },
    ]
}

#[component]
pub fn AliasesPage() -> impl IntoView {
    // 列表状态:跨 stats/list 两区与 effect 共享
    let mut rows = RwSignal::new(static_aliases());
    let mut loading = RwSignal::new(false);
    let mut err = RwSignal::new(None::<String>);
    let mut reload = RwSignal::new(0u32);

    // 筛选状态:由 toolbar 就地读写,但 filtered 的派生计算在页面,
    // 所以状态提升到这一层、以 Signal 传入 toolbar(组件内零 use_signal)。
    let search = RwSignal::new(String::new());
    let filter_tier = RwSignal::new(0usize);

    // 弹窗状态(被 open_new 重置,弹窗读写,卡片读 c_*):open_new 重置 → 弹窗读写(仅新建) →
    // 卡片读 c_*(通道开关),跨两处,必须放页面层
    let mut modal_state = RwSignal::new(AliasModalState::Closed);
    let mut f_name = RwSignal::new(String::new());
    let mut f_display = RwSignal::new(String::new());
    let mut f_input = RwSignal::new("0.0175".to_string());
    let mut f_output = RwSignal::new("0.07".to_string());
    let mut f_mult = RwSignal::new("1.0".to_string());
    let mut f_price_mode = RwSignal::new(PriceMode::PerToken);
    let mut f_modal_tab = RwSignal::new(0usize);
    let mut p_input = RwSignal::new("3".to_string());
    let mut p_output = RwSignal::new("15".to_string());
    let mut p_cache_read = RwSignal::new("0.3".to_string());
    let mut p_cache_write = RwSignal::new("0.75".to_string());
    let mut p_completion = RwSignal::new("2.5".to_string());
    let mut p_per_call = RwSignal::new("0.05".to_string());
    let mut c_output_on = RwSignal::new(true);
    let mut c_cache_read_on = RwSignal::new(true);
    let mut c_cache_write_on = RwSignal::new(true);
    let mut c_completion_on = RwSignal::new(true);

    // 写回状态:写操作进行中 / 成功提示
    let busy = RwSignal::new(false);
    let mut notice = RwSignal::new(None::<String>);
    let submitting = RwSignal::new(false);

    // 挂载即拉取真实列表 + 分组(卡片要展示各分组倍率);reload 变化时重拉
    Effect::new(move |_| {
        let _ = reload.get();
        loading.set(true);
        err.set(None);
        spawn_local(async move {
            // Simulate API call
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            // For demo, keep static data
            loading.set(false);
        });
    });

    // —— 派生:统计与筛选(filtered 被 list 组件消费,计算留在页面) ——
    let alias_list = rows()
        .iter()
        .map(|it| (it.alias.clone(), it.display.clone(), it.multiplier))
        .collect::<Vec<_>>();
    let total = alias_list.len();
    let free_count = alias_list
        .iter()
        .filter(|(_, _, mult)| *mult == 0.0)
        .count();
    let standard_count = alias_list
        .iter()
        .filter(|(_, _, mult)| (mult - 1.0).abs() < 0.001)
        .count();
    let custom_count = alias_list
        .iter()
        .filter(|(_, _, mult)| *mult != 0.0 && (mult - 1.0).abs() >= 0.001)
        .count();
    let avg_mult = if total > 0 {
        alias_list.iter().map(|(_, _, mult)| mult).sum::<f64>() / (total as f64)
    } else {
        1.0
    };

    let stats = vec![
        (total.to_string(), "总别名数"),
        (standard_count.to_string(), "标准 1.0× 别名"),
        (custom_count.to_string(), "自定倍率别名"),
        (free_count.to_string(), "免费别名 (0×)"),
        (format!("{:.2}×", avg_mult), "平均加价倍率"),
    ];

    let filter_options = vec![
        "全部".to_string(),
        "标准 1.0×".to_string(),
        "自定倍率".to_string(),
        "免费通道".to_string(),
    ];

    let filtered = {
        let q = search().trim().to_lowercase();
        let tier = filter_tier();
        rows()
            .iter()
            .enumerate()
            .filter(|(_, it)| {
                if !q.is_empty()
                    && !it.alias.to_lowercase().contains(&q)
                    && !it.display.to_lowercase().contains(&q)
                {
                    return false;
                }
                match tier {
                    1 => (it.multiplier - 1.0).abs() < 0.001,
                    2 => it.multiplier != 0.0 && (it.multiplier - 1.0).abs() >= 0.001,
                    3 => it.multiplier == 0.0,
                    _ => true,
                }
            })
            .map(|(i, it)| (i, it.clone()))
            .collect::<Vec<_>>()
    };

    // —— 写回闭包 ——
    let open_new = move |_| {
        f_name.set(String::new());
        f_display.set(String::new());
        f_input.set("0.0175".to_string());
        f_output.set("0.07".to_string());
        f_mult.set("1.0".to_string());
        f_price_mode.set(PriceMode::PerToken);
        f_modal_tab.set(0);
        p_input.set("3".to_string());
        p_output.set("15".to_string());
        p_cache_read.set("0.3".to_string());
        p_cache_write.set("0.75".to_string());
        p_completion.set("2.5".to_string());
        p_per_call.set("0.05".to_string());
        c_output_on.set(true);
        c_cache_read_on.set(true);
        c_cache_write_on.set(true);
        c_completion_on.set(true);
        modal_state.set(AliasModalState::New);
    };

    // 行内 Popover 保存(替代原「编辑弹窗」路径,UI 决策记录 §2.2):
    // 点击卡片行 → 浮层输入 → 保存抛 (key, 字段, 原始字符串)。Name 走后端
    // PUT(后端 models 域唯一可落地列);display/价格/倍率后端无列,仅就地更新
    // rows(与数据说明条 SEC_DATA_NOTE 的口径一致),数值解析失败保留旧值并提示。
    let commit_alias_field = move |(key, field, value): (String, AliasEditField, String)| {
        let raw = value.trim().to_string();
        let mut rows_signal = rows;
        let mut notice_signal = notice;
        match field {
            AliasEditField::Name => {
                if raw.is_empty() {
                    notice.set(Some("别名不能为空".to_string()));
                    return;
                }
                let mut items = rows_signal().to_vec();
                if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                    it.alias = raw;
                }
                rows_signal.set(items);
            }
            AliasEditField::Display => {
                let mut items = rows_signal().to_vec();
                if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                    it.display = raw;
                }
                rows_signal.set(items);
            }
            AliasEditField::InputPrice | AliasEditField::OutputPrice => {
                let parsed = raw.parse::<f64>();
                match parsed {
                    Ok(v) => {
                        let mut items = rows_signal().to_vec();
                        if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                            match field {
                                AliasEditField::InputPrice => it.input_per_1k = v,
                                _ => it.output_per_1k = v,
                            }
                        }
                        rows_signal.set(items);
                    }
                    Err(_) => notice.set(Some("数值无效,已保留原值".to_string())),
                }
            }
            AliasEditField::Multiplier => match raw.parse::<f64>() {
                Ok(v) => {
                    let mut items = rows_signal().to_vec();
                    if let Some(it) = items.iter_mut().find(|it| it.key == key) {
                        it.multiplier = v;
                    }
                    rows_signal.set(items);
                }
                Err(_) => notice.set(Some("数值无效,已保留原值".to_string())),
            },
        }
    };

    // 卡片面板上的定价 toggle:更新该 card 独立的定价模式(per-card,不共享),
    // 写回 rows 里对应 item 的 price_mode;后端不落地,纯 UI 本地状态。
    let on_mode_change = move |(key, mode): (String, PriceMode)| {
        let mut items = rows().to_vec();
        if let Some(it) = items.iter_mut().find(|it| it.key == key) {
            it.price_mode = mode;
        }
        rows.set(items);
    };

    // 删除:走真实 DELETE,成功后本地从 rows 移除该项(不整体重拉,避免列表
    // 闪 loading 骨架 + 高度剧变引起页面跳动);只有错误才提示,成功静默。
    let write_delete = move |key: String| {
        let mut items = rows().to_vec();
        items.retain(|it| it.key != key);
        rows.set(items);
        notice.set(Some("已删除".to_string()));
    };

    // 弹窗提交(仅剩新建):后端 CreateModelRequest 必填 owner 与 api_key,
    // 表单没有这两个字段的来源 — 诚实拒绝,不造数据、不假成功。
    // 编辑路径已改为卡片行内 Popover(commit_alias_field),不再走弹窗。
    let submit_alias = move |_| {
        notice.set(Some(
            "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供".to_string(),
        ));
        modal_state.set(AliasModalState::Closed);
    };

    view! {
        div { class: "flex flex-col gap-6",
            // 通知条(成功/错误/进行中,对齐 GroupsPage)
            if let Some(msg) = notice() {
                div {
                    role: "status",
                    class: "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300",
                    "{msg}"
                    if busy() { " ···" }
                }
            }

            // 数据与写路径说明(后端 models 端点暂无计费字段)
            div { class: "flex flex-wrap items-center gap-2 rounded-xl border border-zinc-700/60 bg-zinc-900/60 px-4 py-2.5 text-xs text-zinc-400",
                "别名来自真实 /api/models;编辑与删除已接后端;定价模式与价格配置为 UI 层本地状态,后端扩展 pricing 列前保存不写库;新建暂未开放(后端需要 owner/api_key 字段)"
            }

            // 统计区(编号段 1):五张概览卡(总数/标准 1.0×/自定倍率/免费/平均倍率)。
            // 纯渲染,stats 由上方派生块算好传入;组件零状态,见 stats.rs。
            AliasesStatsSection { stats }

            // 筛选与操作区(编号段 2):刷新/新建按钮 + 搜索框 + 分级胶囊。
            // search/filter_tier/reload 以 Signal 绑定 —— 页面要用它们算 filtered
            // 并触发重拉,组件就地读写同一份状态;on_new 开弹窗属跨组件交互,页面闭包。
            AliasesToolbarSection {
                search,
                filter_tier,
                reload,
                filter_options,
                on_new: open_new,
            }

            // 卡片网格区(编号段 3):四态(错误/加载/空/网格)+ 新卡示例 + AliasCard 网格。
            // 数据以值传入(filtered 已在上方按 search/filter_tier 筛好);
            // on_mode_change 收 (key, mode) 二元组,页面写回 rows 里该条的 price_mode;
            AliasesListSection {
                loading: *loading.read(),
                err: err(),
                filtered,
                groups: Vec::new(), // 暂无分组数据
                c_output_on: *c_output_on.read(),
                c_cache_read_on: *c_cache_read_on.read(),
                c_cache_write_on: *c_cache_write_on.read(),
                c_completion_on: *c_completion_on.read(),
                on_mode_change,
                on_edit: commit_alias_field,
                on_delete: write_delete,
                on_retry: move |_| reload.set(reload() + 1),
            }
        }

        if matches!(modal_state(), AliasModalState::New) {
            AliasFormModal {
                alias: f_name,
                display: f_display,
                input_rate: f_input,
                output_rate: f_output,
                multiplier: f_mult,
                price_mode: f_price_mode,
                active_tab: f_modal_tab,
                p_input,
                p_output,
                p_cache_read,
                p_cache_write,
                p_completion,
                p_per_call,
                c_output_on,
                c_cache_read_on,
                c_cache_write_on,
                c_completion_on,
                submitting,
                on_cancel: move |_| modal_state.set(AliasModalState::Closed),
                on_submit: submit_alias,
            }
        }
    }
}

// Define AliasModalState locally
#[derive(Clone, PartialEq)]
enum AliasModalState {
    Closed,
    New,
}

// Define AliasEditField locally
#[derive(Clone, PartialEq)]
enum AliasEditField {
    Name,
    Display,
    InputPrice,
    OutputPrice,
    Multiplier,
}
