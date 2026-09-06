//! 过滤词测试 —— `WordFilter` 一次性过滤 + `StreamFilter` 跨 chunk 流式过滤。
//!
//! 断言的是消费者能观察到的契约：过滤结果与零分配热路径。
//! `StreamFilter` 单次 `push` 返回多少字节是内部 hold 策略，不钉死。

use std::borrow::Cow;
use std::sync::Arc;

use gateway_security::{FilterConfig, StreamFilter, WordFilter};

/// 在 `wordlist.rs` 中的 FilterConfig 默认行为与 serde 行为一致。
#[test]
fn test_filter_config_default() {
    // 仅给定 words 时，replacement 应为 "***"，两个开关均为 true
    let config = FilterConfig {
        words: vec!["secret".to_string()],
        ..Default::default()
    };
    assert_eq!(config.replacement, "***");
    assert!(config.filter_request);
    assert!(config.filter_response);
}

/// 空词表时，WordFilter::is_empty 返回 true；filter 返回原样文本（零分配）。
#[test]
fn test_empty_wordlist() {
    let config = FilterConfig {
        words: vec![],
        replacement: "***".to_string(),
        filter_request: true,
        filter_response: true,
    };
    let filter = WordFilter::new(&config);
    assert!(filter.is_empty());

    // 无匹配时 filter 返回 Cow::Borrowed（零分配）
    let text = "Hello world";
    match filter.filter(text) {
        Cow::Borrowed(s) => assert_eq!(s, text),
        Cow::Owned(_) => panic!("空词表时应该返回 Borrowed"),
    }

    // has_match 应该为 false
    assert!(!filter.has_match(text));
}

/// 大小写不敏感替换正确。包含大小写变体的 "Secret" 应该被替换为 "***"。
#[test]
fn test_case_insensitive_match() {
    let config = FilterConfig {
        words: vec!["secret".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = WordFilter::new(&config);
    assert!(filter.has_match("Secret"));
    assert_eq!(filter.filter("Secret").as_ref(), "***");
    assert_eq!(filter.filter("my secret text").as_ref(), "my *** text");
}

/// 多词、重叠词命中时，替换应用正确（AhoCorasick 保证正确性）。
#[test]
fn test_multiple_matches() {
    let config = FilterConfig {
        words: vec!["bad".to_string(), "word".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = WordFilter::new(&config);
    let text = "This is a bad word and another bad.";
    assert_eq!(
        filter.filter(text).as_ref(),
        "This is a *** *** and another ***."
    );
}

/// 把 chunk 序列喂进 `StreamFilter`，返回 push 输出与 flush 拼接后的完整结果。
///
/// 这是 `StreamFilter` 的唯一契约：客户端看到的是拼接流，单次 `push` 返回多少
/// 字节是内部 hold 策略，不该被测试钉死。
fn drive(words: &[&str], replacement: &str, chunks: &[&str]) -> String {
    let config = FilterConfig {
        words: words.iter().map(|w| w.to_string()).collect(),
        replacement: replacement.to_string(),
        ..Default::default()
    };
    let filter = Arc::new(WordFilter::new(&config));
    let mut sf = StreamFilter::new(filter);
    let mut out = String::new();
    for c in chunks {
        out.push_str(&sf.push(c));
    }
    out.push_str(&sf.flush());
    out
}

/// 跨 chunk 命中：词 "secret" 被切成 "my sec" + "ret here"，单看任一 chunk 都不命中。
/// 这是 `StreamFilter` 存在的唯一理由——`WordFilter` 逐 chunk 调会漏放。
#[test]
fn stream_filter_catches_word_split_across_chunks() {
    assert_eq!(drive(&["secret"], "***", &["my sec", "ret here"]), "my *** here");
    // 对照：逐 chunk 用一次性过滤器会漏放，证明跨 chunk 逻辑不是多余的。
    let one_shot = WordFilter::new(&FilterConfig {
        words: vec!["secret".into()],
        replacement: "***".into(),
        ..Default::default()
    });
    let leaked: String = ["my sec", "ret here"]
        .iter()
        .map(|c| one_shot.filter(c).into_owned())
        .collect();
    assert_eq!(leaked, "my secret here", "逐 chunk 过滤必然漏放");
}

/// UTF-8 边界：中文一个字 3 字节，`hold_len` 按字节算时会落在字中间，必须回退到
/// 字符边界，否则 `is_char_boundary` 之外的切片直接 panic。
///
/// 词 "敏感" 是 6 字节 → `hold_len = 5`，逐字（3 字节）喂入时每次切分都要回退。
#[test]
fn stream_filter_handles_utf8_split_mid_character() {
    let text = "这是敏感内容";
    let chunks: Vec<String> = text.chars().map(|c| c.to_string()).collect();
    let refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
    assert_eq!(drive(&["敏感"], "***", &refs), "这是***内容");

    // 混合切分：把 "敏感" 的两个字分到不同 chunk，且首尾带 ASCII。
    assert_eq!(drive(&["敏感"], "#", &["ab敏", "感cd"]), "ab#cd");
}

/// 词横跨三个 chunk，每个 chunk 只有 2 字节。
#[test]
fn stream_filter_catches_word_split_across_three_chunks() {
    assert_eq!(drive(&["abcdef"], "***", &["ab", "cd", "ef"]), "***");
    assert_eq!(drive(&["abcdef"], "***", &["xab", "cd", "efy"]), "x***y");
}

/// 紧邻重复词：`abababab` 里 `ab` 连续出现，hold 边界若只按固定长度算会拆开漏放。
/// 这是回归测试——曾经的实现在这里既 panic 又漏放。
#[test]
fn stream_filter_handles_adjacent_repeated_words() {
    assert_eq!(drive(&["ab"], "", &["ababababab"]), "");
    assert_eq!(drive(&["ab"], "-", &["aba", "bab", "ab"]), "----");
}

/// `flush` 后状态复位，同一实例可继续用于下一条流。
#[test]
fn stream_filter_resets_after_flush() {
    let filter = Arc::new(WordFilter::new(&FilterConfig {
        words: vec!["secret".into()],
        replacement: "***".into(),
        ..Default::default()
    }));
    let mut sf = StreamFilter::new(filter);

    let mut first = sf.push("secret");
    first.push_str(&sf.flush());
    assert_eq!(first, "***");

    // flush 后不残留上一条流的字节。
    let mut second = sf.push("plain");
    second.push_str(&sf.flush());
    assert_eq!(second, "plain");
}

/// 替换串比词长/短都不能错位：长度变化不影响 hold 边界计算。
#[test]
fn stream_filter_handles_replacement_length_change() {
    // 缩短：6 字节 → 0 字节
    assert_eq!(drive(&["secret"], "", &["a secret b"]), "a  b");
    // 变长：1 字节 → 4 字节
    assert_eq!(drive(&["x"], "LONG", &["axbxc"]), "aLONGbLONGc");
}

/// 空词表：不缓冲、不分配，原样透传。缓冲会凭空增加首字延迟。
#[test]
fn stream_filter_passes_through_when_wordlist_empty() {
    assert_eq!(drive(&[], "***", &["hello ", "world"]), "hello world");
}
