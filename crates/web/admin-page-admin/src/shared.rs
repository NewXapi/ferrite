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

// ---- 模型别名(aliases)—— 类型 / 判定 / 文案 ----
use crate::state::AliasRow;
use crate::tab_page_groups::parse_whitelist;
use contract::api::admin::GroupDto;
pub use ui::{PriceMode, PriceModeToggle};

pub const SEC_STATS: &str = "别名概览";
pub const SEC_FILTER: &str = "筛选与操作";
pub const SEC_LIST_ALIAS: &str = "别名列表";
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
/// 筛选区新建别名按钮。
pub const BTN_NEW_ALIAS: &str = "✚ 新建别名";
/// 列表错误态重试按钮。
/// 弹窗取消按钮。
pub const BTN_CANCEL_ALIAS: &str = "取消";
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
pub const MSG_EMPTY_ALIAS: &str = "没有匹配的模型别名";
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
