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

// ---- channels tab ----
use contract::api::admin::ChannelDto;

/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum ChannelModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类:在写回工厂里区分启停与删除
#[derive(Clone, Copy)]
pub enum WriteOp {
    Toggle(i16),
    Delete,
}

/// 渠道列表筛选纯函数:按关键词(名称/类型/地址/分组,大小写不敏感)+
/// 状态档位(0=全部, 1=启用中, 2=已停用)过滤。
///
/// 抽成模块级 `pub` 纯函数,便于在同层 `tests/` 做无 runtime 的纯函数单测。
///
/// 参数:
/// - `list`:全量渠道(页面 `channels` signal 的值)。
/// - `query`:搜索词,会先 `trim` + 转小写;空串 = 不过滤。命中任一字段即保留
///   (名称 / 类型 / `base_url` / 任一分组名)。
/// - `tier`:状态档,1 取 `status == 1`,2 取 `status != 1`(即「已停用」),
///   其余值(含 0)表示全部。
///
/// 返回过滤后的克隆列表,保持原顺序。纯函数,不改入参、不发网络。
pub fn filter_channels(list: &[ChannelDto], query: &str, tier: usize) -> Vec<ChannelDto> {
    let q = query.trim().to_lowercase();
    list.iter()
        .filter(|c| {
            if !q.is_empty()
                && !c.name.to_lowercase().contains(&q)
                && !c.channel_type.to_lowercase().contains(&q)
                && !c.base_url.to_lowercase().contains(&q)
                && !c.groups.iter().any(|g| g.to_lowercase().contains(&q))
            {
                return false;
            }
            match tier {
                1 => c.status == 1,
                2 => c.status != 1,
                _ => true,
            }
        })
        .cloned()
        .collect()
}

/// 弹窗「绑定分组」输入解析:逗号分隔,trim,去空项(保持输入顺序)。
///
/// 入参为分组的原始输入串(非 `Vec`),输出为分组名列表;不做去重、不校验分组
/// 是否存在(候选校验由后端与弹窗 chips 负责)。纯函数。
pub fn parse_group_input(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// 弹窗「API Key」输入解析:换行分隔多 Key,trim,去空行(保持输入顺序)。
///
/// 由 `modal.rs` 的提交流程调用;空串会得到空 `Vec` —— 编辑态据此让 `keys` 字段
/// 整体缺席(保持现有密钥),而非发送空数组。纯函数。
pub fn parse_keys_input(raw: &str) -> Vec<String> {
    raw.lines()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 概览统计区标题。
pub const SEC_STATS_CHANNELS: &str = "渠道概览";
/// 筛选与操作区标题。
/// 卡片网格区标题。
pub const SEC_LIST_CHANNELS: &str = "渠道列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE_CHANNELS: &str = "按状态或关键词筛选";

// ---- TTL_* : 弹窗标题 ----

/// 编辑渠道时的弹窗标题。
pub const TTL_EDIT: &str = "编辑渠道";
/// 新建渠道时的弹窗标题。
pub const TTL_NEW_CHANNELS: &str = "新建渠道";

// ---- FIELD_* : 表单字段标签 ----

/// 渠道类型下拉标签。
pub const FIELD_CHANNEL_TYPE: &str = "渠道类型";
/// 绑定分组选择区标签。
pub const FIELD_BOUND_GROUPS: &str = "绑定分组（点选，可多选）";
/// 渠道名称输入框标签。
pub const FIELD_CHANNEL_NAME: &str = "渠道名称";
/// Base URL 输入框标签。
pub const FIELD_BASE_URL: &str = "Base URL (代理或官方地址)";
/// API Key 输入框标签。
pub const FIELD_API_KEY: &str = "API Key (多 Key 可换行)";
/// 备注输入框标签。
pub const FIELD_REMARK: &str = "备注 (可选)";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:总渠道数。
pub const LBL_STAT_TOTAL_CHANNELS: &str = "总渠道数";
/// 概览卡:正常启用数。
pub const LBL_STAT_ENABLED: &str = "正常启用";
/// 概览卡:停用/异常数。
pub const LBL_STAT_DISABLED: &str = "停用/异常";
/// 概览卡:密钥总数。
pub const LBL_STAT_KEYS: &str = "密钥总数";
/// 概览卡:绑定分组数。
pub const LBL_STAT_GROUPS: &str = "绑定分组数";
/// 卡片状态徽标:启用中。
pub const LBL_STATUS_ENABLED_CHANNELS: &str = "启用中";
/// 卡片状态徽标:已停用。
pub const LBL_STATUS_DISABLED_CHANNELS: &str = "已停用";
/// 卡片指标行:接口地址。
pub const LBL_BASE_URL: &str = "接口地址";
/// 卡片指标行:权重。
pub const LBL_WEIGHT: &str = "权重";
/// 卡片指标行:备注。
pub const LBL_REMARK: &str = "备注";
// ---- BTN_* : 按钮文案 ----

/// 筛选区刷新按钮。
/// 筛选区新建渠道按钮。
pub const BTN_NEW_CHANNEL: &str = "✚ 新建渠道";
/// 列表错误态重试按钮。
/// 卡片编辑按钮。
/// 卡片删除按钮的悬停提示。
pub const BTN_DELETE_TITLE: &str = "删除渠道";
/// 卡片停用按钮(渠道启用中时显示)。
/// 卡片启用按钮(渠道已停用时显示)。
pub const BTN_ENABLE: &str = "启用";
/// 弹窗取消按钮。
pub const BTN_CANCEL_CHANNELS: &str = "取消";
/// 弹窗编辑态提交按钮。
/// 弹窗新建态提交按钮。
pub const BTN_CREATE_CHANNEL: &str = "创建渠道";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// 计数徽标:加载中替代文案。
/// 分级胶囊:全部档位。
/// 分级胶囊:启用中档位。
pub const OPT_ENABLED: &str = "启用中";
/// 分级胶囊:已停用档位。
pub const OPT_DISABLED: &str = "已停用";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER_CHANNELS: &str = "搜索渠道名称、类型、分组或 API 目标地址...";
/// 渠道名称输入框占位。
pub const MSG_PH_CHANNEL_NAME: &str = "例如: OpenAI 官方, Azure East";
/// Base URL 输入框占位。
pub const MSG_PH_BASE_URL: &str = "https://api.openai.com/v1";
/// 新建态 API Key 输入框占位。
pub const MSG_PH_API_KEY_NEW: &str = "sk-...";
/// 编辑态 API Key 输入框占位。
pub const MSG_PH_API_KEY_EDIT: &str = "留空 = 保持现有密钥；输入明文则整体替换";
/// 备注输入框占位。
pub const MSG_PH_REMARK: &str = "渠道用途说明";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED_CHANNELS: &str = "加载渠道失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST_CHANNELS: &str = "正在加载渠道…";
/// 列表空态占位。
pub const MSG_EMPTY_CHANNELS: &str = "没有匹配的渠道";
/// 分组候选为空且无分组时的提示。
pub const MSG_NO_GROUPS: &str = "暂无分组";
/// 模型候选池为空时的提示。
pub const MSG_NO_MODEL_CANDIDATES: &str = "暂无候选模型；点「拉取上游模型」获取";
/// 写操作成功提示。
pub const MSG_OP_OK: &str = "操作成功";
/// 写操作失败提示前缀(后接错误详情)。
pub const MSG_OP_FAILED: &str = "操作失败:";
/// 保存失败提示前缀(后接错误详情)。
/// 行内 Popover 提交名称为空时的提示。
pub const MSG_NAME_REQUIRED_CHANNELS: &str = "渠道名称不能为空";
/// 拉取上游模型失败提示前缀(后接错误详情)。
pub const MSG_FETCH_MODELS_FAILED: &str = "拉取上游模型失败：";
/// 拉取上游模型按钮的加载态文案。
pub const MSG_FETCHING_MODELS: &str = "拉取中…";
/// 拉取上游模型按钮的常态文案。
pub const MSG_FETCH_MODELS: &str = "拉取上游模型";
/// 拉取上游模型面板的说明。
pub const MSG_FETCH_MODELS_HINT: &str =
    "用该渠道凭据请求上游 /v1/models；勾选项随保存写入（整体替换该列，已含现有模型）";
/// 分组列表拉取失败时的只读回退前缀。
pub const MSG_GROUP_ERR_PREFIX: &str = "分组列表拉取失败（";
/// 分组列表拉取失败时的只读回退中段。
pub const MSG_GROUP_ERR_MID: &str = "）；当前绑定: ";
/// 当前密钥掩码区的标题前缀。
pub const MSG_EXISTING_KEYS_PREFIX: &str = "当前密钥（掩码，共 ";
/// 当前密钥掩码区的标题后缀。
pub const MSG_EXISTING_KEYS_SUFFIX: &str = " 条；明文不出后端）";

// ---- redemptions tab ----
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
pub const SEC_STATS_REDEMPTIONS: &str = "兑换码概览";
/// 筛选与操作区标题。
/// 卡片网格区标题。
pub const SEC_LIST_REDEMPTIONS: &str = "兑换码列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE_REDEMPTIONS: &str = "按状态分级筛选;停用后不可重新启用";

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
pub const LBL_STAT_TOTAL_REDEMPTIONS: &str = "兑换码总数";
/// 概览卡:未使用兑换码数。
pub const LBL_STAT_UNUSED: &str = "未使用";
/// 概览卡:已核销兑换码数。
pub const LBL_STAT_USED: &str = "已核销";
/// 概览卡:已停用兑换码数。
pub const LBL_STAT_DISABLED_REDEMPTIONS: &str = "已停用";
/// 概览卡:可用面额合计。
pub const LBL_STAT_AVAILABLE: &str = "可用面额";
/// 卡片状态徽标:未使用。
pub const LBL_STATUS_UNUSED: &str = "未使用";
/// 卡片状态徽标:已核销。
pub const LBL_STATUS_USED: &str = "已核销";
/// 卡片状态徽标:已停用。
pub const LBL_STATUS_DISABLED_REDEMPTIONS: &str = "已停用";
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
/// 分级胶囊:全部档位。
/// 分级胶囊:未使用档位。
pub const OPT_UNUSED: &str = "未使用";
/// 分级胶囊:已核销档位。
pub const OPT_USED: &str = "已核销";
/// 分级胶囊:已停用档位。

// ---- BTN_* : 按钮文案 ----

/// 筛选区生成兑换码按钮。
pub const BTN_GENERATE: &str = "✚ 生成兑换码";
/// 列表错误态重试按钮。
/// 卡片复制按钮的常态文案。
pub const BTN_COPY: &str = "复制预览";
/// 卡片复制按钮的复制成功态文案。
pub const BTN_COPIED: &str = "已复制";
/// 卡片停用按钮(兑换码未使用时显示)。
/// 卡片已核销状态的禁用占位按钮。
pub const BTN_REDEEMED: &str = "已核销";
/// 卡片已停用状态的禁用占位按钮。
pub const BTN_DISABLED: &str = "已停用";
/// 生成弹窗取消按钮。
pub const BTN_CANCEL_REDEMPTIONS: &str = "取消";
/// 生成弹窗提交按钮。
pub const BTN_SUBMIT_GENERATE: &str = "立即批量生成";
/// 明文码展示弹窗的关闭按钮。
pub const BTN_CLOSE_SAVED: &str = "我已保存,关闭";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER_REDEMPTIONS: &str = "搜索兑换码预览 (如 fx-086c****) ...";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED_REDEMPTIONS: &str = "加载兑换码失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST_REDEMPTIONS: &str = "正在加载兑换码…";
/// 列表空态占位。
pub const MSG_EMPTY_REDEMPTIONS: &str = "没有匹配的兑换码";
/// 生成弹窗底部的明文码一次性提示。
pub const MSG_GENERATE_HINT: &str =
    "提示: 明文卡密只在生成后显示一次,请及时复制保存;后端暂不支持活动名与有效期字段。";
/// 明文码展示弹窗中的醒目提示。
pub const MSG_CODES_WARNING: &str = "以下明文卡密关闭本窗口后无法再次查看,请立即复制保存。";

// ---- subscriptions tab ----
use dioxus::prelude::*;

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 套餐卡指标:价格。
pub const LBL_PRICE: &str = "价格";
/// 套餐卡指标:有效期。
pub const LBL_PERIOD: &str = "有效期";
/// 套餐卡指标:套餐额度。
pub const LBL_QUOTA: &str = "套餐额度";
/// 套餐卡状态徽标:启用。
pub const LBL_ENABLED: &str = "启用";
/// 套餐卡状态徽标:禁用。
pub const LBL_DISABLED: &str = "禁用";
/// 套餐卡额度不限时的替代文案。
pub const LBL_UNLIMITED: &str = "无限制";

/// 套餐卡分组徽标前缀(后接分组名)。
pub const LBL_GROUP_PREFIX: &str = "分组: ";

/// 卡片列表区标题(分页器旁的标题)。
pub const SEC_LIST_SUBSCRIPTIONS: &str = "套餐列表";
/// 卡片列表计数徽标:加载中替代文案。

// ---- BTN_* : 按钮文案 ----

/// 页面顶栏新建套餐按钮。
pub const BTN_NEW_PLAN: &str = "新建套餐";
/// 套餐卡编辑按钮。
/// 弹窗底部关闭按钮。
pub const BTN_CLOSE: &str = "关闭";
/// 弹窗底部保存按钮。
pub const BTN_SAVE: &str = "保存更改";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 顶部栏:套餐按名称去重的提示。
pub const SEC_NAME_DEDUP: &str = "套餐按名称去重：同名保存即更新现有套餐，改名会新建一行";
/// 列表为空时的空态提示。
pub const MSG_EMPTY_SUBSCRIPTIONS: &str = "还没有订阅套餐。点击右上角「新建套餐」创建第一个。";
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
pub const MSG_DELETED_SUBSCRIPTIONS: &str = "套餐已删除";
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
pub const TAB_BASIC_SUBSCRIPTIONS: &str = "基本信息";
/// 弹窗页签:规则与周期。
pub const TAB_RULES: &str = "规则与周期";

// ---- TTL_* : 弹窗 / 抽屉标题 ----

/// 弹窗标题:编辑态。
pub const TTL_EDIT_SUBSCRIPTIONS: &str = "更新套餐信息";
/// 弹窗标题:新建态。
pub const TTL_NEW_SUBSCRIPTIONS: &str = "新建订阅套餐";

// ---- FIELD_* : 表单字段标签 ----

/// 基本信息字段:套餐标题。
pub const FIELD_PLAN_TITLE: &str = "套餐标题";
/// 基本信息字段:套餐价格(币种由旁边下拉决定)。
pub const FIELD_PRICE: &str = "套餐价格";
/// 规则字段:有效期(天)。
pub const FIELD_DURATION: &str = "有效期（天）";
/// 基本信息字段:升级分组。
pub const FIELD_GROUP: &str = "升级分组";
/// 基本信息字段:限购。
pub const FIELD_LIMIT: &str = "限购";
/// 规则字段:启用状态。
pub const FIELD_ENABLED_SUBSCRIPTIONS: &str = "启用状态";

// ---- OPT_* : 下拉选项 / 分段选择器选项 ----

/// 升级分组选项:不升级。
pub const OPT_NO_UPGRADE: &str = "不升级";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 套餐标题输入框占位。
pub const MSG_PH_PLAN_TITLE: &str = "例如：开拓的封赏";
/// 套餐价格输入框旁的说明。
pub const MSG_PRICE_HINT: &str = "用户购买该套餐需支付的金额，具体币种由支付渠道决定";
/// 额度输入框旁的说明。
pub const MSG_QUOTA_HINT: &str = "套餐包含的总额度；0 表示不限量";
/// 限购输入框旁的说明。
pub const MSG_LIMIT_HINT: &str = "单个用户可购买的次数；0 表示不限";

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

// ---- system tab ----
/// 系统综合信息视图,对应后端 `admin_ops::system_info::SystemInfoView`。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfoView {
    #[serde(default)]
    pub runtime: RuntimeInfo,
    #[serde(default)]
    pub uptime: UptimeInfo,
    #[serde(default)]
    pub memory: MemoryInfo,
    #[serde(default)]
    pub cpu: CpuInfo,
    #[serde(default)]
    pub database: DatabaseInfo,
    #[serde(default)]
    pub counts: EntityCounts,
}

/// 运行时基础环境。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub os: String,
    #[serde(default)]
    pub arch: String,
    #[serde(default)]
    pub hostname: String,
}

/// 进程启动与运行时间。`started_at` 是后端 DateTime<Utc> 序列化的 RFC3339 串。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UptimeInfo {
    #[serde(default)]
    pub started_at: String,
    #[serde(default)]
    pub uptime_seconds: u64,
}

/// 内存监控数据(字节)。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryInfo {
    #[serde(default)]
    pub process_rss_bytes: u64,
    #[serde(default)]
    pub system_total_bytes: u64,
    #[serde(default)]
    pub system_used_bytes: u64,
    #[serde(default)]
    pub system_available_bytes: u64,
}

/// CPU 核心数与系统负载。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuInfo {
    #[serde(default)]
    pub num_cpus: usize,
    #[serde(default)]
    pub load_avg_1m: Option<f64>,
    #[serde(default)]
    pub load_avg_5m: Option<f64>,
    #[serde(default)]
    pub load_avg_15m: Option<f64>,
}

/// 数据库连接池诊断。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub pool_size: u32,
    #[serde(default)]
    pub idle_connections: u32,
}

/// 核心业务实体数量统计。
#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityCounts {
    #[serde(default)]
    pub users: i64,
    #[serde(default)]
    pub channels: i64,
    #[serde(default)]
    pub active_channels: i64,
    #[serde(default)]
    pub models: i64,
    #[serde(default)]
    pub tokens: i64,
}

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- SEC_* : 区段标题 / 说明条 ----

/// 系统概览区段标题。
pub const SEC_STATS_SYSTEM: &str = "系统概览";
/// 实体统计区段标题。
pub const SEC_COUNTS: &str = "实体统计";
/// 运行环境区段标题。
pub const SEC_ENV: &str = "运行环境";
/// 运行环境区段说明:数据来源。
pub const SEC_ENV_NOTE: &str = "采集自服务端进程与数据库连接池的实时诊断数据";
/// 站点选项区段说明。
pub const SEC_OPTIONS_NOTE: &str = "运行时选项 (key/value 平表),来自 /api/option 注册表与数据库值";
/// 出口代理节点导入面板的 aria 区域名。
pub const SEC_PROXY_IMPORT: &str = "出口代理节点导入";
/// 代理节点运行态面板的 aria 区域名。
pub const SEC_PROXY_RUNTIME: &str = "代理节点运行态";
/// 代理节点运行态列表的 aria 区域名。
pub const SEC_PROXY_RUNTIME_LIST: &str = "代理节点运行态列表";
/// 代理节点运行态区段说明:数据来源。
pub const SEC_PROXY_RUNTIME_NOTE: &str = "网关数据面实时采集";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:运行时长。
pub const LBL_UPTIME: &str = "运行时长";
/// 概览卡:系统内存。
pub const LBL_SYS_MEMORY: &str = "系统内存 (已用/总量)";
/// 概览卡:CPU 负载。
pub const LBL_CPU_LOAD: &str = "CPU 负载 (1m)";
/// 概览卡:数据库状态。
pub const LBL_DB_STATUS: &str = "数据库状态";
/// 概览卡:进程常驻内存。
pub const LBL_PROCESS_RSS: &str = "进程常驻内存";
/// 实体统计卡:注册用户。
pub const LBL_COUNT_USERS: &str = "注册用户";
/// 实体统计卡:渠道总数。
pub const LBL_COUNT_CHANNELS: &str = "渠道总数";
/// 实体统计卡:启用渠道。
pub const LBL_COUNT_ACTIVE_CHANNELS: &str = "启用渠道";
/// 实体统计卡:模型数量。
pub const LBL_COUNT_MODELS: &str = "模型数量";
/// 实体统计卡:Token 总数。
pub const LBL_COUNT_TOKENS: &str = "Token 总数";
/// 环境明细行:服务版本。
pub const LBL_SERVICE_VERSION: &str = "服务版本";
/// 环境明细行:操作系统。
pub const LBL_OS: &str = "操作系统";
/// 环境明细行:主机名。
pub const LBL_HOSTNAME: &str = "主机名";
/// 环境明细行:启动时间。
pub const LBL_STARTED_AT: &str = "启动时间";
/// 环境明细行:负载均值。
pub const LBL_LOAD_AVG: &str = "负载均值";
/// 环境明细行:连接池。
pub const LBL_CONN_POOL: &str = "连接池";
/// 环境明细行:内存明细。
pub const LBL_MEMORY_DETAIL: &str = "内存明细";
/// 站点选项面板标题(同时用作局部变量名源)。
pub const LBL_SITE_OPTIONS: &str = "站点选项";
/// 出口代理节点面板标题。
pub const LBL_PROXY_NODES: &str = "出口代理节点";
/// 代理节点运行态面板标题。
pub const LBL_PROXY_RUNTIME: &str = "代理节点运行态";
/// 代理节点状态徽标:已启用。
pub const LBL_NODE_ENABLED: &str = "已启用";
/// 代理节点状态徽标:已停用。
pub const LBL_NODE_DISABLED: &str = "已停用";

// ---- BTN_* : 按钮文案 ----

/// 系统概览区刷新按钮。
/// 系统概览区错误态重试按钮。
/// 出口代理节点导入按钮。
pub const BTN_IMPORT: &str = "导入";
/// 出口代理节点导入按钮的加载态文案。
pub const BTN_IMPORTING: &str = "导入中...";

// ---- FIELD_* : 表单字段标签 ----

/// 出口代理节点导入:订阅链接输入框占位。
pub const FIELD_SUB_URL_PLACEHOLDER: &str = "https://example.com/clash.yaml";
/// 出口代理节点导入:渠道列表输入框占位。
pub const FIELD_CHANNELS_PLACEHOLDER: &str = "openai,claude（逗号分隔）";
/// 出口代理节点导入:优先级输入框占位。
pub const FIELD_PRIORITY_PLACEHOLDER: &str = "优先级 (默认 10)";
/// 出口代理节点导入:订阅导入区标题。
pub const FIELD_SUB_IMPORT: &str = "订阅导入";
/// 出口代理节点导入:粘贴分享链接区标题。
pub const FIELD_SHARE_IMPORT: &str = "粘贴分享链接";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 出口代理节点导入:URL 与渠道均必填的校验提示。
pub const MSG_IMPORT_URL_REQUIRED: &str = "URL 和渠道不能为空";
/// 出口代理节点导入:分享链接与渠道均必填的校验提示。
pub const MSG_IMPORT_SHARE_REQUIRED: &str = "分享链接和渠道不能为空";
/// 出口代理节点导入:已创建计数前缀。
pub const MSG_CREATED_PREFIX: &str = "已创建: ";
/// 出口代理节点导入:已跳过计数前缀。
pub const MSG_SKIPPED_PREFIX: &str = "已跳过: ";
/// 出口代理节点导入:失败明细标题。
pub const MSG_FAILURES_TITLE: &str = "失败明细（本功能核心卖点）：";
/// 出口代理节点导入:全部成功提示。
pub const MSG_ALL_IMPORTED: &str = "所有节点导入成功";
/// 代理节点运行态:加载失败前缀(后接错误详情)。
pub const MSG_RUNTIME_LOAD_FAILED: &str = "加载代理节点运行态失败:";
/// 代理节点运行态:加载态占位。
pub const MSG_RUNTIME_LOADING: &str = "正在加载代理节点运行态…";
/// 代理节点运行态:空态标题。
pub const MSG_RUNTIME_EMPTY: &str = "无代理节点";
/// 代理节点运行态:空态说明。
pub const MSG_RUNTIME_EMPTY_HINT: &str =
    "先在上方导入出口代理节点,导入成功的节点运行态会在这里展示";
/// 代理节点运行态:该行无运行态时的说明。
pub const MSG_NO_RUNTIME: &str = "无运行态(未启用或未装配)";
/// 代理节点运行态:在途数标签前缀。
pub const MSG_INFLIGHT_PREFIX: &str = "在途 ";
/// 代理节点运行态:失败数标签前缀。
pub const MSG_FAILURE_PREFIX: &str = "失败 ";
/// 代理节点运行态:冷却剩余标签前缀。
pub const MSG_COOLDOWN_PREFIX: &str = "冷却 ";
/// 代理节点运行态:最近延迟标签前缀。
pub const MSG_DELAY_PREFIX: &str = "延迟 ";
/// 系统概览:加载失败标题。
pub const MSG_LOAD_FAILED_SYSTEM: &str = "加载系统信息失败";
/// 系统概览:加载态占位。
pub const MSG_LOADING_SYSTEM: &str = "正在加载系统信息…";
/// 系统概览:空态标题。
pub const MSG_EMPTY_SYSTEM: &str = "暂无系统信息";
/// 系统概览:空态说明。
pub const MSG_EMPTY_HINT: &str = "后端未返回采集数据 —— 服务重启产生指标后这里会展示真实系统状态";
/// 站点选项:加载态占位。
pub const MSG_OPTIONS_LOADING: &str = "正在加载站点选项…";
/// 站点选项:空态占位。
pub const MSG_OPTIONS_EMPTY: &str = "暂无站点选项";

/// 秒数 → 人类可读运行时长(天/小时/分,不足一分钟时显示秒)。
pub fn format_uptime(total_secs: u64) -> String {
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;
    if days > 0 {
        format!("{days}天 {hours}小时")
    } else if hours > 0 {
        format!("{hours}小时 {mins}分")
    } else if mins > 0 {
        format!("{mins}分 {secs}秒")
    } else {
        format!("{secs}秒")
    }
}

/// 字节数 → 人类可读容量(1 位小数,自动选 GB/MB/KB/B)。
pub fn format_bytes(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const KB: f64 = 1024.0;
    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}GB", b / GB)
    } else if b >= MB {
        format!("{:.1}MB", b / MB)
    } else if b >= KB {
        format!("{:.1}KB", b / KB)
    } else {
        format!("{bytes}B")
    }
}

/// 负载均值 → 两位小数;None(平台不提供)显示占位符,不造数据。
pub fn format_load(v: Option<f64>) -> String {
    v.map(|x| format!("{x:.2}")).unwrap_or_else(|| "—".into())
}

/// 数据库连通状态码 → 中文标签(未知值原样展示)。
pub fn format_db_status(status: &str) -> String {
    match status {
        "connected" => "已连接".to_string(),
        "degraded" => "已降级".to_string(),
        other => other.to_string(),
    }
}

/// RFC3339 启动时间 → 去掉小数秒的可读时间串。
pub fn format_started_at(rfc3339: &str) -> String {
    rfc3339.split('.').next().unwrap_or(rfc3339).to_string()
}
