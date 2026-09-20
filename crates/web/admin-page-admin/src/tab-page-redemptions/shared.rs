//! 兑换码管理共享类型、视图映射与文案。page / card / modal 三处复用。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 stats / toolbar / list / card / modal 里),
//! 不放网络调用(在 `page.rs`);`map_redemption_view` 只做后端 DTO → 页面视图模型的
//! 纯换算(含 `quota` 内部单位 → ¥),不触发任何请求。

use crate::api::RedemptionView;

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum RedModalState {
    Closed,
    Generate,
    /// 生成成功后的一次性明文码展示
    Codes(Vec<String>),
}

/// 页面内兑换码视图模型 (金额已换算为 ¥)。
#[derive(Debug, Clone, PartialEq)]
pub struct RedRowFE {
    pub key: String,
    pub code_preview: String,
    /// 页面展示使用的 CNY 金额（后端 `quota` 按 500000 单位换算）。
    pub quota_cny: f64,
    pub status: u8, // 1 未使用 / 2 已核销 / 3 已停用
    pub redeemed_by: Option<String>,
    pub redeemed_at: String,
    pub created: String,
}

/// 把后端 `RedemptionView` 映射为页面视图模型。
///
/// 金额换算:后端 `quota` 是内部计费单位(500000 = ¥1),
/// 页面统一以 ¥ 展示。`redeemed_at` 缺省为空串(卡片据此隐藏核销时间行)。
pub fn map_redemption_view(v: RedemptionView) -> RedRowFE {
    RedRowFE {
        key: v.key,
        code_preview: v.code_preview,
        quota_cny: v.quota as f64 / 500_000.0,
        status: v.status as u8,
        redeemed_by: v.redeemed_by,
        redeemed_at: v.redeemed_at.unwrap_or_default(),
        created: v.created_at,
    }
}

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 概览统计区标题。
pub const SEC_STATS: &str = "兑换码概览";
/// 筛选与操作区标题。
pub const SEC_FILTER: &str = "筛选与操作";
/// 卡片网格区标题。
pub const SEC_LIST: &str = "兑换码列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE: &str = "按状态分级筛选;停用后不可重新启用";

// ---- TTL_* : 弹窗标题 ----

/// 批量生成弹窗标题。
pub const TTL_GENERATE: &str = "生成批量兑换码";
/// 明文码展示弹窗标题前缀(后接 `{n} 张明文卡密(仅此一次)`)。
pub const TTL_GENERATED_PREFIX: &str = "生成成功 · ";
/// 明文码展示弹窗标题后缀(前接张数)。
pub const TTL_GENERATED_SUFFIX: &str = " 张明文卡密(仅此一次)";

// ---- FIELD_* : 表单字段标签 ----

/// 生成数量输入框标签。
pub const FIELD_COUNT: &str = "生成数量 (1-100)";
/// 单张面额输入框标签。
pub const FIELD_QUOTA: &str = "单张面额 (元)";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:兑换码总数。
pub const LBL_STAT_TOTAL: &str = "兑换码总数";
/// 概览卡:未使用兑换码数。
pub const LBL_STAT_UNUSED: &str = "未使用";
/// 概览卡:已核销兑换码数。
pub const LBL_STAT_USED: &str = "已核销";
/// 概览卡:已停用兑换码数。
pub const LBL_STAT_DISABLED: &str = "已停用";
/// 概览卡:可用面额合计。
pub const LBL_STAT_AVAILABLE: &str = "可用面额";
/// 卡片状态徽标:未使用。
pub const LBL_STATUS_UNUSED: &str = "未使用";
/// 卡片状态徽标:已核销。
pub const LBL_STATUS_USED: &str = "已核销";
/// 卡片状态徽标:已停用。
pub const LBL_STATUS_DISABLED: &str = "已停用";
/// 卡片状态徽标:异常值兜底。
pub const LBL_STATUS_UNKNOWN: &str = "未知状态";
/// 卡片面值徽标前缀(后接 `¥{:.2}`)。
pub const LBL_FACE_VALUE_PREFIX: &str = "面值 ¥";
/// 卡片指标行:可用面额。
pub const LBL_AVAILABLE_QUOTA: &str = "可用面额";
/// 卡片指标行:生成时间。
pub const LBL_CREATED: &str = "生成时间";
/// 卡片指标行:兑换人。
pub const LBL_REDEEMED_BY: &str = "兑换人";
/// 卡片指标行:核销时间。
pub const LBL_REDEEMED_AT: &str = "核销时间";
/// 卡片复制按钮的悬停提示与 aria-label 前缀(后接 key)。
pub const LBL_CARD_ARIA_PREFIX: &str = "兑换码 ";
/// 列表区域的 aria-label。
pub const LBL_LIST_ARIA: &str = "兑换码列表";
/// 列表错误态区域的 aria-label。
pub const LBL_ERROR_ARIA: &str = "兑换码加载失败";
/// 生成弹窗中的批次测算行标签。
pub const LBL_BATCH: &str = "生成总批次";
/// 生成弹窗中的发行总额行标签。
pub const LBL_TOTAL_VALUE: &str = "发行总面值金额";
/// 快捷面额预设区标题。
pub const LBL_QUOTA_PRESETS: &str = "快捷面额预设";
/// 筛选与操作区的 aria-label。
pub const LBL_FILTER_ARIA: &str = "兑换码筛选与操作";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// 卡片网格计数徽标:加载中替代文案。
pub const OPT_BADGE_LOADING: &str = "加载中…";
/// 分级胶囊:全部档位。
pub const OPT_ALL: &str = "全部";
/// 分级胶囊:未使用档位。
pub const OPT_UNUSED: &str = "未使用";
/// 分级胶囊:已核销档位。
pub const OPT_USED: &str = "已核销";
/// 分级胶囊:已停用档位。
pub const OPT_DISABLED: &str = "已停用";

// ---- BTN_* : 按钮文案 ----

/// 筛选区生成兑换码按钮。
pub const BTN_GENERATE: &str = "✚ 生成兑换码";
/// 列表错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 卡片复制按钮的常态文案。
pub const BTN_COPY: &str = "复制预览";
/// 卡片复制按钮的复制成功态文案。
pub const BTN_COPIED: &str = "已复制";
/// 卡片停用按钮(兑换码未使用时显示)。
pub const BTN_DISABLE: &str = "停用";
/// 卡片已核销状态的禁用占位按钮。
pub const BTN_REDEEMED: &str = "已核销";
/// 卡片已停用状态的禁用占位按钮。
pub const BTN_DISABLED: &str = "已停用";
/// 生成弹窗取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 生成弹窗提交按钮。
pub const BTN_SUBMIT_GENERATE: &str = "立即批量生成";
/// 明文码展示弹窗的关闭按钮。
pub const BTN_CLOSE_SAVED: &str = "我已保存,关闭";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER: &str = "搜索兑换码预览 (如 fx-086c****) ...";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED: &str = "加载兑换码失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST: &str = "正在加载兑换码…";
/// 列表空态占位。
pub const MSG_EMPTY: &str = "没有匹配的兑换码";
/// 生成弹窗底部的明文码一次性提示。
pub const MSG_GENERATE_HINT: &str =
    "提示: 明文卡密只在生成后显示一次,请及时复制保存;后端暂不支持活动名与有效期字段。";
/// 明文码展示弹窗中的醒目提示。
pub const MSG_CODES_WARNING: &str = "以下明文卡密关闭本窗口后无法再次查看,请立即复制保存。";
