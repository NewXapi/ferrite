//! 订阅套餐 tab 共享件:页面骨架组件(`GridShell` / `Panel`)、三个通用按钮
//! (`PushBtn` / `DangerBtn` / `GhostBtn`)与状态开关(`ToggleSwitch`),
//! 以及该 tab 的全部用户可见文案常量。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放业务面板(`PlanCard` 在 `card.rs`,`SubscriptionFormModal` 与三个
//! Tab 体在 `modal.rs`,下拉选项列表在 `options.rs`)；不放网络调用(在
//! `page.rs` 的 `commit` 与 `modal.rs`)。订阅套餐后端暂未实现,数据全走
//! `state::EntityStore` 本地演示态。

use dioxus::prelude::*;

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 套餐卡指标:价格。
pub const LBL_PRICE: &str = "价格";
/// 套餐卡指标:有效期。
pub const LBL_PERIOD: &str = "有效期";
/// 套餐卡指标:套餐额度。
pub const LBL_QUOTA: &str = "套餐额度";
/// 套餐卡指标:站内支付 / 渠道。
pub const LBL_PAY_CHANNEL: &str = "站内支付 / 渠道";
/// 套餐卡指标:额度重置。
pub const LBL_RESET_CYCLE: &str = "额度重置";
/// 套餐卡状态徽标:启用。
pub const LBL_ENABLED: &str = "启用";
/// 套餐卡状态徽标:禁用。
pub const LBL_DISABLED: &str = "禁用";
/// 套餐卡额度不限时的替代文案。
pub const LBL_UNLIMITED: &str = "无限制";

/// 套餐卡分组徽标前缀(后接分组名)。
pub const LBL_GROUP_PREFIX: &str = "分组: ";

// ---- BTN_* : 按钮文案 ----

/// 页面顶栏新建套餐按钮。
pub const BTN_NEW_PLAN: &str = "新建套餐";
/// 套餐卡编辑按钮。
pub const BTN_EDIT: &str = "编辑";
/// 弹窗底部关闭按钮。
pub const BTN_CLOSE: &str = "关闭";
/// 弹窗底部保存按钮。
pub const BTN_SAVE: &str = "保存更改";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 顶部栏:套餐按名称去重的提示。
pub const SEC_NAME_DEDUP: &str = "套餐按名称去重：同名保存即更新现有套餐，改名会新建一行";
/// 列表为空时的空态提示。
pub const MSG_EMPTY: &str = "还没有订阅套餐。点击右上角「新建套餐」创建第一个。";
/// 列表加载中提示。
pub const MSG_LOADING: &str = "正在加载订阅套餐…";
/// 列表加载失败前缀(后接错误详情)。
pub const MSG_LOAD_FAIL_PREFIX: &str = "加载失败：";
/// 列表加载失败后缀(预留,当前为空)。
pub const MSG_LOAD_FAIL_SUFFIX: &str = "";
/// 写请求在途提示。
pub const MSG_SAVING: &str = "正在与后端同步…";
/// 保存按钮在途文案。
pub const BTN_SAVING: &str = "保存中…";
/// 更新成功提示。
pub const MSG_UPDATED: &str = "套餐已更新";
/// 创建成功提示。
pub const MSG_CREATED: &str = "套餐已创建";
/// 删除成功提示。
pub const MSG_DELETED: &str = "套餐已删除";
/// 表单校验:名称必填。
pub const MSG_ERR_NAME_REQUIRED: &str = "套餐名称必填";
/// 表单校验:额度非法。
pub const MSG_ERR_QUOTA: &str = "额度必须是数字且不小于 0";
/// 套餐标题旁说明:按名称去重语义。
pub const MSG_TITLE_HINT: &str = "名称唯一：同名保存会更新已有套餐，而非新建";
/// 计价币种旁说明。
pub const MSG_CURRENCY_HINT: &str = "决定列表价格符号；后端要求非空";
/// 有效期(天)旁说明。
pub const MSG_DURATION_HINT: &str = "后端按天存储有效期；至少 1 天";
/// 升级分组旁说明。
pub const MSG_GROUP_HINT: &str = "购买该套餐后升级到该分组；「不升级」表示不改变分组";
/// 限购为 0 时的展示文案。
pub const LBL_NO_LIMIT: &str = "不限";
/// 删除按钮文案。
pub const BTN_DELETE: &str = "✕";

// ---- TAB_* : 弹窗内页签标题 ----

/// 弹窗页签:基本信息。
pub const TAB_BASIC: &str = "基本信息";
/// 弹窗页签:规则与周期。
pub const TAB_RULES: &str = "规则与周期";
/// 弹窗页签:第三方支付配置。
pub const TAB_PAYMENT: &str = "第三方支付配置";

// ---- TTL_* : 弹窗 / 抽屉标题 ----

/// 弹窗标题:编辑态。
pub const TTL_EDIT: &str = "更新套餐信息";
/// 弹窗标题:新建态。
pub const TTL_NEW: &str = "新建订阅套餐";
/// 弹窗副标题:说明。
pub const TTL_SUBTITLE: &str = "修改现有订阅套餐的配置";

// ---- FIELD_* : 表单字段标签 ----

/// 基本信息字段:套餐标题。
pub const FIELD_PLAN_TITLE: &str = "套餐标题";
/// 基本信息字段:套餐副标题。
pub const FIELD_PLAN_SUBTITLE: &str = "套餐副标题";
/// 基本信息字段:套餐价格 ($)。
pub const FIELD_PRICE: &str = "套餐价格 ($)";
/// 基本信息字段:额度 (点)。
pub const FIELD_QUOTA: &str = "额度 (点)";
/// 基本信息字段:套餐价格（菌种）。
pub const FIELD_CURRENCY_PRICE: &str = "套餐价格（菌种）";
/// 基本信息字段:站内支付方式。
pub const FIELD_PAYMENT_METHOD: &str = "站内支付方式";
/// 基本信息字段:升级分组。
pub const FIELD_GROUP: &str = "升级分组";
/// 基本信息字段:降级分组。
pub const FIELD_DOWNGRADE_GROUP: &str = "降级分组";
/// 基本信息字段:限购。
pub const FIELD_LIMIT: &str = "限购";
/// 基本信息字段:排序。
pub const FIELD_SORT: &str = "排序";
/// 规则字段:启用状态。
pub const FIELD_ENABLED: &str = "启用状态";
/// 规则字段:允许余额兑换。
pub const FIELD_ALLOW_REDEEM: &str = "允许余额兑换";
/// 规则字段:额度用尽后允许使用钱包余额。
pub const FIELD_ALLOW_WALLET: &str = "额度用尽后允许使用钱包余额";
/// 规则字段:有效期设置区标题。
pub const FIELD_PERIOD_SECTION: &str = "有效期设置";
/// 规则字段:有效期数值。
pub const FIELD_PERIOD_VAL: &str = "有效期数值";
/// 规则字段:有效期单位。
pub const FIELD_PERIOD_UNIT: &str = "有效期单位";
/// 规则字段:额度重置区标题。
pub const FIELD_RESET_SECTION: &str = "额度重置";
/// 规则字段:重置周期。
pub const FIELD_RESET_CYCLE: &str = "重置周期";
/// 规则字段:自定义秒数。
pub const FIELD_RESET_SECS: &str = "自定义秒数";
/// 支付字段:Stripe Price ID。
pub const FIELD_STRIPE_ID: &str = "Stripe Price ID";
/// 支付字段:Creem Product ID。
pub const FIELD_CREEM_ID: &str = "Creem Product ID";
/// 支付字段:Waffo Pancake Product ID。
pub const FIELD_WAFFO_ID: &str = "Waffo Pancake Product ID";

// ---- OPT_* : 下拉选项 / 分段选择器选项 ----

/// 站内支付方式选项:仅扣菌种。
pub const OPT_PAY_ONLY_SPECIES: &str = "仅扣菌种";
/// 站内支付方式选项:允许余额兑换。
pub const OPT_PAY_WALLET_EXCHANGE: &str = "允许余额兑换";
/// 站内支付方式选项:无限制。
pub const OPT_PAY_UNLIMITED: &str = "无限制";
/// 升级分组选项:不升级。
pub const OPT_NO_UPGRADE: &str = "不升级";
/// 降级分组选项:降级到购买前分组。
pub const OPT_DOWNGRADE_PREV: &str = "降级到购买前分组";
/// 降级分组选项:默认分组。
pub const OPT_DEFAULT_GROUP: &str = "默认分组";
/// 有效期单位选项:小时。
pub const OPT_UNIT_HOUR: &str = "小时";
/// 有效期单位选项:天。
pub const OPT_UNIT_DAY: &str = "天";
/// 有效期单位选项:个月。
pub const OPT_UNIT_MONTH: &str = "个月";
/// 有效期单位选项:年。
pub const OPT_UNIT_YEAR: &str = "年";
/// 有效期单位选项:秒。
pub const OPT_UNIT_SECOND: &str = "秒";
/// 重置周期选项:不重置。
pub const OPT_RESET_NEVER: &str = "不重置";
/// 重置周期选项:每天。
pub const OPT_RESET_DAILY: &str = "每天";
/// 重置周期选项:每周。
pub const OPT_RESET_WEEKLY: &str = "每周";
/// 重置周期选项:每月。
pub const OPT_RESET_MONTHLY: &str = "每月";
/// 重置周期选项:自定义。
pub const OPT_RESET_CUSTOM: &str = "自定义";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 套餐标题输入框占位。
pub const MSG_PH_PLAN_TITLE: &str = "例如：开拓的封赏";
/// 套餐副标题输入框占位。
pub const MSG_PH_PLAN_SUBTITLE: &str = "向你们致敬，向外开拓的勇士们！";
/// 套餐价格输入框旁的说明。
pub const MSG_PRICE_HINT: &str = "用户购买该套餐需支付的金额，具体币种由支付渠道决定";
/// 额度输入框旁的说明。
pub const MSG_QUOTA_HINT: &str = "套餐包含的总额度；0 表示不限量";
/// 套餐价格（菌种）输入框旁的说明。
pub const MSG_CURRENCY_PRICE_HINT: &str = "最小单位 0.1。仅当支付方式包含它时才生效。";
/// 站内支付方式旁的说明。
pub const MSG_PAYMENT_METHOD_HINT: &str = "只影响站内货币，不影响第三方支付渠道。";
/// 降级分组旁的说明。
pub const MSG_DOWNGRADE_HINT: &str = "订阅过期后降级到该分组";
/// 限购输入框旁的说明。
pub const MSG_LIMIT_HINT: &str = "单个用户可购买的次数；0 表示不限";
/// 支付 Tab 顶部说明条。
pub const MSG_PAYMENT_NOTE: &str = "使用此套餐的标题和价格，在已保存的店铺中创建 Pancake 产品。需要先在支付设置中完整配置 Waffo Pancake。";
/// Stripe Price ID 输入框占位。
pub const MSG_PH_STRIPE_ID: &str = "price_1M...";
/// Creem Product ID 输入框占位。
pub const MSG_PH_CREEM_ID: &str = "prod_...";
/// Waffo Pancake Product ID 输入框占位。
pub const MSG_PH_WAFFO_ID: &str = "选择产品或输入 ID";

// ============ 页面骨架 ============

/// 1/3 栏响应式网格(手机 1 / 平板与Web 3 栏)。
///
/// 【是什么】一个纯布局容器:把 children 铺进 1/3 栏响应式网格。
///
/// 【做什么】只做排版;不持有任何状态、不管 children 内容。
///
/// 【交互逻辑】纯展示,无交互。
///
/// 【样式】`grid grid-cols-1 gap-3 md:grid-cols-3`(手机 1 栏 / md 起 3 栏,
/// 列间距 `gap-3`)。
///
/// 【子组件组成】无:直接渲染传入的 `children`。
///
/// 【数据流】
/// - 对内(入):`children` —— 调用方传入的任意 rsx 子树。
/// - 对外(出):无 EventHandler / Signal 写回。
#[component]
pub fn GridShell(children: Element) -> Element {
    rsx! {
        div { class: "grid grid-cols-1 gap-3 md:grid-cols-3", {children} }
    }
}

/// 面板基础件:标题 + 说明 + 内容。
///
/// 【是什么】带标题与说明的圆角描边面板外壳。
///
/// 【做什么】渲染标题行 + 灰色说明行 + children 内容区;不持有状态、
/// 不处理交互。
///
/// 【交互逻辑】纯展示,无交互。
///
/// 【样式】外壳 `space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3`;
/// 标题 `text-sm font-medium text-zinc-100`,说明 `text-[11px] text-zinc-600`。
///
/// 【子组件组成】无:children 直接落在说明行之后。
///
/// 【数据流】
/// - 对内(入):`title`(标题静态串,调用方以 `&'static str` 提供)、
///   `hint`(说明静态串)、`children`(内容区子树)。
/// - 对外(出):无 EventHandler / Signal 写回。
#[component]
pub fn Panel(title: &'static str, hint: &'static str, children: Element) -> Element {
    rsx! {
        section { class: "space-y-2 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3",
            p { class: "text-sm font-medium text-zinc-100", "{title}" }
            p { class: "text-[11px] text-zinc-600", "{hint}" }
            {children}
        }
    }
}

/// 主按钮(确认 / 保存 / 生成等)。
///
/// 【是什么】高对比度主操作按钮(浅底深字)。
///
/// 【做什么】渲染一个带文案的按钮并把点击抛给调用方;不管业务语义。
///
/// 【交互逻辑】点击 → 调 `on_click.call(e)` 把 MouseEvent 抛给调用方 →
/// 由调用方决定后续(本组件不改任何状态、不发网络)。
///
/// 【样式】`rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs
/// font-medium text-zinc-900 hover:bg-zinc-300`(浅色实底 + hover 加深)。
///
/// 【子组件组成】无:仅一个 `button`。
///
/// 【数据流】
/// - 对内(入):`label`(按钮静态文案)、`on_click`(点击事件出口)。
/// - 对外(出):`on_click` 携带 MouseEvent,写回逻辑在调用方。
#[component]
pub(crate) fn PushBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-100 bg-zinc-100 px-3 py-1.5 text-xs font-medium text-zinc-900 hover:bg-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 危险操作(删除 / 停用)。
///
/// 【是什么】危险操作按钮:透明底 + 红字红边。
///
/// 【做什么】渲染一个带文案的危险按钮并把点击抛给调用方;不管业务语义、
/// 不做二次确认(确认弹窗由调用方负责)。
///
/// 【交互逻辑】点击 → 调 `on_click.call(e)` 把 MouseEvent 抛给调用方 →
/// 本组件不改任何状态、不发网络。
///
/// 【样式】`rounded-md border border-red-900/60 px-3 py-1.5 text-xs text-red-400
/// hover:border-red-700`(红边红字,hover 边色加深)。
///
/// 【子组件组成】无:仅一个 `button`。
///
/// 【数据流】
/// - 对内(入):`label`(按钮静态文案)、`on_click`(点击事件出口)。
/// - 对外(出):`on_click` 携带 MouseEvent,写回逻辑在调用方。
#[component]
pub(crate) fn DangerBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-red-900/60 px-3 py-1.5 text-xs text-red-400 hover:border-red-700",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 幽灵操作(清空 / 取消)。
///
/// 【是什么】低存在感的次要操作按钮:透明底 + 灰字灰边。
///
/// 【做什么】渲染一个带文案的幽灵按钮并把点击抛给调用方;不管业务语义。
///
/// 【交互逻辑】点击 → 调 `on_click.call(e)` 把 MouseEvent 抛给调用方 →
/// 本组件不改任何状态、不发网络。
///
/// 【样式】`rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-500
/// hover:border-zinc-600 hover:text-zinc-300`(灰边灰字,hover 提亮)。
///
/// 【子组件组成】无:仅一个 `button`。
///
/// 【数据流】
/// - 对内(入):`label`(按钮静态文案)、`on_click`(点击事件出口)。
/// - 对外(出):`on_click` 携带 MouseEvent,写回逻辑在调用方。
#[component]
pub(crate) fn GhostBtn(label: &'static str, on_click: EventHandler<MouseEvent>) -> Element {
    rsx! {
        button {
            class: "rounded-md border border-zinc-800 px-3 py-1.5 text-xs text-zinc-500 hover:border-zinc-600 hover:text-zinc-300",
            onclick: move |e| on_click.call(e),
            "{label}"
        }
    }
}

/// 状态开关(对齐 new-api 的启用/停用徽章;带 on/off 文字态)。
///
/// 【是什么】一个受控的滑动开关(pill 轨道 + 圆钮)。
///
/// 【做什么】按 `on` 决定轨道色与圆钮位移,点击把「请求翻转」抛给调用方;
/// 自己不持状态(受控组件)。
///
/// 【交互逻辑】点击 → 调 `on_toggle.call(())` 请求翻转 → 由调用方写回
/// `f_enabled` 等 signal,值再回流进 `on`;本组件不发网络。
///
/// 【样式】轨道 `relative h-5 w-9 shrink-0 rounded-full transition-colors`,
/// 按 `on` 切 `bg-zinc-100` / `bg-zinc-700`;圆钮 `absolute top-0.5 left-0.5
/// h-4 w-4 rounded-full bg-zinc-950 transition-transform`,按 `on` 切
/// `translate-x-4` / `translate-x-0`;带 `role="switch"` 与 `aria-checked`。
///
/// 【子组件组成】无:一个受控 `button` + 一个 span 圆钮。
///
/// 【数据流】
/// - 对内(入):`on`(当前开关态,由调用方 signal 提供)。
/// - 对外(出):`on_toggle`(无参,调用方据此取反并写回 signal)。
#[component]
pub(crate) fn ToggleSwitch(on: bool, on_toggle: EventHandler<()>) -> Element {
    let track = if on { "bg-zinc-100" } else { "bg-zinc-700" };
    let knob = if on { "translate-x-4" } else { "translate-x-0" };
    rsx! {
        button {
            class: "relative h-5 w-9 shrink-0 rounded-full transition-colors {track}",
            role: "switch",
            "aria-checked": "{on}",
            onclick: move |_| on_toggle.call(()),
            span { class: "absolute top-0.5 left-0.5 h-4 w-4 rounded-full bg-zinc-950 transition-transform {knob}" }
        }
    }
}
