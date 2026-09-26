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

// ===========================================================================
// 语义设计 token —— 整仓样式的单一来源（页面组件只引用这些，不写裸 Tailwind）。
//
// 三层，全部在本文件；改一处 = 全站生效：
//   ① 文本角色  TYPE_*   —— 标题 / 卡牌标题 / 正文 / 描述 / 微标签 / 数值 / 按钮。
//                            按"这是什么文字"放对应角色，不逐字拆字号颜色。
//   ② 语义色    C_*      —— 主/次/状态文字色（按语义，不按色号）。
//   ③ 表面      S_* / B_* —— 背景（面板/凸起/浅底）与描边。
//
// 主题化：换肤时只改这三个小节里各 token 的右值（都在 styles.rs），全站换。
// 反例（别做）：把 `text-zinc-400` 这种原子拆成常量——那是把颜色写死，没法换主题，
//   且 token 数量爆炸。语义层已够，原子留 Tailwind。
// ===========================================================================

// ① 文本角色（size + weight + color 组合；颜色值跟 ② 一致，换肤时同步改）

/// 区段 / 页面标题：大、亮、粗。
pub const TYPE_TITLE: &str = "text-lg font-bold text-zinc-100";

/// 卡牌 / 列表项标题：中、亮。
pub const TYPE_CARD_TITLE: &str = "text-sm font-medium text-zinc-100";

/// 正文 / 字段说明正文。
pub const TYPE_BODY: &str = "text-sm text-zinc-300";

/// 描述 / 次要说明（全仓最大头的 `text-xs` 说明文字）。
pub const TYPE_DESC: &str = "text-xs text-zinc-400";

/// 表单微标签（NAME / EMAIL / 字段小标签）。
pub const TYPE_LABEL: &str = "text-[11px] font-medium text-zinc-400";

/// 大数值 / 统计主数字。
pub const TYPE_VALUE: &str = "text-2xl font-semibold text-zinc-50";

/// 按钮文字：只定字号字重，颜色随所在底色（浅底用 `C_ON`、深底用 `TYPE_CARD_TITLE` 同款）。
pub const TYPE_BUTTON: &str = "text-sm font-medium";

// ② 语义色（文字）——按语义引用，不按色号

/// 主文字（面板内默认正文色）。
pub const C_TEXT: &str = "text-zinc-100";

/// 次要 / 弱化文字。
pub const C_MUTED: &str = "text-zinc-400";

/// 浅底（白底按钮等）上的文字。
pub const C_ON: &str = "text-zinc-900";

/// 成功。
pub const C_SUCCESS: &str = "text-emerald-400";

/// 危险 / 错误。
pub const C_DANGER: &str = "text-red-400";

/// 警告 / 进行中。
pub const C_WARNING: &str = "text-amber-400";

/// 信息 / 提示。
pub const C_INFO: &str = "text-sky-300";

// ③ 表面（背景）与描边

/// 面板 / 卡片背景。
pub const S_PANEL: &str = "bg-zinc-900";

/// 凸起表面（hover / 选中 / 次级块）。
pub const S_RAISED: &str = "bg-zinc-800";

/// 浅底表面（主按钮白底）。
pub const S_ON: &str = "bg-white";

/// 成功 / 危险 / 警告 / 信息 的语义表面（浅色底）。
pub const S_SUCCESS: &str = "bg-emerald-950";
pub const S_DANGER: &str = "bg-red-950";
pub const S_WARNING: &str = "bg-amber-500";

/// 面板描边。
pub const B_PANEL: &str = "border-zinc-800";
/// 成功 / 危险 / 信息 的语义描边。
pub const B_DANGER: &str = "border-red-500";
pub const B_SUCCESS: &str = "border-emerald-500";
pub const B_INFO: &str = "border-sky-500";
