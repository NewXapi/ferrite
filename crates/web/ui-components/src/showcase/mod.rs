//! 只读展示卡牌组件(showcase cards) — 从 admin-page-overview 的三种演示卡牌逐 DOM 抽象而来。
//!
//! 职责与约定:
//! - 纯只读展示组件: 数据全部由调用方通过 props 注入, 组件不做数据获取、不依赖任何
//!   mock/page crate(本 crate 只依赖 contract + dioxus); 仅保留卡内 UI 状态
//!   (翻牌开关 / 鼠标倾斜 / 三内部 tab 切换)。
//! - 样式基座为 admin-web entry.css 的 showcase 类(poster-flip / poster-flip-inner /
//!   card-frame / card-frosted / card-vignette / card-corner-shade / row-text /
//!   row-tip / scroll-subtle 等) + Tailwind 原子类; 本 crate 不携带这些 CSS,
//!   宿主应用需自行引入对应样式表。
//!
//! 组件一览:
//! - [`radar_flip_card::RadarFlipCard`] — 头牌翻牌卡: 左侧翻牌立绘(正面磨砂立绘 /
//!   背面六维明细 + 综合分), 右侧信息面板; card-tilt 鼠标跟随倾斜。
//! - [`poster_card::PosterCard`] — 立绘海报卡: 正面立绘 + 浓缩六维雷达 SVG(排名角标 /
//!   光点 / 均值虚线) + 关键数据行, 点击顺时针翻 180° 看背面明细; 246x368 倾斜 hover。
//! - [`stat_tabs_card::StatTabsCard`] — 三内部 tab 展示卡: 概览(价格行 / 大数字 /
//!   sparkline / 热力条) / 分组价格表 / 待定占位。
//!
//! DOM id 注意: PosterCard 的雷达渐变与动画路径 id 由 `name` 派生
//! (`rank-grad-{name}` / `poster-path-{name}`, 与原实现一致); 同屏展示多张同名卡时
//! 由调用方保证 name 可区分。

pub mod poster_card;
pub mod radar_flip_card;
pub mod stat_tabs_card;

use dioxus::prelude::*;

pub use poster_card::PosterCard;
pub use radar_flip_card::RadarFlipCard;
pub use stat_tabs_card::StatTabsCard;

/// 三种展示卡统一的外层 hover 边框变亮类(维护者点名): 边框色 150ms 过渡到 white/30。
///
/// 仅对本身带 border 的卡生效(RadarFlipCard 外层 border-zinc-800、StatTabsCard 外层
/// border-white/10); PosterCard 外层容器无 border, 其 hover 边框变亮需由宿主在
/// entry.css 的 `.poster-flip:hover` 语境追加规则, 不经过本常量。
pub const HOVER_BORDER_BRIGHT: &str =
    "transition-[border-color] duration-150 hover:border-white/30";

/// 立绘元素(RadarFlipCard / PosterCard 共用): 有图用图, 无图用首字符占位。
///
/// * `art` — 立绘资产; None 时渲染 `alt` 首字符的占位大字。
/// * `alt` — 图片 alt 文案, 通常为模型名(同时是占位字符来源)。
/// * `extra_style` — 追加到 img 的内联样式(如背面暗化 `filter: brightness(0.28); ...`)。
fn art_img(art: Option<Asset>, alt: &str, extra_style: &str) -> Element {
    match art {
        Some(art) => rsx! {
            img {
                class: "absolute inset-0 h-full w-full object-cover",
                style: "{extra_style}",
                src: art,
                alt: "{alt}",
            }
        },
        None => rsx! {
            span { class: "absolute inset-0 flex items-center justify-center text-9xl {crate::T_font_bold} text-zinc-600/40",
                "{alt.chars().next().unwrap_or('?')}"
            }
        },
    }
}
