//! `FormatRegistry` 两跳路由的行为测试：方向不串、零转换直通、未注册错误、
//! 每流编码器状态隔离。
//!
//! 用假 codec（记录调用序列 + 在产物里留可识别标记）而非内置 codec —— 内置实现
//! 由 WP2-WP5 落地，本文件测的是 WP1 这层的调度语义，不依赖任何具体格式。

use bytes::Bytes;
use gateway_protocol_bridge::adaptor::{AdaptorError, Protocol};
use gateway_protocol_bridge::format_codec::{FormatCodec, FormatRegistry, StreamEncoder};
use gateway_protocol_bridge::ir::{
    ContentBlock, LlmRequest, LlmResponse, StopReason, StreamEvent, Usage,
};
use serde_json::{Map, Value, json};
use std::sync::{Arc, Mutex};

// ---------- 假 codec ----------

/// 记录调用序列、并在产物里留格式标记的假 codec。
///
/// `encode_*` 的输出带 `"encoded_by": "<格式>"`，让调用方能断言「产物确实出自目标
/// 格式的 encode」——比只断言 `is_ok()` 强：断言的是可观测产物，不是调用痕迹。
struct FakeCodec {
    format: Protocol,
    calls: Arc<Mutex<Vec<String>>>,
}

impl FakeCodec {
    fn tag(&self) -> &'static str {
        match self.format {
            Protocol::OpenAi => "openai",
            Protocol::Claude => "claude",
            Protocol::Gemini => "gemini",
            Protocol::OpenAIResp => "openai_resp",
            Protocol::Passthrough => "passthrough",
        }
    }

    fn record(&self, what: &str) {
        self.calls
            .lock()
            .unwrap()
            .push(format!("{}::{what}", self.tag()));
    }
}

impl FormatCodec for FakeCodec {
    fn format(&self) -> Protocol {
        self.format
    }

    fn decode_request(&self, body: Bytes) -> Result<LlmRequest, AdaptorError> {
        self.record("decode_request");
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        Ok(mk_request(
            v.get("model").and_then(Value::as_str).unwrap_or(""),
        ))
    }

    fn encode_request(&self, req: &LlmRequest) -> Result<Bytes, AdaptorError> {
        self.record("encode_request");
        Ok(Bytes::from(
            serde_json::to_vec(&json!({
                "encoded_by": self.tag(),
                "model": req.model,
            }))
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))?,
        ))
    }

    fn decode_response(&self, body: Bytes) -> Result<LlmResponse, AdaptorError> {
        self.record("decode_response");
        let v: Value =
            serde_json::from_slice(&body).map_err(|e| AdaptorError::DecodeFailed(e.to_string()))?;
        Ok(LlmResponse {
            id: v
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("fallback")
                .to_string(),
            model: "m".into(),
            outputs: vec![ContentBlock::Text { text: "x".into() }],
            usage: Usage {
                prompt_tokens: 1,
                completion_tokens: 2,
                cached_tokens: None,
            },
            stop_reason: StopReason::Stop,
        })
    }

    fn encode_response(&self, resp: &LlmResponse) -> Result<Bytes, AdaptorError> {
        self.record("encode_response");
        Ok(Bytes::from(
            serde_json::to_vec(&json!({
                "encoded_by": self.tag(),
                "id": resp.id,
            }))
            .map_err(|e| AdaptorError::EncodeFailed(e.to_string()))?,
        ))
    }

    fn decode_event(&self, data: &str) -> Result<Vec<StreamEvent>, AdaptorError> {
        self.record("decode_event");
        Ok(vec![StreamEvent::TextDelta {
            index: 0,
            text: data.to_string(),
        }])
    }

    fn stream_encoder(&self) -> Box<dyn StreamEncoder> {
        self.record("stream_encoder");
        Box::new(FakeEncoder {
            tag: self.tag(),
            // 跨事件累积：finish() 之前不吐字节，证明状态真的留在了 encoder 里。
            pending: Vec::new(),
        })
    }
}

/// 累积式编码器：`encode_event` 只暂存，`finish` 一次性吐出全部累积内容。
///
/// 形状故意与「逐事件 1:1 直吐」相反——若 `stream_encoder()` 每次都新建（无共享状态，
/// 契约要求如此）且调用方正确持有同一个实例，`finish` 就能看到全部事件；
/// 若状态被跨流复用或丢失，断言立刻失败。
struct FakeEncoder {
    tag: &'static str,
    pending: Vec<String>,
}

impl StreamEncoder for FakeEncoder {
    fn encode_event(&mut self, ev: &StreamEvent) -> Result<Vec<Bytes>, AdaptorError> {
        match ev {
            StreamEvent::TextDelta { text, .. } => self.pending.push(text.clone()),
            other => self.pending.push(format!("{other:?}")),
        }
        // 累积期不吐字节：这是合法的（trait 文档写明可能返回零帧）。
        Ok(vec![])
    }

    fn finish(&mut self) -> Result<Vec<Bytes>, AdaptorError> {
        Ok(vec![Bytes::from(format!(
            "{}:{}",
            self.tag,
            self.pending.join("|")
        ))])
    }
}

// ---------- 夹具 ----------

fn mk_request(model: &str) -> LlmRequest {
    LlmRequest {
        model: model.into(),
        messages: vec![],
        system: vec![],
        tools: vec![],
        tool_choice: None,
        sampling: None,
        stream: false,
        extra: Map::new(),
    }
}

fn mk_registry(formats: &[Protocol]) -> (FormatRegistry, Arc<Mutex<Vec<String>>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let mut reg = FormatRegistry::new();
    for fmt in formats {
        reg.register(Arc::new(FakeCodec {
            format: *fmt,
            calls: calls.clone(),
        }));
    }
    (reg, calls)
}

/// 非法 JSON 的二进制垃圾：任何真解析它的 codec 都会失败。
const GARBAGE: &[u8] = b"\x00\xff\x01prose not json";

// ---------- 测试 ----------

// 1. 两跳路由：src 解码 + dst 编码，产物必须出自 dst 的 encode。
//    防的是「注册了但没走两跳」——例如错查 pair 表、或只调了一侧就返回原 body。
#[test]
fn translate_request_runs_two_hops_and_returns_dst_encoding() {
    let (reg, calls) = mk_registry(&[Protocol::Claude, Protocol::Gemini]);

    let out = reg
        .translate_request(
            Protocol::Claude,
            Protocol::Gemini,
            Bytes::from_static(b"{\"model\":\"m1\"}"),
        )
        .expect("两跳应成功");

    let v: Value = serde_json::from_slice(&out).expect("产物是 JSON");
    assert_eq!(
        v["encoded_by"], "gemini",
        "产物必须出自目标格式的 encode，实际: {v}"
    );
    assert_eq!(v["model"], "m1", "IR 必须把 model 原样带到第二跳");

    assert_eq!(
        *calls.lock().unwrap(),
        vec!["claude::decode_request", "gemini::encode_request"],
        "两跳应恰好是 src.decode_request → dst.encode_request"
    );
}

// 2. 方向不串：请求方向只碰 request 方法，响应方向只碰 response 方法。
//    旧架构的坑是「拿请求方向的 codec 转响应 → 502 Unsupported」；新抽象下方向由
//    方法名承载，本用例锁死这条契约，防止将来把方向塞回某个标志位。
#[test]
fn translate_keeps_request_and_response_directions_separate() {
    let (reg, calls) = mk_registry(&[Protocol::Claude, Protocol::OpenAi]);

    reg.translate_request(
        Protocol::Claude,
        Protocol::OpenAi,
        Bytes::from_static(b"{\"model\":\"m\"}"),
    )
    .expect("请求方向应成功");

    let after_req = calls.lock().unwrap().clone();
    assert!(
        !after_req.iter().any(|c| c.contains("response")),
        "translate_request 不得触碰任何 response 方法，实际: {after_req:?}"
    );

    calls.lock().unwrap().clear();

    reg.translate_response(
        Protocol::OpenAi,
        Protocol::Claude,
        Bytes::from_static(b"{\"id\":\"r1\"}"),
    )
    .expect("响应方向应成功");

    let after_resp = calls.lock().unwrap().clone();
    assert_eq!(
        after_resp,
        vec!["openai::decode_response", "claude::encode_response"],
        "translate_response 应恰好走 response 两个方法"
    );
    assert!(
        !after_resp.iter().any(|c| c.contains("request")),
        "translate_response 不得触碰任何 request 方法，实际: {after_resp:?}"
    );
}

// 3. 零转换直通：同格式、或任一侧是 Passthrough 的场合，字节必须原样返回。
//    用非法 JSON 当载荷——只要有任何一侧真去解析它就会报错，故「原样返回」是可证伪的。
//    防的是「同格式也强行过 IR」：那会把上游的非标准/超大 body 解析坏。
#[test]
fn translate_passthrough_returns_bytes_verbatim() {
    // 注册表故意为空：直通不依赖任何 codec。
    let reg = FormatRegistry::new();
    let garbage = Bytes::from_static(GARBAGE);

    for (src, dst) in [
        (Protocol::Claude, Protocol::Claude),
        (Protocol::OpenAi, Protocol::OpenAi),
        (Protocol::OpenAIResp, Protocol::OpenAIResp),
        (Protocol::Passthrough, Protocol::Claude),
        (Protocol::Gemini, Protocol::Passthrough),
    ] {
        let out = reg
            .translate_request(src, dst, garbage.clone())
            .unwrap_or_else(|e| panic!("{src:?}→{dst:?} 应直通，却失败: {e}"));
        assert_eq!(out, garbage, "{src:?}→{dst:?} 请求直通必须原样返回");

        let out = reg
            .translate_response(src, dst, garbage.clone())
            .unwrap_or_else(|e| panic!("{src:?}→{dst:?} 响应应直通，却失败: {e}"));
        assert_eq!(out, garbage, "{src:?}→{dst:?} 响应直通必须原样返回");
    }
}

// 4. 未注册格式：resolve 返回 None，translate 返回 NotRegistered（不得 panic、不得
//    静默直通）。防的是「查不到 codec 就当透传」——那会把没实现的转换伪装成成功，
//    在数据面上表现为响应形状对不上而不是明确报错。
#[test]
fn unregistered_format_reports_not_registered() {
    let (reg, calls) = mk_registry(&[Protocol::Claude]);

    assert!(
        reg.resolve(Protocol::Gemini).is_none(),
        "未注册格式 resolve 应为 None"
    );
    assert!(
        reg.resolve(Protocol::Claude).is_some(),
        "已注册格式 resolve 应有值"
    );

    let err = reg
        .translate_request(
            Protocol::Claude,
            Protocol::Gemini,
            Bytes::from_static(b"{\"model\":\"m\"}"),
        )
        .expect_err("目标格式未注册应报错");

    match err {
        AdaptorError::NotRegistered { from, to } => {
            assert_eq!(from, Protocol::Claude);
            assert_eq!(to, Protocol::Gemini);
        }
        other => panic!("期望 NotRegistered，实际: {other}"),
    }

    // 源格式未注册同样必须报错（不能因为 dst 存在就放行）。
    let err = reg
        .translate_response(
            Protocol::Gemini,
            Protocol::Claude,
            Bytes::from_static(b"{}"),
        )
        .expect_err("源格式未注册应报错");
    assert!(matches!(err, AdaptorError::NotRegistered { .. }));

    // 两次失败都不应产生任何 codec 调用。
    assert_eq!(
        calls.lock().unwrap().len(),
        0,
        "未注册路径不得调用任何 codec 方法"
    );
}

// 5. 流编码器每流新建、状态互不串：两个 encoder 各喂各的事件，finish 只应看到自己的。
//    防的是「流状态挂在 codec 上」——那会让并发流互相污染（前一条流的文本漏进后一条）。
#[test]
fn stream_encoders_do_not_share_state() {
    let (reg, _calls) = mk_registry(&[Protocol::Claude]);
    let codec = reg.resolve(Protocol::Claude).expect("已注册");

    let mut a = codec.stream_encoder();
    let mut b = codec.stream_encoder();

    a.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "alpha".into(),
    })
    .expect("a 编码");
    b.encode_event(&StreamEvent::TextDelta {
        index: 0,
        text: "beta".into(),
    })
    .expect("b 编码");

    let out_a = a.finish().expect("a 收尾");
    let out_b = b.finish().expect("b 收尾");

    let a_text = String::from_utf8_lossy(&out_a.concat()).to_string();
    let b_text = String::from_utf8_lossy(&out_b.concat()).to_string();

    assert_eq!(a_text, "claude:alpha", "流 a 只应包含自己的事件");
    assert_eq!(b_text, "claude:beta", "流 b 只应包含自己的事件");
}

// 6. 编码器确实跨事件持有状态：单流内多个事件在 finish 时汇总可见。
//    防的是「encode_event 各自为政」——Claude/Responses 的 block 边界帧和 tool 碎片
//    装配都依赖这条状态链；若 encoder 无状态，缺口会在 WP3/WP5 才暴露。
#[test]
fn stream_encoder_accumulates_across_events() {
    let (reg, calls) = mk_registry(&[Protocol::OpenAIResp]);
    let codec = reg.resolve(Protocol::OpenAIResp).expect("已注册");

    let mut enc = codec.stream_encoder();

    // 累积期不吐字节是合法行为：断言零帧，锁死这条语义不让它被误改。
    for text in ["one", "two", "three"] {
        let frames = enc
            .encode_event(&StreamEvent::TextDelta {
                index: 0,
                text: text.into(),
            })
            .expect("编码");
        assert!(frames.is_empty(), "累积期不应吐帧，实际: {frames:?}");
    }

    let out = enc.finish().expect("收尾");
    let text = String::from_utf8_lossy(&out.concat()).to_string();
    assert_eq!(
        text, "openai_resp:one|two|three",
        "finish 必须看到全部三个事件（证明跨事件状态在 encoder 内）"
    );

    assert_eq!(
        *calls.lock().unwrap(),
        vec!["openai_resp::stream_encoder"],
        "取 encoder 只应调用 stream_encoder 一次"
    );
}
