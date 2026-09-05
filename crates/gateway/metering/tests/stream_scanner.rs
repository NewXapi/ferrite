use bytes::Bytes;
use metering::scanner::StreamScanner;

#[test]
fn stream_scanner_extracts_openai_usage() {
    let mut s = StreamScanner::new();
    let _ = s.push(&Bytes::from_static(
        b"data: {\"content\":\"hello\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
    ));
    let counts = s.finish(10);
    assert_eq!(counts.prompt, 10);
    assert_eq!(counts.completion, 5);
}

#[test]
fn stream_scanner_fallback_estimation() {
    let mut s = StreamScanner::new();
    let _ = s.push(&Bytes::from_static(
        b"data: {\"content\":\"hello world\"}\n\n",
    ));
    let counts = s.finish(5);
    assert_eq!(counts.prompt, 5);
    assert!(counts.completion > 0);
}

#[test]
fn stream_scanner_done_line_ignored() {
    let mut s = StreamScanner::new();
    let _ = s.push(&Bytes::from_static(b"data: [DONE]\n\n"));
    let counts = s.finish(0);
    assert_eq!(counts.completion, 0);
}
