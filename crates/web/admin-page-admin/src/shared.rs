//! 跨组件共享的文案与样式常量(i18n 第 1 层 + 语义色 token)。
//!
//! 按组件族分段;常量值即原 rsx 字面量,逐字符一致保证零渲染变化。
//! `components/` 只放 `#[component]` 组件,常量一律挂本根级模块(spec 理念 1)。

// ---- 网关渠道健康(gateway)----

// ---- 语义色（非文案，与文案同文件便于表头/行共用） ----

/// cooling=红系。
pub const TONE_COOLING: &str = "border-red-500/30 bg-red-500/15 text-red-300";
/// slow_start=黄系。
pub const TONE_SLOW_START: &str = "border-amber-500/30 bg-amber-500/15 text-amber-300";
/// ok=绿系。
pub const TONE_OK: &str = "border-emerald-500/30 bg-emerald-500/15 text-emerald-400";

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 面板标题与 region 的 aria-label。
pub const SEC_PANEL: &str = "网关渠道健康";
/// 空态主文案(说明空列表是正常态)。
pub const SEC_EMPTY_NOTE: &str = "暂无渠道健康记录——正常态";
/// 空态补充说明(后端只上报出过错的项目)。
pub const SEC_EMPTY_HINT: &str = "网关只上报发生过错的渠道;全部健康时列表为空";
/// 错误态标题。
pub const SEC_LOAD_FAILED: &str = "网关健康拉取失败";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 表头计数徽标:轮询进行中。
pub const LBL_POLLING: &str = "轮询中 · 5s";
/// 表头计数徽标:无异常项、已停止轮询。
pub const LBL_SYNCED: &str = "已同步";
/// 表头计数徽标:冷却项计数前缀(后接数量)。
pub const LBL_COUNT_COOLING_PREFIX: &str = "冷却 ";
/// 表头计数徽标:慢启动项计数前缀(后接数量)。
pub const LBL_COUNT_SLOW_PREFIX: &str = "慢启动 ";
/// 表头计数徽标:正常项计数前缀(后接数量)。
pub const LBL_COUNT_OK_PREFIX: &str = "正常 ";
/// 行内冷却倒计时前缀(后接秒数)。
pub const LBL_REMAINING_PREFIX: &str = "余 ";

// ---- BTN_* : 按钮文案 ----

/// 面板手动刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 错误态重试按钮。
pub const BTN_RETRY: &str = "重试";

// ---- MSG_* : 提示 / 空态文案 ----

/// 渠道名与模型名都缺省时的行内回退前缀。
pub const MSG_CHANNEL_FALLBACK_PREFIX: &str = "未同步渠道 (";
/// 渠道名与模型名都缺省时的行内回退后缀(前接 unit_key 前 6 位)。
pub const MSG_CHANNEL_FALLBACK_SUFFIX: &str = "…)";
/// 模型名缺省且 unit_key 无 `:` 分段时的徽标回退。
pub const MSG_MODEL_FALLBACK: &str = "未知模型";

// ---- 货币管理(currency)----

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
