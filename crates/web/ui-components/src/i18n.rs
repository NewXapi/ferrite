//! 跨 crate 文案抽象（i18n 第 1 层）：locale 状态、key 解析与复数形。
//!
//! ## 定位与边界
//!
//! - 这是「零依赖 locale 机制层」：只有两个编译期常量表（zh/en）+ 一个
//!   `GlobalSignal<Locale>` + 查表函数。**不引入 fluent / rust-i18n 等运行时
//!   依赖**，不解析 `.ftl` 文件；页面 crate 的文案仍按前缀常量
//!   （`SEC_*` / `LBL_*` / `BTN_*` / `MSG_*`）集中在各自 `tab-page-*/shared.rs`，
//!   本模块只负责「同一 key 的多语言取值」。
//! - **移植铁律：从中文抽 key 时必须中英成对补齐。** 每抽一条中文文案，必须同时
//!   在本文件的 `ZH` 与 `EN` 两张表里各加一行（key 相同）；只加中文不加英文的
//!   key 会在英文界面回落成中文（见 [`t`] 的回落链），属于必须清零的欠账。
//!   禁止把 `<span class="x">中文</span>` 换成 `<span class="x">中文</span>`
//!   这种 class 照抄、只抽半截文案的"伪迁移"。
//! - 本模块不持有任何业务文案的归属判断：key 该叫什么、归哪个 tab，由使用方
//!   在自己的 `shared.rs` 里决定；这里只管解析。
//!
//! ## 设计取舍
//!
//! - 用 `GlobalSignal` 而非 `use_context`：admin-web / tavern-web 所有页面 crate
//!   都在同一组件树下，全局信号免去逐层 provide/inject，且 crate 之间不需要
//!   共享 context 类型。
//! - 查表用 `const` 数组 + 线性扫（`&'static [(&str, &str)]`）：文案条目量级
//!   在百级，线性扫的缓存友好性优于 `phf`；零构建脚本、零 proc-macro 依赖，
//!   wasm32 可直接编。
//! - 复数形走 [`plural`] 显式函数，不做 CLDR 自动选择：项目当前只有 zh/en
//!   两个 locale，规则（other / one-other）写死在 match 里比引入 icu4x 更诚实。

use dioxus::prelude::*;

/// 界面语言。
///
/// 目前只覆盖 zh / en 两种；新增语言时在此加变体、补对应常量表，并扩展
/// [`plural`] 的分词规则。
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Locale {
    /// 中文（默认）
    Zh,
    /// English
    En,
}

impl Locale {
    /// 全部可用 locale（供语言切换 UI 遍历）。
    pub const ALL: [Locale; 2] = [Locale::Zh, Locale::En];

    /// locale 的稳定短码（`<html lang>`、持久化存储用）。
    pub fn code(self) -> &'static str {
        match self {
            Locale::Zh => "zh",
            Locale::En => "en",
        }
    }

    /// 该 locale 的显示名（语言切换器里展示，始终用目标语言自称）。
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::Zh => "中文",
            Locale::En => "English",
        }
    }
}

/// 全局当前 locale。默认中文；语言切换器写它即全站即时切换。
///
/// 放在 `GlobalSignal` 而非 context：页面 crate 无需感知 provider 层级，
/// 任何深度组件直接 `t(key)` 即可读取当前语言。
pub static LOCALE: GlobalSignal<Locale> = Signal::global(|| Locale::Zh);

/// 按键取当前语言的文案。
///
/// 回落链：当前语言表 → 中文表 → key 原样返回（便于 grep 出漏配 key）。
/// 查表为线性扫，条目百级内耗时可忽略；key 必须来自编译期常量表，
/// 不要拼接动态 key。
pub fn t(key: &str) -> &str {
    t_in(LOCALE(), key)
}

/// 按指定 locale 取文案（不读全局信号；测试与服务端场景用）。
pub fn t_in(locale: Locale, key: &str) -> &str {
    let primary = match locale {
        Locale::Zh => ZH,
        Locale::En => EN,
    };
    primary
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
        .or_else(|| ZH.iter().find(|(k, _)| *k == key).map(|(_, v)| *v))
        .unwrap_or(key)
}

/// 按数量选择单复数文案。
///
/// - zh：无词形变化，恒取 `other`（`plural(3, "条记录", "records")` → `"条记录"`）。
/// - en：`count == 1` 取 `one`，否则取 `other`。
///
/// 调用方负责把 `count` 拼进结果（如 `format!("{count} {}", plural(...))`），
/// 本函数只解决词形，不做插值。
pub fn plural(locale: Locale, count: i64, one: &'static str, other: &'static str) -> &'static str {
    match locale {
        Locale::Zh => other,
        Locale::En => {
            if count == 1 {
                one
            } else {
                other
            }
        }
    }
}

// ============ 文案常量表（i18n 第 1 层） ============
//
// key 命名：`<域>.<语义>` 小写点分（如 `common.retry` / `overview.stats.total_users`）。
// 两表 key 集合必须严格一致 —— 新增 key 时中英同时加，少一边即回落欠账。

/// 中文文案表。
pub const ZH: &[(&str, &str)] = &[
    ("common.retry", "重试"),
    ("common.loading", "正在加载…"),
    ("common.no_data", "暂无数据"),
];

/// English copy table. Keys must mirror [`ZH`] exactly.
pub const EN: &[(&str, &str)] = &[
    ("common.retry", "Retry"),
    ("common.loading", "Loading…"),
    ("common.no_data", "No data"),
];
