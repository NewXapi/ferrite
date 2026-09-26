//! 全仓统一样式常量：把页面里反复手抄的 Tailwind class 串收敛到单一来源。
//!
//! 这些常量是视觉 token，不是逻辑。页面组件应通过 `ui::styles` 引用，
//! 而不是在 `rsx!` 里写长字面量；改一处即可全仓生效。

/// 页面背景：深色全屏 + 防横向滚动。
pub const PAGE_BG: &str = "relative min-h-screen overflow-x-hidden bg-zinc-950 text-zinc-100";

/// 认证页卡片：窄卡 + 毛玻璃 + 强阴影。
pub const AUTH_CARD: &str = "w-full max-w-md rounded-2xl border border-zinc-800 bg-zinc-900/70 p-8 shadow-2xl shadow-black/40 backdrop-blur";

/// 认证页 tab 切换器外壳：胶囊 + 内部滑块。
pub const TAB_SWITCHER: &str =
    "relative mb-6 flex rounded-full border border-zinc-800 bg-zinc-900 p-1";

/// tab 滑块（绝对定位，`translate-x-0` / `translate-x-full` 由调用方拼接）。
pub const TAB_INDICATOR: &str =
    "absolute inset-y-0 w-1/2 rounded-full bg-zinc-100 transition-transform duration-200";

/// 激活 tab 文案。
pub const TAB_ACTIVE: &str = "relative z-10 flex-1 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 text-zinc-900";

/// 未激活 tab 文案。
pub const TAB_INACTIVE: &str = "relative z-10 flex-1 rounded-full px-3.5 py-1.5 text-xs font-medium transition-colors duration-200 text-zinc-400 hover:text-zinc-200";

/// 通用区段外壳：圆角描边面板 + 内边距。
pub const SECTION: &str = "rounded-xl border border-zinc-800 bg-zinc-900 p-6";

/// 带 ScrollSpy 锚点的区段外壳。
pub const SECTION_SCROLL: &str = "scroll-mt-8 rounded-xl border border-zinc-800 bg-zinc-900 p-6";

/// 区段标题行：左标题 + 右侧徽章 / 计数。
pub const SECTION_HEADER: &str = "mb-4 flex items-center justify-between";

/// 区段大标题（`text-lg`）。
pub const SECTION_TITLE: &str = "text-lg font-medium text-zinc-100";

/// 状态 / 计数胶囊。
pub const STATUS_PILL: &str = "rounded-full bg-zinc-800 px-3 py-1 text-xs text-zinc-400";

/// 输入框 / 选择框统一 class。
pub const INPUT: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 text-sm focus:border-zinc-500 focus:outline-none";

/// 等宽字体输入框。
pub const INPUT_MONO: &str = "w-full rounded-xl border border-zinc-700 bg-zinc-950 px-4 py-2.5 font-mono text-sm focus:border-zinc-500 focus:outline-none";

/// 幽灵按钮（次级操作，描边 + 悬停提亮）。
pub const GHOST_BTN: &str = "flex-1 rounded-xl border border-zinc-700 py-2.5 text-sm text-zinc-400 transition-colors hover:bg-zinc-800";

/// 主操作按钮（白底高对比）。
pub const PRIMARY_BTN: &str = "flex-1 rounded-xl bg-white px-4 py-2.5 text-sm font-medium text-zinc-900 transition-colors hover:bg-zinc-200 disabled:cursor-not-allowed disabled:opacity-50";

/// 刷新 / 次要小按钮。
pub const REFRESH_BTN: &str = "rounded-xl border border-zinc-700 bg-zinc-950 px-3.5 py-2 text-xs font-medium text-zinc-300 transition-colors hover:border-zinc-500 hover:text-white";

/// 信息条（只读说明 / 状态提示）。
pub const INFO_BAR: &str =
    "rounded-xl border border-zinc-700 bg-zinc-900 px-4 py-2 text-xs text-zinc-300";

/// 危险条（错误 / 破坏性提示）。
pub const DANGER_BAR: &str =
    "rounded-xl border border-red-900/60 bg-red-950/30 px-4 py-2 text-xs text-red-300";

/// 内容卡外壳（区段内的单卡，内边距 p-6）。
pub const CARD_CONTENT: &str = "rounded-xl border border-zinc-800 bg-zinc-900/60 p-6";

/// 实体卡网格（手机 1 / 中屏 3 / 大屏 5 栏）。
pub const CARD_GRID: &str = "grid grid-cols-1 gap-3 md:grid-cols-3 lg:grid-cols-5";

// ---------------------------------------------------------------------------
// Typography roles — 复用文本样式（size + weight + color 固定组合，整仓统一）。
// 颜色先用裸 zinc（视觉定稿前的过渡，非主题化；换主题另议）。
// 组件在 rsx! 里引用 `ui::TYPE_*`，取代散落的 `text-*`/`font-*` 字面量。
// ---------------------------------------------------------------------------

/// 大字 / hero / 最大数值：很大 + 粗 + 最亮。
pub const TYPE_DISPLAY: &str = "text-3xl font-bold text-zinc-50";

/// 区段标题（角色①）：大 / 亮 / 粗。取代反复手抄的 `text-lg font-medium text-zinc-100`。
pub const TYPE_TITLE: &str = "text-lg font-bold text-zinc-50";

/// tab 文字（角色②）：中等 / 亮 / 不粗。用于 tab 切换器的文案部分。
pub const TYPE_TAB: &str = "text-sm text-zinc-200";

/// 卡牌标题（角色④）：正常 / 亮。卡片名 / 列表项标题。
pub const TYPE_CARD_TITLE: &str = "text-sm font-medium text-zinc-100";

/// 数值 / 字段值：亮 + 粗。统计大数字、Profile 的 NAME/EMAIL 值。
pub const TYPE_VALUE: &str = "text-xl font-semibold text-zinc-50";

/// 正文：正常 / 中亮。取代散落的 `text-sm text-zinc-300`。
pub const TYPE_BODY: &str = "text-sm text-zinc-300";

/// 描述 / 次要说明（角色③）：暗 / 正常。取代裸 `text-xs`（全仓最大头）。
pub const TYPE_DESC: &str = "text-xs text-zinc-500";

/// 微标签：暗 / 小 / 粗 + 字距。字段小标签（NAME/EMAIL/SIGN-IN 那种）。
pub const TYPE_LABEL: &str = "text-[10px] font-medium tracking-wider text-zinc-500";
