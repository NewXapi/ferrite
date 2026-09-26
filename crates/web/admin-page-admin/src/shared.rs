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
