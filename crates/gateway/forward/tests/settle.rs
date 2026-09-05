//! settle 集成测试 — 验证流式转发经扫描链后自动结算。

use bytes::Bytes;
use forward::stream::{SseContext, pipe_chunk};
use metering::scanner::StreamScanner;

#[test]
fn stream_scanner_extracts_usage_for_settle() {
    let mut s = StreamScanner::new();
    s.push(&Bytes::from_static(
        b"data: {\"content\":\"hello\",\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5}}\n\n",
    ));
    let counts = s.finish(10);
    assert_eq!(counts.prompt, 10);
    assert_eq!(counts.completion, 5);
}

#[test]
fn sse_context_pipe_chunk_detects_first_token() {
    let mut ctx = SseContext::new();
    let out = pipe_chunk(
        &mut ctx,
        &Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n"),
    );
    assert_eq!(
        out.passthrough,
        Bytes::from_static(b"data: {\"role\":\"assistant\"}\n\n")
    );
    assert_eq!(out.events.len(), 1);
}

#[test]
fn sse_context_settle_generates_usage_event() {
    use metering::pricing::{ModelPrice, PriceTable};
    use metering::scanner::TokenCounts;

    struct FixedPriceTable;
    impl PriceTable for FixedPriceTable {
        fn lookup(&self, _model: &str) -> Option<ModelPrice> {
            Some(ModelPrice {
                input: 15.0,
                output: 60.0,
                cache: 0.0,
                group_multiplier: 1.0,
            })
        }
    }

    let hold = metering::ledger::Hold {
        id: 1,
        amount: 100,
        user_key: "user1".into(),
        token_key: "tok1".into(),
    };
    let counts = TokenCounts {
        prompt: 100,
        completion: 50,
        cached: 0,
    };
    let pt = FixedPriceTable;
    let event = metering::settle_event(
        counts, &hold, &pt, "ch1", "u1", "gpt-4o", "gpt-4o", 100, 500, 200, None,
    );
    assert_eq!(event.prompt_tokens, 100);
    assert_eq!(event.completion_tokens, 50);
    assert!(event.cost > 0);
    assert_eq!(event.status_code, 200);
}
