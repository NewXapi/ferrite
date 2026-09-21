//! 分组管理共享类型、纯函数与文案常量。page / toolbar / list / modal 四处复用。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 stats / toolbar / list / modal 里),不放网络
//! 调用(在 `page.rs`);`WriteOp` 只表达「要做什么写操作」,不含任何请求实现。

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum ModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类(目前仅删除;工厂保留扩展位)
#[derive(Clone, Copy)]
pub enum WriteOp {
    Delete,
    /// 启用/停用切换 (status: 1=启用, 2=停用)
    ToggleStatus(i16),
    /// 倍率滑条拖动写回 (ratio)
    SetRatio(f64),
}

/// 把用户输入的逗号/分号分隔白名单拆成模型名数组(去空、trim)。
/// 与后端 `validate_whitelist` 对齐:每项必须是非空字符串。
pub fn parse_whitelist_raw(raw: &str) -> Vec<String> {
    raw.split([',', '，', ';', '；'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

/// 从后端返回的 `model_whitelist` JSON(字符串数组或空)取回白名单。
pub fn parse_whitelist(v: &serde_json::Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m.as_str().map(ToOwned::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 概览统计区标题。
pub const SEC_STATS: &str = "分组概览";
/// 筛选与操作区标题。
pub const SEC_FILTER: &str = "筛选与操作";
/// 卡片网格区标题。
pub const SEC_LIST: &str = "分组列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE: &str = "按倍率分级或关键词筛选";
/// 页面根节点的 aria-label。
pub const SEC_PAGE_ARIA: &str = "分组管理";

// ---- TTL_* : 弹窗标题 ----

/// 编辑分组时的弹窗标题。
pub const TTL_EDIT: &str = "编辑分组";
/// 新建分组时的弹窗标题。
pub const TTL_NEW: &str = "新建分组";

// ---- TAB_* : 弹窗内页签标题 ----

/// 弹窗「分组信息」页签。
pub const TAB_BASIC: &str = "分组信息";
/// 弹窗「映射别名」页签。
pub const TAB_ALIAS: &str = "映射别名";

// ---- FIELD_* : 表单字段标签 ----

/// 分组标识输入框标签。
pub const FIELD_GROUP_NAME: &str = "分组标识 (英文唯一标识)";
/// 展示备注输入框标签。
pub const FIELD_REMARK: &str = "展示备注 (可选)";
/// 模型白名单输入框标签。
pub const FIELD_WHITELIST: &str = "模型白名单 (逗号分隔,可选)";
/// 计费倍率输入框标签。
pub const FIELD_RATIO: &str = "计费倍率 (ratio ≥ 0)";
/// 映射别名输入框标签。
pub const FIELD_ALIAS: &str = "映射别名 (多选,逗号分隔)";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:总分组数。
pub const LBL_STAT_TOTAL: &str = "总分组数";
/// 概览卡:启用中分组数。
pub const LBL_STAT_ENABLED: &str = "启用中";
/// 概览卡:已停用分组数。
pub const LBL_STAT_DISABLED: &str = "已停用";
/// 概览卡:平均倍率。
pub const LBL_STAT_AVG_RATIO: &str = "平均倍率";
/// 概览卡:非基准倍率分组数。
pub const LBL_STAT_CUSTOM_RATIO: &str = "非基准倍率";
/// 卡片头部:默认分组标签(兼作悬停标题)。
pub const LBL_DEFAULT: &str = "默认";
/// 卡片徽标:系统内置分组。
pub const LBL_BUILTIN: &str = "系统内置";
/// 卡片指标行:计费倍率。
pub const LBL_RATIO: &str = "计费倍率";
/// 卡片指标行:100 额度实扣。
pub const LBL_EXAMPLE_COST: &str = "100额度实扣";
/// 卡片指标行:调度作用域。
pub const LBL_SCOPE: &str = "调度作用域";
/// 卡片指标行:调度作用域取值。
pub const LBL_SCOPE_VALUE: &str = "全模型匹配";
/// 卡片倍率徽标:基准倍率前缀(后接 `{m:.2}×`)。
pub const LBL_MULT_BASELINE_PREFIX: &str = "基准 ";
/// 卡片倍率徽标:优惠倍率前缀(后接 `{m:.2}× (-{discount}%)`)。
pub const LBL_MULT_DISCOUNT_PREFIX: &str = "优惠 ";
/// 卡片倍率徽标:溢价倍率前缀(后接 `{m:.2}× (+{markup}%)`)。
pub const LBL_MULT_MARKUP_PREFIX: &str = "溢价 ";
/// 弹窗关闭按钮的 aria-label。
pub const LBL_CLOSE: &str = "关闭";
/// 计费预览行:该分组实际扣费。
pub const LBL_ACTUAL_COST: &str = "该分组实际扣费";
/// 批量多选区标题。
pub const LBL_BULK_SELECT: &str = "批量操作: 点选分组";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// 卡片网格计数徽标:加载中替代文案。
pub const OPT_BADGE_LOADING: &str = "加载中…";
/// 分级胶囊:全部档位。
pub const OPT_ALL: &str = "全部";
/// 分级胶囊:启用中档位。
pub const OPT_ENABLED: &str = "启用中";
/// 分级胶囊:已停用档位。
pub const OPT_DISABLED: &str = "已停用";
/// 快捷倍率预设:0.5× 半价。
pub const OPT_RATIO_HALF: &str = "0.5× 半价";
/// 快捷倍率预设:0.8× 优惠。
pub const OPT_RATIO_DISCOUNT: &str = "0.8× 优惠";
/// 快捷倍率预设:1.0× 基准。
pub const OPT_RATIO_BASELINE: &str = "1.0× 基准";
/// 快捷倍率预设:1.2× 溢价。
pub const OPT_RATIO_MARKUP: &str = "1.2× 溢价";
/// 快捷倍率预设:1.5× 高配。
pub const OPT_RATIO_HIGH: &str = "1.5× 高配";
/// 快捷倍率预设:2.0× 双倍。
pub const OPT_RATIO_DOUBLE: &str = "2.0× 双倍";
/// 快捷倍率预设区标题。
pub const OPT_RATIO_PRESETS: &str = "快捷倍率预设";

// ---- BTN_* : 按钮文案 ----

/// 筛选区刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 筛选区新建分组按钮。
pub const BTN_NEW_GROUP: &str = "✚ 新建分组";
/// 列表错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 卡片编辑按钮。
pub const BTN_EDIT: &str = "编辑";
/// 卡片停用按钮(分组启用中时显示)。
pub const BTN_DISABLE: &str = "停用";
/// 卡片启用按钮(分组已停用时显示)。
pub const BTN_ENABLE: &str = "启用";
/// 卡片删除按钮。
pub const BTN_DELETE: &str = "删除";
/// 默认分组不可删时的占位按钮。
pub const BTN_BUILTIN: &str = "内置";
/// 弹窗取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 弹窗编辑态提交按钮。
pub const BTN_SAVE_CHANGES: &str = "保存修改";
/// 弹窗新建态提交按钮。
pub const BTN_CREATE_GROUP: &str = "创建分组";
/// 批量动作条:批量启用。
pub const BTN_BULK_ENABLE: &str = "批量启用";
/// 批量动作条:批量停用。
pub const BTN_BULK_DISABLE: &str = "批量停用";
/// 批量动作条:清除选中。
pub const BTN_BULK_CLEAR: &str = "清除";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER: &str = "搜索分组标识或备注...";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED: &str = "加载分组失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST: &str = "正在加载分组…";
/// 列表空态占位。
pub const MSG_EMPTY: &str = "没有匹配的分组";
/// 写操作成功提示。
pub const MSG_OP_OK: &str = "操作成功";
/// 写操作失败提示前缀(后接错误详情)。
pub const MSG_OP_FAILED: &str = "操作失败:";
/// 批量操作全部成功的提示。
pub const MSG_BULK_OK: &str = "批量操作成功";
/// 批量操作汇总通知前缀。
pub const MSG_BULK_PREFIX: &str = "批量操作：";
/// 批量操作汇总通知中「成功」前的数字与单位后缀。
pub const MSG_BULK_OK_SUFFIX: &str = " 成功，";
/// 批量操作汇总通知中「失败」前的数字与单位后缀。
pub const MSG_BULK_FAIL_SUFFIX: &str = " 失败（key: ";
/// 批量操作汇总通知的结尾括号。
pub const MSG_BULK_TAIL: &str = "）";
/// 批量动作条已选计数前缀。
pub const MSG_BULK_SELECTED_PREFIX: &str = "已选 ";
/// 批量动作条已选计数后缀。
pub const MSG_BULK_SELECTED_SUFFIX: &str = " 项";
/// 分组标识输入框占位。
pub const MSG_PH_GROUP_NAME: &str = "例如: vip, claude, fast";
/// 展示备注输入框占位。
pub const MSG_PH_REMARK: &str = "例如: VIP会员专线、高峰备用组";
/// 模型白名单输入框占位。
pub const MSG_PH_WHITELIST: &str = "例如: gpt-4o, claude-3.5";
/// 映射别名输入框占位。
pub const MSG_PH_ALIAS: &str = "例如: gpt-4o, claude-3.5";
/// 模型白名单输入行下方的说明。
pub const MSG_WHITELIST_HINT: &str = "留空 = 全模型可用;填了 = 仅这些模型";
/// 映射别名输入行下方的说明。
pub const MSG_ALIAS_HINT: &str = "本 MVP 仅登记, 后端暂无映射列; 留空 = 不映射";
/// 默认分组标识旁的只读说明。
pub const MSG_DEFAULT_LOCKED: &str = "默认分组标识不可更改";
/// 映射别名候选为空时的提示。
pub const MSG_NO_ALIAS_OPTIONS: &str = "暂无可选模型别名";
