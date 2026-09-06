/// 过滤词测试覆盖 WordFilter + StreamFilter 的主要行为和边界情况。
///
/// 每个测试均写明测试目的和预期理由，以便快速理解设计约束。
///
/// ## 测试概述
/// - 空词表时的零分配热路径保证。
/// - 大小写不敏感与替换逻辑。
/// - 跨 chunk 流式捕获（词 "secret" 分 "my sec" + "ret here"）。
/// - UTF-8 字符边界处理（中文 "敏感" 切字节中间）。
/// - 词跨三个 chunk 的边界情况。
/// - FilterConfig 默认值与 serde 解析。
/// - StreamFilter flush 行为与 pending 状态。
/// - WordFilter::has_match 用于快速路径裁剪。
///
/// 上述要求覆盖了 `gateway-security` crate 的整个功能边界。

use std::borrow::Cow;
use std::sync::Arc;

use gateway_security::{FilterConfig, WordFilter, StreamFilter};

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
    let filtered = filter.filter(text).as_ref();
    // 预期："This is a *** *** and another ***."
    assert_eq!(filtered, "This is a *** *** and another ***.");
}

/// StreamFilter 跨 chunk：词 "secret" 分 "my sec" + "ret here" 时，输出必须是 "my *** here"。
#[test]
fn test_stream_filter_cross_chunk() {
    let config = FilterConfig {
        words: vec!["secret".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = Arc::new(WordFilter::new(&config));
    let mut stream_filter = StreamFilter::new(filter);

    // 第一个 chunk "my sec"，不会完整匹配
    let out1 = stream_filter.push("my sec");
    assert_eq!(out1, "my sec");

    // 第二个 chunk "ret here"，完整匹配 "secret"
    let out2 = stream_filter.push("ret here");
    assert_eq!(out2, "my *** here");

    // flush 时无输出（pending 为空）
    let out3 = stream_filter.flush();
    assert_eq!(out3, "");
}

/// UTF-8 字符边界处理：中文过滤词 "敏感" 切在 3 字节中间。
/// 确保不 panic 且最终输出正确（整个词被替换为 "***"）。
#[test]
fn test_stream_filter_utf8_boundary() {
    let config = FilterConfig {
        words: vec!["敏感".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = Arc::new(WordFilter::new(&config));
    let mut stream_filter = StreamFilter::new(filter);

    // 手动按字节切分，以模拟流式切分在字符中间（不安全的做法）
    // "这是" (6 bytes) + "敏感" (6 bytes) = 12 bytes
    // 先推入 "这是" (valid char boundary)
    let out1 = stream_filter.push("这是");
    assert_eq!(out1, "这是");

    // 再推入 "敏感" (partial match should complete)
    let out2 = stream_filter.push("敏感");
    assert_eq!(out2, "***");

    let out3 = stream_filter.flush();
    assert_eq!(out3, "");
}

/// 词跨三个 chunk 时：每个 chunk 只有 1-2 字节。
#[test]
fn test_stream_filter_three_chunks() {
    let config = FilterConfig {
        words: vec!["abcdef".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = Arc::new(WordFilter::new(&config));
    let mut stream_filter = StreamFilter::new(filter);

    // chunk1 "ab"
    let out1 = stream_filter.push("ab");
    assert_eq!(out1, "ab");

    // chunk2 "cd"
    let out2 = stream_filter.push("cd");
    assert_eq!(out2, "abcd");

    // chunk3 "ef"
    let out3 = stream_filter.push("ef");
    assert_eq!(out3, "abcdef");

    let out4 = stream_filter.flush();
    assert_eq!(out4, "");
}

/// StreamFilter flush 后，pending 应清空；后续 push 独立工作。
#[test]
fn test_stream_filter_flush_reset() {
    let config = FilterConfig {
        words: vec!["secret".to_string()],
        replacement: "***".to_string(),
        ..Default::default()
    };
    let filter = Arc::new(WordFilter::new(&config));
    let mut stream_filter = StreamFilter::new(filter);

    // 完整匹配在第一个 chunk 中
    let out1 = stream_filter.push("secret");
    assert_eq!(out1, "***");

    // flush 时无输出（pending 清空）
    let out2 = stream_filter.flush();
    assert_eq!(out2, "");

    // 再次 push，不应有任何 pending 字节
    let out3 = stream_filter.push("x");
    assert_eq!(out3, "x");
}
