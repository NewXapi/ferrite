use tavern_client::SseEvent;
use tavern_client::parse_sse_line;

#[test]
fn test_parse_sse_line_with_data_content() {
    let line = "data: hello world";
    let event = parse_sse_line(line);
    assert_eq!(event, Some(SseEvent::Message("hello world".to_string())));
}

#[test]
fn test_parse_sse_line_with_json_delta() {
    let line = "data: {\"choices\":[{\"delta\":{\"content\":\"incremental\"}}]}";
    let event = parse_sse_line(line);
    assert_eq!(event, Some(SseEvent::Message("incremental".to_string())));
}

#[test]
fn test_parse_sse_line_without_delta() {
    let line = "data: plain text";
    let event = parse_sse_line(line);
    assert_eq!(event, Some(SseEvent::Message("plain text".to_string())));
}

#[test]
fn test_parse_sse_line_done() {
    let line = "data: [DONE]";
    let event = parse_sse_line(line);
    assert_eq!(event, Some(SseEvent::Done));
}

#[test]
fn test_parse_sse_line_empty() {
    let line = "";
    let event = parse_sse_line(line);
    assert_eq!(event, None);
}

#[test]
fn test_parse_sse_line_no_data_prefix() {
    let line = "event: test\nevent: message";
    let event = parse_sse_line(line);
    assert_eq!(event, None);
}

#[test]
fn test_parse_sse_line_multiple_delta_messages() {
    let line1 = "data: hello";
    let line2 = "data: {\"choices\":[{\"delta\":{\"content\":\",\"}}]}";
    let line3 = "data: world";
    let event1 = parse_sse_line(line1);
    let event2 = parse_sse_line(line2);
    let event3 = parse_sse_line(line3);
    assert_eq!(event1, Some(SseEvent::Message("hello".to_string())));
    assert_eq!(event2, Some(SseEvent::Message(",".to_string())));
    assert_eq!(event3, Some(SseEvent::Message("world".to_string())));
}

#[test]
fn test_parse_sse_line_json_without_content() {
    let line = "data: {\"foo\": \"bar\"}";
    let event = parse_sse_line(line);
    assert_eq!(
        event,
        Some(SseEvent::Message("{\"foo\": \"bar\"}".to_string()))
    );
}

#[test]
fn test_parse_sse_line_invalid_json() {
    let line = "data: {\"invalid\": }";
    let event = parse_sse_line(line);
    assert_eq!(
        event,
        Some(SseEvent::Message("{\"invalid\": }".to_string()))
    );
}

#[test]
fn test_parse_sse_line_whitespace_handling() {
    // 行首空白被 trim（SSE 规范允许行首缩进），但 `data: ` 之后的载荷
    // 会被 trim：避免无意义的空白字符影响流式拼接。
    let line = "  data:   whitespace test  ";
    let event = parse_sse_line(line);
    assert_eq!(
        event,
        Some(SseEvent::Message("whitespace test".to_string()))
    );
}

#[test]
fn test_sse_event_message_string() {
    let event = SseEvent::Message("test".to_string());
    match event {
        SseEvent::Message(s) => assert_eq!(s, "test"),
        _ => panic!("Expected Message variant"),
    }
}

#[test]
fn test_sse_event_done() {
    let event = SseEvent::Done;
    assert!(matches!(event, SseEvent::Done));
}
