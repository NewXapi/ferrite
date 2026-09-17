//! 货币页路由 wiring 一致性回归。
//!
//! 背景：CurrencyPage 的 UI（src/currency.rs）与 /api/currency 适配早已就绪，
//! 但管理区 tab 标签数组只到 index 8（网关健康），面板匹配臂
//! `(Section::Manage, 9) => CurrencyPage` 没有对应 tab 入口，
//! get_initial_route 也缺 `#currency` 映射——页面写好了却不可达（fix/currency-page-route）。
//!
//! 本测试钉死三处接线，任一处脱节即 FAIL：
//! 1. hash 映射：`#currency` → (Section::Manage, 9)；
//! 2. tab 标签：Manage 数组 index 9 = 「货币」；
//! 3. 面板匹配臂：(Section::Manage, 9) 渲染 CurrencyPage。
//!
//! 另外做通用不变量断言（覆盖同类 bug，不局限于货币页）：每个 Manage tab
//! index 都有对应面板匹配臂（无悬空 tab），且每个 hash 映射的 index 落在
//! 标签数组边界内——否则 active_tab 的 clamp `(dash_tab() as usize)
//! .min(labels.len() - 1)` 会把越界 tab 静默吞成末位 tab。
//!
//! 被测对象是路由配置（apps/admin-web，wasm 目标 + web_sys 依赖，无法在
//! 本 crate 的 native 测试里实例化），故采用编译期 include_str! 源码对账：
//! 路径错位在编译期即失败，而非测试时才炸（与 crates/api/admin-billing/
//! tests/invitees_list.rs 的 include_str! 用法同款约定）。

/// 被对账的宿主源码（apps/admin-web 的路由装配）。
const HOST_SRC: &str = include_str!("../../../../apps/admin-web/src/lib.rs");

/// Manage section 面板匹配臂的前缀，用于定位每条 `(Section::Manage, N)`。
const MANAGE_ARM: &str = "(Section::Manage, ";

/// 提取 `s` 中所有双引号字符串字面量（标签/hash 均为纯文本，无转义）。
fn quoted_tokens(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut open = false;
    for ch in s.chars() {
        match ch {
            '"' if !open => open = true,
            '"' if open => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                open = false;
            }
            _ if open => cur.push(ch),
            _ => {}
        }
    }
    out
}

/// Manage section 的 tab 标签数组（`Section::Manage => vec![ ... ]`），按源码顺序。
///
/// 数组是扁平的 `"xx".into(),` 列表，不含嵌套 `]`，故从 `vec![` 扫到首个 `]`。
fn manage_labels(src: &str) -> Vec<String> {
    let tag = "Section::Manage => vec![";
    let start = src
        .find(tag)
        .expect("Manage tab 标签数组（Section::Manage => vec![）必须存在");
    let region = &src[start + tag.len()..];
    let end = region.find(']').expect("Manage tab 标签数组必须以 ] 闭合");
    quoted_tokens(&region[..end])
}

/// `get_initial_route` 里所有 `"#hash" => (Section::Manage, N)` 映射（含 `|` 多 hash 臂）。
fn manage_hash_routes(src: &str) -> Vec<(String, u8)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim();
        if !t.starts_with('"') || !t.contains("=> (Section::Manage,") {
            continue;
        }
        let idx = t.find(MANAGE_ARM).expect("已校验含该前缀");
        let after = &t[idx + MANAGE_ARM.len()..];
        let digits: String = after
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        let Ok(n) = digits.parse::<u8>() else {
            continue;
        };
        let (hashes, _) = t.split_once(" =>").unwrap_or((t, ""));
        for h in quoted_tokens(hashes) {
            if h.starts_with('#') {
                out.push((h, n));
            }
        }
    }
    out
}

/// `(Section::Manage, N)` 面板匹配臂中实际渲染的页面标识符（取臂内首个大驼峰单词）。
/// 返回 (index, 页面名)。`_` 通配臂被跳过。
fn manage_panel_arms(src: &str) -> Vec<(u8, String)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(pos) = rest.find(MANAGE_ARM) {
        let after = &rest[pos + MANAGE_ARM.len()..];
        let digits: String = after
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        rest = after;
        if digits.is_empty() {
            // `_ =>` 通配臂或非字面量 index，跳过。
            continue;
        }
        let Ok(idx) = digits.parse::<u8>() else {
            continue;
        };
        // 臂体到行尾或下一个 `Section::` 之间取首个 PascalCase 标识符作为页面名。
        let body_end = after.find("Section::").unwrap_or(after.len().min(400));
        let body = &after[..body_end];
        let page = body
            .split_whitespace()
            .find(|tok| tok.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
            .map(str::to_string);
        if let Some(name) = page {
            out.push((idx, name));
        }
    }
    out
}

/// `#currency` → (Section::Manage, 9)：缺这条映射，刷新/直达 `#currency` 会回落总览。
#[test]
fn currency_hash_maps_to_manage_index_9() {
    let routes = manage_hash_routes(HOST_SRC);
    assert!(
        routes.iter().any(|(h, n)| h == "#currency" && *n == 9),
        "get_initial_route 必须含 `\"#currency\" => (Section::Manage, 9)`，实际: {routes:?}"
    );
}

/// Manage 标签数组 index 9 = 「货币」，且数组恰为 10 项——index 9 越界时
/// active_tab 的 clamp 会把它吞成 index 8（网关健康）。
#[test]
fn manage_labels_have_currency_at_index_9() {
    let labels = manage_labels(HOST_SRC);
    assert_eq!(
        labels.len(),
        10,
        "Manage 标签数组应为 10 项，实际 {} 项: {labels:?}",
        labels.len()
    );
    assert_eq!(
        labels[9], "货币",
        "Manage tab index 9 必须是「货币」，实际: {}",
        labels[9]
    );
}

/// `(Section::Manage, 9)` 臂渲染 CurrencyPage：tab 点进来必须落到货币面板。
#[test]
fn manage_panel_arm_9_renders_currency_page() {
    let arms = manage_panel_arms(HOST_SRC);
    let page = arms
        .iter()
        .find(|(i, _)| *i == 9)
        .map(|(_, name)| name.as_str())
        .unwrap_or("<<缺失>>");
    assert_eq!(
        page, "CurrencyPage",
        "(Section::Manage, 9) 必须渲染 CurrencyPage，实际: {page}"
    );
}

/// 通用不变量：每个 Manage tab index 都有面板匹配臂。
/// 货币页 bug 的根因正是「有匹配臂无 tab」；反向（有 tab 无臂）会让 tab
/// 点进去渲染空面板。任一方向脱节这里都会 FAIL。
#[test]
fn every_manage_tab_has_panel_arm() {
    let labels = manage_labels(HOST_SRC);
    let arms = manage_panel_arms(HOST_SRC);
    let armed: Vec<u8> = arms.iter().map(|(i, _)| *i).collect();

    for i in 0..labels.len() {
        let i = i as u8;
        assert!(
            armed.contains(&i),
            "Manage tab index {i}（「{}」）缺面板匹配臂 (Section::Manage, {i})",
            labels[i as usize]
        );
    }
    // 无重复臂（重复臂会让 match 展开报警告并可能遮蔽货币页）。
    let mut sorted = armed.clone();
    sorted.sort_unstable();
    let mut deduped = sorted.clone();
    deduped.dedup();
    assert_eq!(
        sorted, deduped,
        "(Section::Manage, N) 匹配臂不应重复: {armed:?}"
    );
}

/// 通用不变量：每个 hash 映射的 index 必须落在标签数组内，否则 clamp 吞掉。
#[test]
fn every_manage_hash_index_is_within_labels() {
    let labels = manage_labels(HOST_SRC);
    let routes = manage_hash_routes(HOST_SRC);
    assert!(
        !routes.is_empty(),
        "get_initial_route 应至少有一条 Manage hash 映射"
    );

    for (hash, n) in &routes {
        assert!(
            (*n as usize) < labels.len(),
            "hash {hash} → index {n} 越出 Manage 标签数组（{} 项），会被 active_tab clamp 吞掉",
            labels.len()
        );
    }
}
