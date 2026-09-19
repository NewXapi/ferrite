//! 货币管理共享类型与文案。page / list / form 三处复用。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 `list` / `form` 里),不放网络调用
//! (在 `page.rs`);这里只有 `Kind` 类型、区段文案与提示文案常量。

/// `kind` 语义对齐 0014 换算层：
/// - `Points` = 可扣费余额货币
/// - `Fiat` = 仅计价展示（不进余额）
#[derive(Clone, PartialEq)]
pub enum Kind {
    Points,
    Fiat,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Points => "points",
            Kind::Fiat => "fiat",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "fiat" {
            Kind::Fiat
        } else {
            Kind::Points
        }
    }
}

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 列表区标题。
pub const SEC_LIST: &str = "货币列表";
/// 表单区标题(新增态)。
pub const SEC_FORM: &str = "新增 / 编辑货币";
/// 页面顶部的 kind 语义与基准货币说明条。
pub const SEC_NOTE: &str = "kind=points 的货币进钱包余额并可扣费；kind=fiat 仅作计价/展示（不进余额）。基准 USD 的汇率恒为 1。";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 页面标题与 region 的 aria-label。
pub const LBL_PAGE: &str = "货币管理";
/// 列表区 region 的 aria-label。
pub const LBL_LIST_REGION: &str = "货币定义列表";
/// 表单区 region 的 aria-label。
pub const LBL_FORM_REGION: &str = "货币表单";
/// 表头:货币代号。
pub const LBL_COL_CODE: &str = "Code";
/// 表头:展示符号。
pub const LBL_COL_SYMBOL: &str = "符号";
/// 表头:名称。
pub const LBL_COL_NAME: &str = "名称";
/// 表头:货币类别。
pub const LBL_COL_KIND: &str = "kind";
/// 表头:汇率。
pub const LBL_COL_RATE: &str = "汇率";
/// 表头:小数位。
pub const LBL_COL_PRECISION: &str = "小数位";
/// 表头:启用状态。
pub const LBL_COL_STATUS: &str = "状态";
/// 行状态:启用。
pub const LBL_STATUS_ENABLED: &str = "启用";
/// 行状态/行操作:停用。
pub const LBL_STATUS_DISABLED: &str = "停用";
/// 表单字段:名称。
pub const LBL_FIELD_NAME: &str = "名称";
/// 表单字段:备注。
pub const LBL_FIELD_REMARK: &str = "备注";

// ---- FIELD_* : 表单字段标签 ----

/// 表单字段:货币代号。
pub const FIELD_CODE: &str = "Code";
/// 表单字段:展示符号。
pub const FIELD_SYMBOL: &str = "符号（如 ¥ / $ / P）";
/// 表单字段:货币类别下拉。
pub const FIELD_KIND: &str = "kind";
/// 表单字段:汇率。
pub const FIELD_RATE: &str = "汇率（1 单位 = 多少内部单位，500_000 = $1）";
/// 表单字段:小数位。
pub const FIELD_PRECISION: &str = "小数位（precision）";
/// 表单字段:启用勾选框。
pub const FIELD_ENABLED: &str = "启用";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// kind 下拉:余额货币。
pub const OPT_KIND_POINTS: &str = "points（余额货币）";
/// kind 下拉:仅计价展示。
pub const OPT_KIND_FIAT: &str = "fiat（仅计价展示）";

// ---- BTN_* : 按钮文案 ----

/// 列表错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 表头行操作:编辑。
pub const BTN_EDIT: &str = "编辑";
/// 表头行操作:停用。
pub const BTN_DISABLE: &str = "停用";
/// 表单编辑态提交按钮。
pub const BTN_SAVE_CHANGES: &str = "保存修改";
/// 表单新增态提交按钮。
pub const BTN_CREATE: &str = "创建货币";
/// 表单「取消(转新增)」按钮。
pub const BTN_CANCEL: &str = "取消（转新增）";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 列表空态占位。
pub const MSG_EMPTY: &str = "还没有货币定义。用下方表单创建第一个货币。";
/// 数据拉取失败前缀(后接错误详情)。
pub const MSG_LOAD_FAILED_PREFIX: &str = "数据加载失败：";
/// 表单校验:代号必填。
pub const MSG_ERR_CODE_REQUIRED: &str = "code 必填";
/// 表单校验:fiat 必配展示符号。
pub const MSG_ERR_FIAT_SYMBOL: &str = "fiat 货币必须配展示符号（如 ¥ / $）";
/// 表单校验:fiat 小数位下限。
pub const MSG_ERR_FIAT_PRECISION: &str = "fiat 货币 precision 必须 ≥ 1";
/// 表单校验:新增时汇率须为正。
pub const MSG_ERR_RATE_POSITIVE: &str = "internal_rate 必须为正数";
/// 表单字段旁的 USD 汇率锁定说明。
pub const MSG_USD_LOCKED: &str = "USD 是基准货币，汇率恒为 1，不可修改";
/// 表单底部警示:汇率影响面。
pub const MSG_WARN_RATE: &str = "修改汇率会实时影响全体用户可用额度（历史交易不锁汇率）。";
/// 表单底部警示:删除即软禁用。
pub const MSG_WARN_DISABLE: &str = "「删除」即软禁用：余额非零的货币不可物理删除，仅可停用。";
/// 编辑成功提示后缀(前接货币代号)。
pub const MSG_OK_UPDATED_SUFFIX: &str = " 已更新";
/// 新建成功提示后缀(前接货币代号)。
pub const MSG_OK_CREATED_SUFFIX: &str = " 已创建";
/// 停用成功提示后缀(前接货币代号)。
pub const MSG_OK_DISABLED_SUFFIX: &str = " 已停用";
