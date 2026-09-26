//! 模型别名管理共享类型、判定与文案。page / list / modal 三处复用。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 stats / toolbar / list / modal 里),不放网络
//! 调用(在 `page.rs`);`PriceMode` 与 `PriceModeToggle` 的定义已上移到 ui-components,
//! 这里只做 `pub use` 再导出以维持既有导入路径。

use contract::api::admin::GroupDto;

use crate::state::AliasRow;
use crate::tab_page_groups::parse_whitelist;

pub const SEC_STATS: &str = "别名概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST: &str = "别名列表";
/// 别名列表项:后端 ModelView 的 key(UUID) + 页面展示行 + per-card 定价模式。
/// key 不并入 AliasRow — AliasRow 被 entities.rs 结构体字面量构造,
/// 本页独立持有 key 以定位 PUT/DELETE 路径;price_mode 是 UI 层本地状态,
/// 后端 models 域无 pricing_mode 列,保存不写回。
#[derive(Clone, PartialEq)]
pub struct AliasItem {
    pub key: String,
    pub row: AliasRow,
    pub price_mode: PriceMode,
}

/// 弹窗状态（仅剩新建：编辑已改走卡片行内 Popover，见决策记录 §2.2）
#[derive(Clone, PartialEq)]
pub enum AliasModalState {
    Closed,
    New,
}

/// 计算「可用此别名的分组及其倍率」。
///
/// 后端 `model_whitelist` 是「分组内可用的模型名列表」;空白名单 = 该分组
/// 可用全部模型。因此「可用此别名」= 白名单为空(默认全可用)或显式包含
/// 该别名。原型新卡与旧卡片网格共用此判定,避免两处逻辑分叉。
///
/// 参数:
/// - `alias`:别名标识(后端 `ModelView.name`),与白名单里的模型名做精确比较。
/// - `groups`:全部候选分组(页面 `list_groups_api` 拉到),不区分是否已有别名。
///
/// 返回 `(分组名, 该分组倍率 ratio)` 列表,保持传入 `groups` 的顺序;
/// 未命中任何分组时返回空 `Vec`。纯函数,无副作用、不发网络。
pub fn usable_groups_for(alias: &str, groups: &[GroupDto]) -> Vec<(String, f64)> {
    groups
        .iter()
        .filter(|g| {
            let names = parse_whitelist(&g.model_whitelist);
            names.is_empty() || names.iter().any(|n| n.as_str() == alias)
        })
        .map(|g| (g.name.clone(), g.ratio))
        .collect()
}

// ============ 定价模式 toggle（已上提到 ui-components，卡片/弹窗/本页共用） ============

// `PriceMode` 与 `PriceModeToggle` 原定义于此；卡片改由 ui-components 的
// `AliasCard` 承载后上提到该 crate（避免 ui-components 反向依赖页面 crate）。
// 此处再导出，保证 `crate::tab_page_aliases::{PriceMode, PriceModeToggle}`
// 这一既有路径继续可用，公开面零变化。
pub use ui::{PriceMode, PriceModeToggle};

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE: &str = "按倍率与资费规则快速筛选";
/// 页面顶部的数据来源与写路径说明条。
pub const SEC_DATA_NOTE: &str = "别名来自真实 /api/models;编辑与删除已接后端;定价模式与价格配置为 UI 层本地状态,后端扩展 pricing 列前保存不写库;新建暂未开放(后端需要 owner/api_key 字段)";
/// 按量 tab 内,定价模式开关的补充说明。
pub const SEC_MODE_NOTE: &str = "切换到按量定价后,补充通道的启用开关在「按量定价」tab";
/// 按次 tab 内的后端落地说明。
pub const SEC_PER_CALL_NOTE: &str = "后端落地前按次价格暂存于倍率字段,仅 UI 层生效。";

// ---- TTL_* : 弹窗标题 ----

/// 新建别名时的弹窗标题。
pub const TTL_NEW: &str = "新建模型别名";

// ---- TAB_* : 弹窗内页签标题 ----

/// 弹窗「基本」页签。
pub const TAB_BASIC: &str = "基本";
/// 弹窗「按量定价」页签。
pub const TAB_PER_TOKEN: &str = "按量定价";
/// 弹窗「按次定价」页签。
pub const TAB_PER_CALL: &str = "按次定价";

// ---- FIELD_* : 表单字段标签 ----

/// 别名标识输入框标签。
pub const FIELD_ALIAS_ID: &str = "别名标识 (API 请求匹配名)";
/// 展示名称输入框标签。
pub const FIELD_DISPLAY: &str = "展示名称 (可选)";
/// 计费倍率输入框标签。
pub const FIELD_MULTIPLIER: &str = "计费倍率 (multiplier ≥ 0)";
/// 定价模式选择器标签。
pub const FIELD_PRICE_MODE: &str = "定价模式 (启用哪种定价)";
/// 输入价格输入框标签。
pub const FIELD_INPUT_PRICE: &str = "输入价格";
/// 单次调用价格输入框标签。
pub const FIELD_PER_CALL_PRICE: &str = "单次调用价格";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:总别名数。
pub const LBL_STAT_TOTAL: &str = "总别名数";
/// 概览卡:标准倍率别名数。
pub const LBL_STAT_STANDARD: &str = "标准 1.0× 别名";
/// 概览卡:自定倍率别名数。
pub const LBL_STAT_CUSTOM: &str = "自定倍率别名";
/// 概览卡:免费别名数。
pub const LBL_STAT_FREE: &str = "免费别名 (0×)";
/// 概览卡:平均加价倍率。
pub const LBL_STAT_AVG: &str = "平均加价倍率";
/// 卡片网格的 aria-label。
pub const LBL_ALIAS_LIST: &str = "别名列表";
/// 弹窗页签栏的 aria-label。
pub const LBL_MODAL_TABLIST: &str = "别名编辑选项";
/// 输入价格行下方的说明。
pub const LBL_INPUT_PRICE_DESC: &str = "每 100 万输入 token 的价格。";
/// 单次调用价格行下方的说明。
pub const LBL_PER_CALL_DESC: &str = "每次调用(不论 token 数)固定扣费。";
/// 补充通道:输出价格。
pub const LBL_CH_OUTPUT: &str = "输出价格";
/// 补充通道:缓存读取价格。
pub const LBL_CH_CACHE_READ: &str = "缓存读取价格";
/// 补充通道:缓存写入价格。
pub const LBL_CH_CACHE_WRITE: &str = "缓存写入价格";
/// 补充通道:补全价格。
pub const LBL_CH_COMPLETION: &str = "补全价格";
/// 输出价格行的悬停说明。
pub const LBL_CH_OUTPUT_DESC: &str = "生成内容的输出 token 价格(悬停标题查看)";
/// 缓存读取价格行的悬停说明。
pub const LBL_CH_CACHE_READ_DESC: &str = "缓存读取 token 价格(悬停标题查看)";
/// 缓存写入价格行的悬停说明。
pub const LBL_CH_CACHE_WRITE_DESC: &str = "缓存写入 token 价格(悬停标题查看)";
/// 补全价格行的悬停说明。
pub const LBL_CH_COMPLETION_DESC: &str = "补全(输出)调用的 token 价格(悬停标题查看)";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// 计数徽标:加载中替代文案。
pub const OPT_BADGE_LOADING: &str = "加载中…";
/// 分级胶囊:全部档位。
pub const OPT_ALL: &str = "全部";
/// 分级胶囊:标准 1.0× 档位。
pub const OPT_STANDARD: &str = "标准 1.0×";
/// 分级胶囊:自定倍率档位。
pub const OPT_CUSTOM: &str = "自定倍率";
/// 分级胶囊:免费通道档位。
pub const OPT_FREE: &str = "免费通道";

// ---- BTN_* : 按钮文案 ----

/// 筛选区刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 筛选区新建别名按钮。
pub const BTN_NEW_ALIAS: &str = "✚ 新建别名";
/// 列表错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 弹窗取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 弹窗新建态提交按钮。
pub const BTN_CREATE_ALIAS: &str = "创建别名";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 输入框占位:别名标识示例。
pub const MSG_PH_ALIAS_ID: &str = "例如: gpt-4o, claude-3-5-sonnet";
/// 输入框占位:展示名称示例。
pub const MSG_PH_DISPLAY: &str = "例如: GPT-4o 旗舰模型";
/// 输入框占位:单次调用价格示例。
pub const MSG_PH_PER_CALL: &str = "例如: 0.05";
/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER: &str = "搜索别名 ID 或展示名称 (如 gpt-4o, claude-sonnet)...";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED: &str = "加载别名失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST: &str = "正在加载模型别名…";
/// 列表空态占位。
pub const MSG_EMPTY: &str = "没有匹配的模型别名";
/// 删除成功提示。
pub const MSG_DELETED: &str = "已删除";
/// 保存失败提示前缀(后接错误详情)。
pub const MSG_SAVE_FAILED: &str = "保存失败:";
/// 行内 Popover 提交别名为空时的提示。
pub const MSG_NAME_REQUIRED: &str = "别名不能为空";
/// 行内 Popover 提交数值字段解析失败时的提示。
pub const MSG_NUM_INVALID: &str = "数值无效,已保留原值";
/// 删除失败提示前缀(后接错误详情)。
pub const MSG_DELETE_FAILED: &str = "删除失败:";
/// 新建被诚实拒绝的提示。
pub const MSG_CREATE_REJECTED: &str =
    "新建未执行:后端创建模型需要 owner 与 api_key 字段,当前表单未提供";
