//! 渠道管理共享类型与纯函数:弹窗状态、写操作种类、筛选与输入解析。
//! page / modal 复用;筛选与两个解析函数保持 `pub`,供 `tests/` 无 runtime 单测。
//!
//! 本文件同时承载该 tab 的全部用户可见文案常量（i18n 第 1 层）：
//! 常量值即原字面量，逐字符保持一致以保证零渲染变化。
//!
//! 边界:不放组件(`#[component]` 在 stats / toolbar / list / card / modal 里),
//! 不放网络调用(在 `page.rs` 与 `modal.rs`);这里只有类型定义、纯函数与文案常量,
//! 保证可被同层 `tests/` 以无 runtime 方式单测。

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
pub const SEC_STATS: &str = "渠道概览";
/// 筛选与操作区标题。
pub const SEC_FILTER: &str = "筛选与操作";
/// 卡片网格区标题。
pub const SEC_LIST: &str = "渠道列表";
/// 筛选与操作区标题旁的说明。
pub const SEC_FILTER_NOTE: &str = "按状态或关键词筛选";

// ---- TTL_* : 弹窗标题 ----

/// 编辑渠道时的弹窗标题。
pub const TTL_EDIT: &str = "编辑渠道";
/// 新建渠道时的弹窗标题。
pub const TTL_NEW: &str = "新建渠道";

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
pub const LBL_STAT_TOTAL: &str = "总渠道数";
/// 概览卡:正常启用数。
pub const LBL_STAT_ENABLED: &str = "正常启用";
/// 概览卡:停用/异常数。
pub const LBL_STAT_DISABLED: &str = "停用/异常";
/// 概览卡:密钥总数。
pub const LBL_STAT_KEYS: &str = "密钥总数";
/// 概览卡:绑定分组数。
pub const LBL_STAT_GROUPS: &str = "绑定分组数";
/// 卡片状态徽标:启用中。
pub const LBL_STATUS_ENABLED: &str = "启用中";
/// 卡片状态徽标:已停用。
pub const LBL_STATUS_DISABLED: &str = "已停用";
/// 卡片指标行:接口地址。
pub const LBL_BASE_URL: &str = "接口地址";
/// 卡片指标行:权重。
pub const LBL_WEIGHT: &str = "权重";
/// 卡片指标行:备注。
pub const LBL_REMARK: &str = "备注";
// ---- BTN_* : 按钮文案 ----

/// 筛选区刷新按钮。
pub const BTN_REFRESH: &str = "刷新";
/// 筛选区新建渠道按钮。
pub const BTN_NEW_CHANNEL: &str = "✚ 新建渠道";
/// 列表错误态重试按钮。
pub const BTN_RETRY: &str = "重试";
/// 卡片编辑按钮。
pub const BTN_EDIT: &str = "编辑";
/// 卡片删除按钮的悬停提示。
pub const BTN_DELETE_TITLE: &str = "删除渠道";
/// 卡片停用按钮(渠道启用中时显示)。
pub const BTN_DISABLE: &str = "停用";
/// 卡片启用按钮(渠道已停用时显示)。
pub const BTN_ENABLE: &str = "启用";
/// 弹窗取消按钮。
pub const BTN_CANCEL: &str = "取消";
/// 弹窗编辑态提交按钮。
pub const BTN_SAVE_CHANGES: &str = "保存修改";
/// 弹窗新建态提交按钮。
pub const BTN_CREATE_CHANNEL: &str = "创建渠道";

// ---- OPT_* : 下拉选项 / 分段选择器选项文案 ----

/// 计数徽标:加载中替代文案。
pub const OPT_BADGE_LOADING: &str = "加载中…";
/// 分级胶囊:全部档位。
pub const OPT_ALL: &str = "全部";
/// 分级胶囊:启用中档位。
pub const OPT_ENABLED: &str = "启用中";
/// 分级胶囊:已停用档位。
pub const OPT_DISABLED: &str = "已停用";

// ---- MSG_* : 提示 / 错误 / 空态文案 ----

/// 搜索框占位。
pub const MSG_SEARCH_PLACEHOLDER: &str = "搜索渠道名称、类型、分组或 API 目标地址...";
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
pub const MSG_LOAD_FAILED: &str = "加载渠道失败";
/// 列表加载态占位。
pub const MSG_LOADING_LIST: &str = "正在加载渠道…";
/// 列表空态占位。
pub const MSG_EMPTY: &str = "没有匹配的渠道";
/// 分组候选为空且无分组时的提示。
pub const MSG_NO_GROUPS: &str = "暂无分组";
/// 模型候选池为空时的提示。
pub const MSG_NO_MODEL_CANDIDATES: &str = "暂无候选模型；点「拉取上游模型」获取";
/// 写操作成功提示。
pub const MSG_OP_OK: &str = "操作成功";
/// 写操作失败提示前缀(后接错误详情)。
pub const MSG_OP_FAILED: &str = "操作失败:";
/// 保存失败提示前缀(后接错误详情)。
pub const MSG_SAVE_FAILED: &str = "保存失败:";
/// 行内 Popover 提交名称为空时的提示。
pub const MSG_NAME_REQUIRED: &str = "渠道名称不能为空";
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
