//! 模型 tab 共享层:文案常量(i18n)。
//!
//! 本文件零逻辑、零状态;卡面映射与取数留在 `page` / `card`。

/// 模型 tab 标题与 `aria-label`。
pub const MODELS_TITLE: &str = "模型";
/// 模型总数小字前缀(后接模型数)。
pub const MODELS_COUNT_HEAD: &str = "共 ";
/// 模型总数小字后缀。
pub const MODELS_COUNT_TAIL: &str = " 个";
/// 列表拉取失败文案。
pub const MODELS_ERR: &str = "加载模型列表失败";
/// 失败卡的重试按钮文案。
pub const BTN_RETRY: &str = "重试";
/// 列表加载中文案。
pub const MODELS_LOADING: &str = "正在加载模型列表…";
/// 列表空态主文案。
pub const MODELS_EMPTY: &str = "暂无模型";
/// 列表空态副文案。
pub const MODELS_EMPTY_HINT: &str = "/api/models 返回空列表 —— 配置模型后这里会展示真实卡片";

// ---- 模型卡面 ----

/// 无数据占位符(后端缺字段时的诚实降级:不编造值)。
pub const DASH: &str = "—";
/// 卡面头条标签:累计调用。
pub const CARD_HEADLINE: &str = "累计调用";
/// 迷你统计:类型。
pub const CARD_TYPE: &str = "类型";
/// 迷你统计:最大上下文。
pub const CARD_CONTEXT: &str = "最大上下文";
/// 迷你统计:状态。
pub const CARD_STATUS: &str = "状态";
/// 状态值:启用(status == 1)。
pub const CARD_ENABLED: &str = "启用";
/// 状态值:停用(status != 1)。
pub const CARD_DISABLED: &str = "停用";
