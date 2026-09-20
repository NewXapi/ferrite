//! 卡片网格分页器：条目数超过固定页大小时，在区段标题旁渲染 tab 式页码条。
//!
//! 维护者需求（2026-09-20）：管理页各卡片列表「固定多少个卡牌就需要出现分页
//! 浏览，有点类似 tab，位置放到那个标题旁边」。形态参照 `DotTabBar` 的圆点
//! 思路改为数字页码：当前页白底胶囊、其余描边幽灵胶囊；条目不超一页时整条
//! 不渲染（`Pager` 内部判断，调用方无需自己分支）。

use dioxus::prelude::*;

/// 卡片网格默认页大小：5 列网格 × 3 行。
pub const CARD_PAGE_SIZE: usize = 15;

/// 取某一页的切片；`page` 越界时收敛到最后一页（筛选后条数变小的场景）。
pub fn page_slice<T>(items: &[T], page: usize, size: usize) -> &[T] {
    if size == 0 || items.is_empty() {
        return &[];
    }
    let pages = items.len().div_ceil(size);
    let clamped = page.min(pages - 1);
    let start = clamped * size;
    &items[start..(start + size).min(items.len())]
}

/// 页数（向上取整）；`size` 为 0 时按 1 页算，避免除零。
pub fn page_count(total: usize, size: usize) -> usize {
    if size == 0 {
        return 1;
    }
    total.div_ceil(size)
}

///  tab 式页码条：放在区段标题行右侧（`SectionHeader` 的 `trailing` 插槽）。
///
/// 【是什么】卡片网格的分页控件：`1 2 3 …` 数字胶囊 + 上一页/下一页箭头。
///
/// 【做什么】条目总数不超过一页时**不渲染任何东西**；超过时渲染页码条，
/// 当前页白底高亮，点击经 `on_change` 抛回调用方改页码。不做数据切片
/// （切片用同模块纯函数 `page_slice`，调用方自行施加）。
///
/// 【交互逻辑】点页码 → `on_change.call(idx)`；点上/下一页 → 同样抛索引，
/// 已在边界内（首页点上一页、末页点下一页）时不抛。键盘可达：每个页码都是
/// 真实 `button`，Tab 聚焦、Enter/Space 触发。
///
/// 【数据流】对内(入)：`total`（条目总数）/ `page_size` / `page`（当前页，
/// 受控：调用方持有）/ `testid`。对外(出)：`on_change(usize)` 新页码。
#[component]
pub fn Pager(
    /// 条目总数
    total: usize,
    /// 每页条数
    #[props(default = CARD_PAGE_SIZE)]
    page_size: usize,
    /// 当前页（0 基，调用方持有）
    page: Signal<usize>,
    /// 页码变更回调
    on_change: EventHandler<usize>,
    /// 整条分页器的测试标识
    #[props(default)]
    testid: String,
) -> Element {
    let pages = page_count(total, page_size);
    if pages <= 1 {
        return rsx! {};
    }
    let current = page().min(pages - 1);
    rsx! {
        nav {
            class: "flex items-center gap-1",
            role: "navigation",
            "aria-label": "分页",
            "data-testid": "{testid}",
            button {
                class: "rounded-lg border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800 disabled:opacity-40",
                "data-testid": "{testid}-prev",
                disabled: current == 0,
                onclick: move |_| on_change.call(current - 1),
                "‹"
            }
            for i in 0..pages {
                {
                    let is_active = i == current;
                    rsx! {
                        button {
                            key: "pager-page-{i}",
                            class: if is_active {
                                "rounded-lg bg-white px-2.5 py-1 text-[11px] font-medium text-zinc-900 transition-colors"
                            } else {
                                "rounded-lg border border-zinc-700 px-2.5 py-1 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800"
                            },
                            "data-testid": "{testid}-page-{i}",
                            "aria-current": if is_active { "page" } else { "" },
                            onclick: move |_| on_change.call(i),
                            "{i + 1}"
                        }
                    }
                }
            }
            button {
                class: "rounded-lg border border-zinc-700 px-2 py-1 text-[11px] text-zinc-300 transition-colors hover:bg-zinc-800 disabled:opacity-40",
                "data-testid": "{testid}-next",
                disabled: current + 1 >= pages,
                onclick: move |_| on_change.call(current + 1),
                "›"
            }
        }
    }
}
