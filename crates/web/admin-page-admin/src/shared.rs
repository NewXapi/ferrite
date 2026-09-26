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

// ---- entities tab ----
use dioxus::prelude::*;

// ============ 用户可见文案常量（i18n 第 1 层） ============

// ---- FIELD_* : 表单字段标签 ----

/// 跨卡片共享:展示名录入框标签。
pub const FIELD_DISPLAY_ENTITIES: &str = "展示名";
/// 分组卡:分组名录入框标签(编辑态)。
pub const FIELD_GROUP_NAME_LOCKED: &str = "分组名（锁读，后端无改名路径）";
/// 分组卡:分组名录入框标签(新增态)。
pub const FIELD_GROUP_NAME: &str = "分组名";
/// 别名卡:别名录入框标签。
pub const FIELD_ALIAS: &str = "别名";
/// 别名卡:输入价录入框标签。
pub const FIELD_INPUT_PRICE_ENTITIES: &str = "输入价 ¥/1k";
/// 别名卡:输出价录入框标签。
pub const FIELD_OUTPUT_PRICE: &str = "输出价 ¥/1k";
/// 渠道卡:渠道名称录入框标签。
/// 渠道卡:渠道类型下拉标签。
pub const FIELD_CHANNEL_TYPE_ENTITIES: &str = "类型";
/// 渠道卡:Base URL 录入框标签。
pub const FIELD_BASE_URL_ENTITIES: &str = "Base URL";
/// 渠道卡:API Key 录入框标签。
pub const FIELD_API_KEY_ENTITIES: &str = "API Key（留空 = 不改动现有密钥）";

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
pub const BTN_CANCEL_ENTITIES: &str = "取消";
/// 渠道卡:新建渠道按钮。
pub const BTN_NEW_CHANNEL_ENTITIES: &str = "＋ 新建渠道";
/// 渠道卡:保存按钮。
pub const BTN_SAVE_ENTITIES: &str = "保存";
/// 渠道卡:停用按钮(渠道启用中时显示)。
/// 渠道卡:启用按钮(渠道停用时显示)。
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
pub const MSG_PH_CHANNEL_NAME_ENTITIES: &str = "OpenAI 官方";
/// 渠道 Base URL 录入框占位。
pub const MSG_PH_BASE_URL_ENTITIES: &str = "https://…";
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
                span { class: "text-sm font-medium text-zinc-100", "{title}" }
                span { class: "rounded-full border border-zinc-700 px-1.5 text-[11px] text-zinc-400", "{count}" }
                span { class: "truncate text-[11px] text-zinc-600", "{hint}" }
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
            span { class: "text-[11px] text-zinc-600", "{text}" }
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
                span { class: "text-xs font-medium", "{label}" }
                if !sub.is_empty() {
                    span { class: "text-[11px] {sub_tone}", "{sub}" }
                }
            }
            button {
                class: "px-1 text-[11px] opacity-50 hover:text-red-400 hover:opacity-100",
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
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
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
            span { class: "text-[11px] text-zinc-500", "{label}" }
            select {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors focus:border-zinc-500",
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
            span { class: "text-[11px] text-zinc-500", "{label}" }
            input {
                class: "w-full rounded-md border border-zinc-800 bg-zinc-950 px-3 py-1.5 text-sm text-zinc-200 outline-none transition-colors placeholder:text-zinc-600 focus:border-zinc-500",
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

// ---- groups tab ----
/// 弹窗状态
#[derive(Clone, PartialEq)]
pub enum ModalState {
    Closed,
    New,
    Edit(String),
}

/// 写操作种类(目前仅删除;工厂保留扩展位)
#[derive(Clone, Copy)]
pub enum WriteOpGroups {
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
pub const SEC_STATS_GROUPS: &str = "分组概览";
/// 筛选与操作区标题。
/// 卡片网格区标题。
pub const SEC_LIST_GROUPS: &str = "分组列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE_GROUPS: &str = "按倍率分级或关键词筛选";
/// 页面根节点的 aria-label。
pub const SEC_PAGE_ARIA: &str = "分组管理";

// ---- TTL_* : 弹窗标题 ----

/// 编辑分组时的弹窗标题。
pub const TTL_EDIT_GROUPS: &str = "编辑分组";
/// 新建分组时的弹窗标题。
pub const TTL_NEW_GROUPS: &str = "新建分组";

// ---- TAB_* : 弹窗内页签标题 ----

/// 弹窗「分组信息」页签。
pub const TAB_BASIC_GROUPS: &str = "分组信息";
/// 弹窗「映射别名」页签。
pub const TAB_ALIAS: &str = "映射别名";

// ---- FIELD_* : 表单字段标签 ----

/// 分组标识输入框标签。
pub const FIELD_GROUP_NAME_GROUPS: &str = "分组标识 (英文唯一标识)";
/// 展示备注输入框标签。
pub const FIELD_REMARK_GROUPS: &str = "展示备注 (可选)";
/// 模型白名单输入框标签。
pub const FIELD_WHITELIST: &str = "模型白名单 (逗号分隔,可选)";
/// 计费倍率输入框标签。
pub const FIELD_RATIO: &str = "计费倍率 (ratio ≥ 0)";
/// 映射别名输入框标签。
pub const FIELD_ALIAS_GROUPS: &str = "映射别名 (多选,逗号分隔)";

// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 概览卡:总分组数。
pub const LBL_STAT_TOTAL_GROUPS: &str = "总分组数";
/// 概览卡:启用中分组数。
pub const LBL_STAT_ENABLED_GROUPS: &str = "启用中";
/// 概览卡:已停用分组数。
pub const LBL_STAT_DISABLED_GROUPS: &str = "已停用";
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
/// 分级胶囊:全部档位。
/// 分级胶囊:启用中档位。
/// 分级胶囊:已停用档位。
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
/// 筛选区新建分组按钮。
pub const BTN_NEW_GROUP: &str = "✚ 新建分组";
/// 列表错误态重试按钮。
/// 卡片编辑按钮。
/// 卡片停用按钮(分组启用中时显示)。
/// 卡片启用按钮(分组已停用时显示)。
/// 卡片删除按钮。
pub const BTN_DELETE_GROUPS: &str = "删除";
/// 默认分组不可删时的占位按钮。
pub const BTN_BUILTIN: &str = "内置";
/// 弹窗取消按钮。
pub const BTN_CANCEL_GROUPS: &str = "取消";
/// 弹窗编辑态提交按钮。
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
pub const MSG_SEARCH_PLACEHOLDER_GROUPS: &str = "搜索分组标识或备注...";
/// 列表错误态标题。
pub const MSG_LOAD_FAILED_GROUPS: &str = "加载分组失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST_GROUPS: &str = "正在加载分组…";
/// 列表空态占位。
pub const MSG_EMPTY_GROUPS_FILTER: &str = "没有匹配的分组";
/// 写操作成功提示。
/// 写操作失败提示前缀(后接错误详情)。
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
pub const MSG_PH_GROUP_NAME_GROUPS: &str = "例如: vip, claude, fast";
/// 展示备注输入框占位。
pub const MSG_PH_REMARK_GROUPS: &str = "例如: VIP会员专线、高峰备用组";
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

// ---- network tab ----
// ---- LBL_* : 标签 / 表头 / 字段旁的说明 ----

/// 节点层名:分组。
pub const LBL_GROUP: &str = "分组";
/// 节点层名:模型别名。
pub const LBL_ALIAS: &str = "模型别名";
/// 节点层名:调度模型。
pub const LBL_DISPATCH: &str = "调度模型";
/// 抽屉页签:节点。
pub const LBL_NODES: &str = "节点";
/// 设置抽屉滚动导航里的实体卡名:渠道。
pub const LBL_CHANNELS: &str = "渠道";
/// 导入抽屉头副标题。
pub const LBL_IMPORT_SUBTITLE: &str = "把 JSON 包进来，一个渠道一个";
/// 导入表单:渠道名输入框标签。
pub const LBL_CHANNEL_NAME_OPT: &str = "渠道名（可选）";
/// 导入表单:Base URL 输入框标签。
pub const LBL_BASE_URL_NETWORK: &str = "Base URL";
/// 导入表单:API Key 输入框标签。
pub const LBL_API_KEY_MULTI: &str = "API Key（多 key 换行）";

// ---- BTN_* : 按钮文案 ----

/// HUD / 抽屉页签:导入。
/// HUD / 抽屉页签:设置。
pub const BTN_SETTINGS: &str = "设置";
/// HUD:适配(缩放平移以框住全部可见节点)。
pub const BTN_FIT: &str = "适配";
/// 抽屉头关闭按钮的悬停提示。
pub const BTN_CLOSE_TITLE: &str = "关闭";
/// 检视器底部删除按钮。
pub const BTN_DELETE_NETWORK: &str = "删除";
/// 检视器底部保存按钮。
pub const BTN_SAVE_NETWORK: &str = "保存";
/// 分组检视:保存展示名按钮。
pub const BTN_SAVE_DISPLAY: &str = "保存展示名";
/// 渠道检视:保存渠道按钮。
pub const BTN_SAVE_CHANNEL: &str = "保存渠道";
/// 导入表单:提交按钮。
pub const BTN_IMPORT_CHANNEL: &str = "导入渠道";
/// 检视器删除按钮的悬停提示:分组。
pub const BTN_DELETE_GROUP_TITLE: &str = "删除分组";
/// 检视器删除按钮的悬停提示:渠道。
pub const BTN_DELETE_CHANNEL_TITLE: &str = "删除渠道";

// ---- SEC_* : 区段标题 / 说明条 ----

/// 设置抽屉:调度数据拉取失败时的顶部提示。
pub const SEC_SETTINGS_STALE: &str = "调度数据拉取失败,设置页显示的是本地缓存";
/// 焦点空间模式下的操作提示。
pub const SEC_HINT_FOCUS: &str = "焦点空间 · 左键节点切换 · 右键空白或再点同节点返回";
/// 连线拖拽中的操作提示。
pub const SEC_HINT_WIRING: &str = "拖到相邻层节点松开连线";
/// 节点拖拽中的操作提示。
pub const SEC_HINT_MOVING: &str = "松开落位";
/// 常态操作提示。
pub const SEC_HINT_IDLE: &str =
    "滚轮缩放 · 拖空白平移 · Shift拖空白框选 · Ctrl点选多个 · 拖节点摆位 · 拖圆点连线";

// ---- FIELD_* : 表单字段标签 ----

/// 分组检视:展示名输入框标签。
pub const FIELD_DISPLAY_NETWORK: &str = "展示名";
/// 导入表单/渠道检视:渠道名输入框占位示例。
pub const EXAMPLE_CHANNEL: &str = "OpenAI 官方";

// ---- MSG_* : 提示 / 错误 / 空态 / 占位 ----

/// 拓扑数据拉取:分组端点失败前缀(后接错误详情)。
pub const MSG_LOAD_GROUPS_FAILED: &str = "拉取分组失败: ";
/// 拓扑数据拉取:渠道端点失败前缀(后接错误详情)。
pub const MSG_LOAD_CHANNELS_FAILED: &str = "拉取渠道失败: ";
/// 拓扑数据拉取:模型端点失败前缀(后接错误详情)。
pub const MSG_LOAD_MODELS_FAILED: &str = "拉取模型失败: ";
/// 画布加载态 aria 标签。
pub const MSG_LOADING_ARIA: &str = "正在加载调度数据";
/// 画布加载态占位。
pub const MSG_LOADING_NETWORK: &str = "正在加载调度数据…";
/// 画布空态 aria 标签 / 占位。
pub const MSG_EMPTY_NETWORK: &str = "暂无调度数据";
/// 已提交连线的右键菜单项:删除连线。
pub const MSG_WIRE_DELETE_HINT: &str = "右键删除连线";
/// 导入表单:API Key 为空时的提示。
pub const MSG_IMPORT_KEY_REQUIRED: &str = "API Key 不能为空";
/// 导入表单:渠道名留空时的默认名。
pub const MSG_DEFAULT_CHANNEL_NAME: &str = "新渠道";
/// 导入表单:导入失败前缀(后接错误详情)。
pub const MSG_IMPORT_FAILED_PREFIX: &str = "导入失败：";
/// 写操作失败前缀(后接错误详情)。共享于删除/保存失败三类消息。
pub const MSG_WRITE_FAILED_PREFIX: &str = "删除失败：";
/// 保存操作失败前缀(后接错误详情)。
/// 分组定位失败提示前缀(后接分组名)。
/// 渠道定位失败提示前缀(后接渠道名)。
/// 定位失败提示后缀(后接关闭引号前的说明)。
pub const MSG_MISSING_SUFFIX: &str = "」不存在于后端（可能已被删除）";
/// 分组检视:该分组不存在。
pub const MSG_GROUP_ABSENT: &str = "该分组不存在";
/// 别名检视:该别名不存在。
pub const MSG_ALIAS_ABSENT: &str = "该别名不存在";
/// 检视器删除按钮在不可删时的悬停提示。
pub const MSG_DELETE_DISABLED_TITLE: &str = "别名无删除端点（锁读）";
/// 检视器保存按钮在有权限时的悬停提示。
pub const MSG_SAVE_TITLE: &str = "保存按钮在各编辑卡片内（展示名/渠道）";
/// 检视器保存按钮在无权限时的悬停提示。
pub const MSG_SAVE_DISABLED_TITLE: &str = "别名锁读，无写路径";
/// 删除确认弹窗正文。
pub const MSG_DELETE_CONFIRM: &str = "确认删除？该操作直接生效于后端，不可撤销。";
/// 分组检视:分组名锁读标签。
pub const MSG_GROUP_NAME_LOCKED: &str = "分组名（锁读：后端无改名路径）";
/// 分组检视:展示名输入框占位。
pub const MSG_PH_DEFAULT_GROUP: &str = "默认分组";
/// 分组检视:包含的模型别名列表标题。
pub const MSG_INSPECT_ALIASES_TITLE: &str = "包含的模型别名";
/// 分组检视:别名列表空态。
pub const MSG_INSPECT_ALIASES_EMPTY: &str = "拖端口连线以加入别名";
/// 别名检视:别名锁读标签。
pub const MSG_ALIAS_LOCKED: &str = "别名（锁读：写路径按 UUID key，此处定位不到）";
/// 别名检视:展示名锁读标签。
pub const MSG_ALIAS_DISPLAY_LOCKED: &str = "展示名（锁读：后端 models 域无对应列）";
/// 别名检视:所属分组列表标题。
pub const MSG_INSPECT_GROUPS_TITLE: &str = "所属分组";
/// 别名检视:所属分组列表空态。
pub const MSG_INSPECT_GROUPS_EMPTY: &str = "未加入任何分组";
/// 别名检视:路由到的调度模型列表标题。
pub const MSG_INSPECT_DISPATCH_TITLE: &str = "路由到的调度模型";
/// 别名检视:调度模型列表空态。
pub const MSG_INSPECT_DISPATCH_EMPTY: &str = "未连接调度模型";
/// 调度检视:模型名只读标签。
pub const MSG_MODEL_NAME_READONLY: &str = "模型名（只读，来自上游）";
/// 调度检视:所属渠道区标题。
pub const MSG_OWNER_CHANNEL: &str = "所属渠道";
/// 调度检视:渠道名称输入框标签。
pub const MSG_CHANNEL_NAME: &str = "渠道名称";
/// 调度检视:API Key 输入框标签。
pub const MSG_API_KEY_EDIT: &str = "API Key（多 key 一行一个；留空 = 保留现有密钥）";
/// 调度检视:被哪些别名路由列表标题。
pub const MSG_INSPECT_ROUTED_TITLE: &str = "被哪些别名路由";
/// 调度检视:被哪些别名路由列表空态。
pub const MSG_INSPECT_ROUTED_EMPTY: &str = "未被任何别名引用";
