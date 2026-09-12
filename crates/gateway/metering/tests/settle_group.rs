//! settle / sink 测试 — group-aware 定价链路 + 结算产物落地通道。
//!
//! 覆盖三条行为：
//! 1. `settle_event` 把 `group` 透传进 `PriceTable::lookup`、把 `group_ratio`
//!    乘进 cost（组倍率死链打通的库侧证据）；
//! 2. 未知模型走默认免费价但事件照常产出（下限语义）；
//! 3. [`SettleSink`] 可被 apps 侧以内存 mock 实现并收到事件。

use contract::records::UsageEventRecord;
use metering::ledger::Hold;
use metering::pricing::{ModelPrice, PriceTable};
use metering::scanner::TokenCounts;
use metering::settle::extract_usage;
use metering::{SettleSink, settle_event};
use std::sync::Mutex;

/// Fake 定价表：同 model 按 group 返回不同价——用来证明 group 真的进了 lookup。
struct GroupTable;

impl PriceTable for GroupTable {
    fn lookup(&self, model: &str, group: &str) -> Option<ModelPrice> {
        match (model, group) {
            ("gpt-4o", "vip") => Some(ModelPrice {
                input: 30.0,
                output: 60.0,
                cache: 0.0,
                group_multiplier: 1.0,
            }),
            ("gpt-4o", _) => Some(ModelPrice {
                input: 15.0,
                output: 60.0,
                cache: 0.0,
                group_multiplier: 1.0,
            }),
            _ => None,
        }
    }
}

fn hold() -> Hold {
    Hold {
        id: 0,
        amount: 0,
        user_key: "u1".into(),
        token_key: "t1".into(),
    }
}

/// group 选价 + group_ratio 叠乘：1M prompt 在 vip 价 ($30/M) × ratio 2 → $60。
#[test]
fn settle_event_uses_group_and_group_ratio() {
    let counts = TokenCounts {
        prompt: 1_000_000,
        completion: 0,
        cached: 0,
    };
    let ev = settle_event(
        counts,
        "vip",
        2.0,
        &hold(),
        &GroupTable,
        "ch",
        "ru",
        "gpt-4o",
        "gpt-4o",
        0,
        0,
        200,
        None,
    );
    // $30/M × 1M × ratio 2 = $60 → 60 * 500_000 = 30_000_000
    assert_eq!(ev.cost, 30_000_000);

    // default 组回到 $15/M 且 ratio 1.0 → 7_500_000
    let ev = settle_event(
        counts,
        "default",
        1.0,
        &hold(),
        &GroupTable,
        "ch",
        "ru",
        "gpt-4o",
        "gpt-4o",
        0,
        0,
        200,
        None,
    );
    assert_eq!(ev.cost, 7_500_000);

    // 事件归因键来自 hold
    assert_eq!(ev.user_key, "u1");
    assert_eq!(ev.token_key, "t1");
}

/// 未知模型 → 默认免费价 (0) 但事件仍产出；error/status 原样落事件。
#[test]
fn settle_event_unknown_model_free_and_error_fields_carried() {
    let counts = TokenCounts {
        prompt: 1000,
        completion: 500,
        cached: 0,
    };
    let ev = settle_event(
        counts,
        "vip",
        2.0,
        &hold(),
        &GroupTable,
        "ch",
        "ru",
        "free-model",
        "free",
        0,
        0,
        200,
        None,
    );
    assert_eq!(ev.cost, 0, "无价模型免费, 但仍记账");

    let ev = settle_event(
        counts,
        "default",
        1.0,
        &hold(),
        &GroupTable,
        "ch",
        "ru",
        "gpt-4o",
        "gpt-4o",
        0,
        0,
        500,
        Some("upstream exploded"),
    );
    assert_eq!(ev.status_code, 500);
    assert_eq!(ev.error.as_deref(), Some("upstream exploded"));
}

/// 内存 sink mock（apps 侧实现的同型物）：settle 产物可被收集。
#[derive(Default)]
struct VecSink(Mutex<Vec<UsageEventRecord>>);

impl SettleSink for VecSink {
    fn submit(&self, event: UsageEventRecord) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn sink_receives_settled_events() {
    let sink = VecSink::default();
    let counts = TokenCounts {
        prompt: 100,
        completion: 50,
        cached: 0,
    };
    let ev = settle_event(
        counts,
        "default",
        1.0,
        &hold(),
        &GroupTable,
        "ch",
        "ru",
        "gpt-4o",
        "gpt-4o",
        0,
        0,
        200,
        None,
    );
    sink.submit(ev);

    let events = sink.0.lock().unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].prompt_tokens, 100);
    assert_eq!(events[0].completion_tokens, 50);
    // (100×15 + 50×60)/1e6 × 500_000 = 2_250
    assert_eq!(events[0].cost, 2_250);
}

/// 非流式响应体 usage 提取：OpenAI / Claude 两种形状、缺失与全零回退 None。
#[test]
fn extract_usage_handles_openai_claude_and_absent() {
    // OpenAI: prompt/completion + details.cached_tokens
    let body = br#"{"choices":[],"usage":{"prompt_tokens":11,"completion_tokens":7,"prompt_tokens_details":{"cached_tokens":3}}}"#;
    let c = extract_usage(body).expect("openai usage");
    assert_eq!((c.prompt, c.completion, c.cached), (11, 7, 3));

    // Claude: input/output_tokens + cache_read_input_tokens
    let body = br#"{"usage":{"input_tokens":5,"output_tokens":9,"cache_read_input_tokens":2}}"#;
    let c = extract_usage(body).expect("claude usage");
    assert_eq!((c.prompt, c.completion, c.cached), (5, 9, 2));

    // 缺 usage / 非 JSON / 全零 → None，由调用方估算兜底
    assert!(extract_usage(b"{}").is_none());
    assert!(extract_usage(b"not json").is_none());
    assert!(extract_usage(br#"{"usage":{"prompt_tokens":0,"completion_tokens":0}}"#).is_none());
}
