//! 实体设置页共享类型、小件与文案：实体卡外壳、录入原子件、数值解析。
//! cards / channels / page 三处复用。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:三张实体卡各自的业务逻辑(分组写回 / 渠道 CRUD)在 `cards.rs` /
//! `channels.rs`;这里只放跨卡复用的样式外壳、输入原子件与文案常量。

use dioxus::prelude::*;

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- FIELD_* : 表单字段标签 ----

/// 跨卡片共享:展示名录入框标签。
pub const FIELD_DISPLAY: &str = "展示名";
/// 分组卡:分组名录入框标签(编辑态)。
pub const FIELD_GROUP_NAME_LOCKED: &str = "分组名（锁读，后端无改名路径）";
/// 分组卡:分组名录入框标签(新增态)。
pub const FIELD_GROUP_NAME: &str = "分组名";
/// 别名卡:别名录入框标签。
pub const FIELD_ALIAS: &str = "别名";
/// 别名卡:输入价录入框标签。
pub const FIELD_INPUT_PRICE: &str = "输入价 ¥/1k";
/// 别名卡:输出价录入框标签。
pub const FIELD_OUTPUT_PRICE: &str = "输出价 ¥/1k";
/// 渠道卡:渠道名称录入框标签。
pub const FIELD_CHANNEL_NAME: &str = "渠道名称";
/// 渠道卡:渠道类型下拉标签。
pub const FIELD_CHANNEL_TYPE: &str = "类型";
/// 渠道卡:Base URL 录入框标签。
pub const FIELD_BASE_URL: &str = "Base URL";
/// 渠道卡:API Key 录入框标签。
pub const FIELD_API_KEY: &str = "API Key（留空 = 不改动现有密钥）";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 跨卡片共享:倍率录入框标签。
pub const LBL_MULTIPLIER: &str = "倍率";
/// 渠道卡:候补池区段标题。
pub const LBL_CANDIDATE_POOL: &str = "候补池";
/// 渠道卡:候补池区段说明。
pub const LBL_CANDIDATE_POOL_HINT: &str = "拉取结果，尚未进入拓扑";
/// 渠道卡:调度模型区段标题。
pub const LBL_DISPATCH_MODELS: &str = "调度模型";
/// 渠道卡:调度模型区段说明。
pub const LBL_DISPATCH_MODELS_HINT: &str = "已在拓扑中；名字来自上游，不可改";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 分组卡标题。
pub const SEC_CARD_GROUPS: &str = "分组";
/// 分组卡标题旁说明。
pub const SEC_CARD_GROUPS_HINT: &str = "对模型别名分组；分组本身只有名字";
/// 别名卡标题。
pub const SEC_CARD_ALIASES: &str = "模型别名";
/// 别名卡标题旁说明。
pub const SEC_CARD_ALIASES_HINT: &str = "对外暴露给用户的模型名；卡牌样式后续再做";
/// 渠道卡标题。
pub const SEC_CARD_CHANNELS: &str = "渠道";
/// 渠道卡标题旁说明。
pub const SEC_CARD_CHANNELS_HINT: &str = "URL + Key 是凭证容器；调度模型由候补池加入，名字不可改";

// ---- TTL_* : 弹窗标题 ----

/// 删除分组确认弹窗标题。
pub const TTL_DELETE_GROUP: &str = "删除分组";
/// 删除渠道确认弹窗标题。
pub const TTL_DELETE_CHANNEL: &str = "删除渠道";

// ---- BTN_* : 按钮文案 ----

/// 录入行新增态提交按钮。
pub const BTN_NEW: &str = "新增";
/// 录入行编辑态提交按钮。
pub const BTN_UPDATE: &str = "更新";
/// 录入行取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 渠道卡:新建渠道按钮。
pub const BTN_NEW_CHANNEL: &str = "＋ 新建渠道";
/// 渠道卡:保存按钮。
pub const BTN_SAVE: &str = "保存";
/// 渠道卡:停用按钮(渠道启用中时显示)。
pub const BTN_DISABLE: &str = "停用";
/// 渠道卡:启用按钮(渠道停用时显示)。
pub const BTN_ENABLE: &str = "启用";
/// 渠道卡:删除按钮。
pub const BTN_DELETE_CHANNEL: &str = "删除此渠道";
/// 渠道卡:把勾选的候补模块加入调度。
pub const BTN_JOIN_DISPATCH: &str = "加入调度 →";
/// 渠道卡:清空候补池。
pub const BTN_CLEAR_CANDIDATES: &str = "清空候补";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位文案 ----

/// 分组卡空态。
pub const MSG_EMPTY_GROUPS: &str = "还没有分组";
/// 别名卡空态。
pub const MSG_EMPTY_ALIASES: &str = "还没有模型别名";
/// 渠道卡:草稿态节点区占位。
pub const MSG_DRAFT_CHANNEL: &str = "草稿渠道：填好 URL + Key 保存后，这里才会显示候补池与调度模型";
/// 渠道卡:候补池空态。
pub const MSG_EMPTY_CANDIDATES: &str = "点「拉取模型」获取候补";
/// 渠道卡:调度模型空态。
pub const MSG_EMPTY_DISPATCH: &str = "从左侧候补池加入";
/// 渠道卡:新建态渠道名称预填值。
pub const MSG_NEW_CHANNEL_NAME: &str = "新渠道";
/// 渠道卡:新建态缺少 API Key 的拦截提示。
pub const MSG_ERR_NEED_API_KEY: &str = "新建渠道至少填写一个 API Key";
/// 渠道卡:草稿未保存时启停按钮的 title。
pub const MSG_TITLE_DRAFT_NO_TOGGLE: &str = "草稿未保存，保存后才能启停";
/// 渠道卡:草稿未保存时删除按钮的 title。
pub const MSG_TITLE_DRAFT_NO_DELETE: &str = "草稿未保存，保存后才能删除";
/// 渠道卡:调度模型行移出按钮的 title。
pub const MSG_TITLE_MOVE_OUT: &str = "移出拓扑，退回候补池";
/// 分组名称录入框占位。
pub const MSG_PH_GROUP_NAME: &str = "vip";
/// 分组展示名录入框占位。
pub const MSG_PH_GROUP_DISPLAY: &str = "默认分组（可选）";
/// 渠道名称录入框占位。
pub const MSG_PH_CHANNEL_NAME: &str = "OpenAI 官方";
/// 渠道 Base URL 录入框占位。
pub const MSG_PH_BASE_URL: &str = "https://…";
/// 渠道 API Key 录入框占位。
pub const MSG_PH_API_KEY: &str = "sk-…";
/// 别名展示名录入框占位。
pub const MSG_PH_ALIAS_DISPLAY: &str = "GPT-4o（可选）";

// ---- 写回结果提示（前缀 / 后缀，与动态内容拼接） ----

/// 写回失败提示前缀(后接错误详情)。
pub const MSG_SAVE_FAILED_PREFIX: &str = "保存失败：";
/// 删除失败提示前缀(后接错误详情)。
pub const MSG_DELETE_FAILED_PREFIX: &str = "删除失败：";
/// 启停失败提示前缀(后接错误详情)。
pub const MSG_TOGGLE_FAILED_PREFIX: &str = "启停失败：";
/// 分组在写回时已不存在于后端的提示前缀(后接分组名)。
pub const MSG_GROUP_MISSING_PREFIX: &str = "分组「";
/// 分组在写回时已不存在于后端的提示后缀。
pub const MSG_GROUP_MISSING_SUFFIX: &str = "」不存在于后端（可能已被删除）";
/// 渠道在启停时已不存在于后端的提示前缀(后接渠道名)。
pub const MSG_CHANNEL_MISSING_PREFIX: &str = "渠道「";
/// 渠道在启停时已不存在于后端的提示后缀。
pub const MSG_CHANNEL_MISSING_SUFFIX: &str = "」不存在于后端（可能已被删除）";
/// 编辑行的本地下标已失效时的提示。
pub const MSG_ROW_STALE: &str = "分组行已失效，请刷新后重试";
/// 删除分组确认弹窗正文前缀(后接分组名)。
pub const MSG_CONFIRM_DELETE_GROUP_PREFIX: &str = "确认删除分组「";
/// 删除渠道确认弹窗正文前缀(后接渠道名)。
pub const MSG_CONFIRM_DELETE_CHANNEL_PREFIX: &str = "确认删除渠道「";
/// 删除确认弹窗正文后缀(共用)。
pub const MSG_CONFIRM_DELETE_SUFFIX: &str = "」？该操作直接生效于后端，不可撤销。";

// ============ 共享小件 ============

/// 实体卡外壳：可折叠的卡片面板，头部按钮整行切换展开态。
///
/// 【是什么】实体设置页三张实体卡共用的可折叠外壳(标题 + 计数 + 提示 + 内容区)。
///
/// 【做什么】渲染卡片边框、头部按钮(标题/计数胶囊/灰色提示)与展开时的内容区;
/// 不负责卡片内部业务内容与写路径(由调用方通过 `children` 注入)。
///
/// 【交互逻辑】点头部按钮 → `on_toggle(MouseEvent)` 抛回调用方(页面持 `open` 数组),
/// 组件自身不持展开态、不发网络。
///
/// 【样式】外壳 `shrink-0 overflow-hidden rounded-xl border border-zinc-800
/// bg-zinc-900/60`;头部 `flex w-full items-center gap-2 px-4 py-2.5 text-left
/// hover:bg-zinc-900`;计数胶囊 `rounded-full border border-zinc-700 px-1.5
/// text-[11px] text-zinc-400`;内容区 `space-y-3 border-t border-zinc-800 p-4`。
///
/// 【子组件组成】无独立子组件,只有内联 `section` / `button` / `span` / `div`。
///
/// 【数据流】
/// - 对内(入):`section_index`(拼锚点 id `ent-card-{n}`)、`title` / `hint`(头部
///   标题与灰色说明)、`count`(计数胶囊)、`open`(展开态,由调用方持有)、
///   `children`(卡片内容)。
/// - 对外(出):`on_toggle(MouseEvent)` → 调用方翻转 `open` 数组中对应下标。
#[component]
pub fn CardPanel(
    section_index: usize,
    title: &'static str,
    hint: &'static str,
    count: usize,
    open: bool,
    on_toggle: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    let id = format!("ent-card-{section_index}");
    rsx! {
        section { id: "{id}", class: "shrink-0 overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60",
            button {
                class: "flex w-full items-center gap-2 px-4 py-2.5 text-left transition-colors hover:bg-zinc-900",
                onclick: move |e| on_toggle.call(e),
                span { class: "{ui::TYPE_CARD_TITLE}", "{title}" }
                span { class: "rounded-full border border-zinc-700 px-1.5 {ui::TYPE_LABEL}", "{count}" }
                span { class: "truncate {ui::TYPE_LABEL}", "{hint}" }
            }
            if open {
                div { class: "space-y-3 border-t border-zinc-800 p-4", {children} }
            }
        }
    }
}

/// 卡片下半的拓扑节点区容器：统一深底描边的内容槽。
///
/// 【是什么】三张实体卡下半区(候补池 / 调度模型 / 实体列表)共用的内容容器。
///
/// 【做什么】只提供 `min-h-[104px]` 的深色描边槽位并渲染 `children`;不负责槽内布局。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】`div.min-h-[104px] rounded-lg border border-zinc-800 bg-zinc-950 p-3`。
///
/// 【子组件组成】无独立子组件,只渲染 `children`。
///
/// 【数据流】
/// - 对内(入):`children`(槽内内容,由调用方组装)。
/// - 对外(出):无。
#[component]
pub fn NodeArea(children: Element) -> Element {
    rsx! {
        div { class: "min-h-[104px] rounded-lg border border-zinc-800 bg-zinc-950 p-3", {children} }
    }
}

/// 节点区内的空态提示：居中灰字。
///
/// 【是什么】拓扑节点区与候补池 / 调度模型列表共用的空态占位行。
///
/// 【做什么】在可用空间内居中渲染一行 `text-[11px]` 灰字提示;不负责图标与操作入口。
///
/// 【交互逻辑】纯展示，无交互。
///
/// 【样式】外层 `flex h-full min-h-[72px] items-center justify-center`,
/// 文案 `text-[11px] text-zinc-600`。
///
/// 【子组件组成】无独立子组件,只有内联 `div` / `span`。
///
/// 【数据流】
/// - 对内(入):`text`(空态文案,调用方传入文案常量)。
/// - 对外(出):无。
#[component]
pub fn EmptyHint(text: &'static str) -> Element {
    rsx! {
        div { class: "flex h-full min-h-[72px] items-center justify-center",
            span { class: "{ui::TYPE_LABEL}", "{text}" }
        }
    }
}

/// 可分组的实体胶囊：标签 + 可选副标题 + 右侧移除按钮。
///
/// 【是什么】分组 / 别名的实体胶囊,点击主体进入编辑、点右上角 ✕ 请求删除。
///
/// 【做什么】按 `active` 切换选中配色并渲染 `label`(必显)与 `sub`(非空才显);
/// 不负责编辑表单回填与删除确认弹窗(由调用方的两个回调处理)。
///
/// 【交互逻辑】用户操作 → 组件行为 → 数据交互:
/// - 点胶囊主体 → `on_pick(MouseEvent)` 抛回调用方(通常回填录入行并置编辑态)。
/// - 点 ✕ → `on_remove(MouseEvent)` 抛回调用方(通常打开删除确认弹窗)。
/// 组件自身无状态、不发网络。
///
/// 【样式】胶囊 `inline-flex items-center gap-1.5 rounded-full border py-1 pl-3 pr-1.5
/// transition-colors`;选中 `border-zinc-100 bg-zinc-100 text-zinc-900`,
/// 未选中 `border-zinc-700 bg-zinc-900 text-zinc-200 hover:border-zinc-500`;
/// 副标题随选中态在 `text-zinc-600` / `text-zinc-500` 间切换;✕ 按钮 `opacity-50
/// hover:text-red-400 hover:opacity-100`。
///
/// 【子组件组成】无独立子组件,只有内联 `span` 与两个 `button`。
///
/// 【数据流】
/// - 对内(入):`label`(主文案)、`sub`(副文案,空串则不渲染)、`active`(是否选中)、
///   `on_pick` / `on_remove`。
/// - 对外(出):`on_pick` → 调用方进入该实体的编辑态;`on_remove` → 调用方置确认下标。
#[component]
pub fn EntityChip(
    label: String,
    sub: String,
    active: bool,
    on_pick: EventHandler<MouseEvent>,
    on_remove: EventHandler<MouseEvent>,
) -> Element {
    let tone = if active {
        "border-zinc-100 bg-zinc-100 text-zinc-900"
    } else {
        "border-zinc-700 bg-zinc-900 text-zinc-200 hover:border-zinc-500"
    };
    let sub_tone = if active {
        "text-zinc-600"
    } else {
        "text-zinc-500"
    };
    rsx! {
        span { class: "inline-flex items-center gap-1.5 rounded-full border py-1 pl-3 pr-1.5 transition-colors {tone}",
            button {
                class: "flex items-baseline gap-1.5",
                onclick: move |e| on_pick.call(e),
                span { class: "{ui::TYPE_DESC}", "{label}" }
                if !sub.is_empty() {
                    span { class: "{ui::TYPE_LABEL} {sub_tone}", "{sub}" }
                }
            }
            button {
                class: "px-1 {ui::TYPE_LABEL} opacity-50 hover:text-red-400 hover:opacity-100",
                onclick: move |e| on_remove.call(e),
                "✕"
            }
        }
    }
}

/// 录入行里的文本输入格：标签 + 单行 input，双向绑定 Signal。
///
/// 【是什么】三张实体卡录入行共用的文本输入原子件(标签在输入框上方)。
///
/// 【做什么】渲染标签与 `input`,并把输入值直接写回调用方传入的 `Signal<String>`;
/// 不负责校验、不负责提交。
///
/// 【交互逻辑】在输入框输入 → `value.set(e.value())` 就地写回 Signal(状态住在
/// 页面 / store),无网络、无外部回调。
///
/// 【样式】`label.block space-y-1`(`grow` 时追加 `min-w-[140px] flex-1`);
/// 标签 `text-[11px] text-zinc-500`;输入框 `w-full rounded-md border border-zinc-800
/// bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors
/// placeholder:text-zinc-600 focus:border-zinc-500`。
///
/// 【子组件组成】无独立子组件,只有内联 `label` / `span` / `input`。
///
/// 【数据流】
/// - 对内(入):`label`(字段名)、`value`(双向绑定句柄)、`placeholder`、
///   `grow`(是否撑满剩余宽度,默认 false)。
/// - 对外(出):写回 `value` Signal;无 EventHandler。
#[component]
pub fn InputCell(
    label: &'static str,
    value: Signal<String>,
    placeholder: &'static str,
    #[props(default = false)] grow: bool,
) -> Element {
    let width = if grow { "min-w-[140px] flex-1" } else { "" };
    rsx! {
        label { class: "block space-y-1 {width}",
            span { class: "{ui::TYPE_LABEL}", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 {ui::TYPE_BODY} outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value.read()}",
                placeholder: "{placeholder}",
                oninput: move |e| value.set(e.value()),
            }
        }
    }
}

/// 录入行里的原生下拉格：标签 + select（移动端友好）。
///
/// 【是什么】实体录入行共用的下拉选择原子件,选项由静态切片传入。
///
/// 【做什么】渲染标签与 `select`,选中项与 `value` 比较以标 `selected`;
/// 不负责选项候选的拉取。样式与 `InputCell` / `TextCell` 保持一致。
///
/// 【交互逻辑】切换选项 → `oninput.call(e.value())` 把新值抛回调用方
/// (调用方据此写状态),无网络。
///
/// 【样式】`label.block space-y-1`;标签 `text-[11px] text-zinc-500`;
/// `select` 为 `w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5
/// text-sm text-zinc-200 outline-none transition-colors focus:border-zinc-500`。
///
/// 【子组件组成】无独立子组件,只有内联 `label` / `span` / `select` / `option`。
///
/// 【数据流】
/// - 对内(入):`label`(字段名)、`value`(当前值)、`options`(静态候选)、`oninput`。
/// - 对外(出):`oninput(String)` → 调用方更新该字段状态。
#[component]
pub fn SelectCell(
    label: &'static str,
    value: String,
    options: &'static [&'static str],
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "{ui::TYPE_LABEL}", "{label}" }
            select {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 {ui::TYPE_BODY} outline-none transition-colors focus:border-zinc-500",
                value: "{value}",
                oninput: move |e| oninput.call(e.value()),
                for opt in options {
                    option { value: "{opt}", selected: *opt == value, "{opt}" }
                }
            }
        }
    }
}

/// 录入行里的受控文本格：标签 + input，值经 EventHandler 抛回。
///
/// 【是什么】与 `InputCell` 同为文本输入原子件,区别在状态不经 Signal 而经回调上报。
///
/// 【做什么】渲染标签与 `input`,把当前值显示出来;不负责持有状态(状态在调用方)。
///
/// 【交互逻辑】在输入框输入 → `oninput.call(e.value())` 抛回调用方,无网络。
///
/// 【样式】与 `InputCell` 完全一致:`label.block space-y-1` + 标签 `text-[11px]
/// text-zinc-500` + 输入框 `w-full rounded-md border border-zinc-800 bg-zinc-950
/// px-3 py-1.5 text-sm ... focus:border-zinc-500`。
///
/// 【子组件组成】无独立子组件,只有内联 `label` / `span` / `input`。
///
/// 【数据流】
/// - 对内(入):`label`(字段名)、`value`(当前值快照)、`placeholder`、`oninput`。
/// - 对外(出):`oninput(String)` → 调用方更新该字段状态。
#[component]
pub fn TextCell(
    label: &'static str,
    value: String,
    placeholder: &'static str,
    oninput: EventHandler<String>,
) -> Element {
    rsx! {
        label { class: "block space-y-1",
            span { class: "{ui::TYPE_LABEL}", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 {ui::TYPE_BODY} outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
                value: "{value}",
                placeholder: "{placeholder}",
                oninput: move |e| oninput.call(e.value()),
            }
        }
    }
}

/// 非负单价解析:空/非法回退 0,负数归零
pub fn parse_nonneg(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(0.0).max(0.0)
}

/// 倍率解析:空/非法回退 1.0,负数归零
pub fn parse_mult(s: &str) -> f64 {
    s.trim().parse::<f64>().unwrap_or(1.0).max(0.0)
}
