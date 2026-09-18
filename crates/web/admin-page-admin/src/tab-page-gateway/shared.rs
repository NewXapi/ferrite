//! 网关渠道健康共享常量:三态 Badge 语义色。
//!
//! 面板表头的冷却 / 慢启动 / 正常计数徽标与行内三态徽标共用同一组常量,
//! 保证表头计数与行徽标的颜色语义始终一致 (shadcn dashboard 既有 token
//! 风格,禁 emoji):cooling=红系 / slow_start=黄系 / ok=绿系。

/// cooling=红系。
pub const TONE_COOLING: &str = "border-red-500/30 bg-red-500/15 text-red-300";
/// slow_start=黄系。
pub const TONE_SLOW_START: &str = "border-amber-500/30 bg-amber-500/15 text-amber-300";
/// ok=绿系。
pub const TONE_OK: &str = "border-emerald-500/30 bg-emerald-500/15 text-emerald-400";
