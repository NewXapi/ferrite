//! i18n 常量表一致性测试。
//!
//! 守护「移植铁律」：ZH 与 EN 两表 key 集合必须严格一致 —— 任何一边漏加
//! 都会在这里直接失败，而不是等到界面上回落出中文才被发现。

use ui_components::i18n::{EN, Locale, ZH, plural, t_in};

/// 两表 key 集合严格一致（顺序无关）：新增 key 必须中英成对。
#[test]
fn zh_en_key_sets_match() {
    let zh_keys: std::collections::BTreeSet<&str> = ZH.iter().map(|(k, _)| *k).collect();
    let en_keys: std::collections::BTreeSet<&str> = EN.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        zh_keys,
        en_keys,
        "ZH/EN key 集合不一致：仅 ZH 有 {:?}，仅 EN 有 {:?}",
        zh_keys.difference(&en_keys).collect::<Vec<_>>(),
        en_keys.difference(&zh_keys).collect::<Vec<_>>()
    );
}

/// 表内 key 不重复（线性查表取首个命中，重复 key 会让后者成死条目）。
#[test]
fn no_duplicate_keys() {
    for (name, table) in [("ZH", ZH), ("EN", EN)] {
        let mut seen = std::collections::HashSet::new();
        for (k, _) in table {
            assert!(seen.insert(k), "{name} 表存在重复 key: {k}");
        }
    }
}

/// 指定 locale 查表：命中取对应语言；未知 key 回落中文，再落空则原样返回 key。
#[test]
fn lookup_and_fallback() {
    assert_eq!(t_in(Locale::Zh, "common.retry"), "重试");
    assert_eq!(t_in(Locale::En, "common.retry"), "Retry");
    // 英文表缺 key 时回落中文（此行为靠 key 集合测试兜底为不可达，仍钉住语义）
    assert_eq!(t_in(Locale::En, "nonexistent.key"), "nonexistent.key");
}

/// 复数形：zh 无词形变化；en 按 1 / 非 1 分词。
#[test]
fn plural_rules() {
    assert_eq!(plural(Locale::Zh, 1, "record", "条记录"), "条记录");
    assert_eq!(plural(Locale::Zh, 5, "record", "条记录"), "条记录");
    assert_eq!(plural(Locale::En, 1, "record", "records"), "record");
    assert_eq!(plural(Locale::En, 0, "record", "records"), "records");
    assert_eq!(plural(Locale::En, 2, "record", "records"), "records");
}

/// locale 短码与显示名稳定（持久化与语言切换 UI 依赖）。
#[test]
fn locale_codes() {
    assert_eq!(Locale::Zh.code(), "zh");
    assert_eq!(Locale::En.code(), "en");
    assert_eq!(Locale::ALL.len(), 2);
}
