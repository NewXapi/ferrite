//! P1-C 双健康账本桥集成测试（proxy::manager 节点账本 → dispatch route unit 账本）。
//!
//! 测的行为：ForwardStage 走生产租约路径（proxies=Some 时注入 egress 被替换，
//! 所以用 std::net 起本地真 HTTP mock，reqwest 直连/经节点都走真协议栈）。
//! 桥接语义经真实 `Dispatcher` + `MemoryHealthTable` 观测（外部可观测状态 =
//! unit 的 failure_streak / cooldown_until_ms / mock 命中数，不测私有标记位）：
//!
//! 1. 渠道绑定的代理节点全部冷却 → 直连回落失败被记成 route unit 的
//!    Retryable 失败 streak；连续达阈值（5）→ unit 进冷却，select 不再
//!    反复选中这个"代理全挂"的渠道（正向）。
//! 2. 渠道本就无绑定节点 → 直连是预期路径，401 失败走 P1-B Neutral，
//!    unit 不记 streak 不进冷却（反向钉：区分"降级兜底"与"设计如此"）。
//! 3. 绑定节点健康、被选中（无直连回落发生）→ 节点上的 401 不触发
//!    降级记账，unit 健康与节点账本互不污染（反向钉：回落不存在时不受影响）。

use bytes::Bytes;
use contract::records::{ChannelKey, ChannelRecord, RouteUnitRecord, SyncMeta};
use dispatch::health::{HealthTable, MemoryHealthTable};
use dispatch::{Dispatcher, RetryPolicy, Snapshot};
use forward::ForwardStage;
use forward::egress::{Egress, ForwardedResponse, Timeouts};
use gateway_pipeline::ctx::{BodySource, ProtocolKind, RequestMeta, StreamedAccum};
use gateway_pipeline::{RequestCtx, Stage, StageOutcome, TokenInfo};
use gateway_proxy::manager::ProxyManager;
use gateway_proxy::node::{ProxyNode, ProxyScheme};
use gateway_proxy::pool::ProxySnapshot;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

// ---------- 本地 mock 上游 ----------

/// 最小 HTTP/1.1 mock：每个请求固定回 `status` + JSON body，`connection: close`。
/// 先吃下头 + content-length 声明的 body 再应答（避免提前关闭触发 RST，
/// 把 401 语义请求劣化成 502 传输错误）。返回 (端口, 命中计数)。
fn spawn_mock(status: u16, body: &'static [u8]) -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&hits);
    std::thread::spawn(move || {
        let head = format!(
            "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n",
            body.len()
        );
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            counter.fetch_add(1, Ordering::Relaxed);
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let t = line.trim().to_ascii_lowercase();
                if t.is_empty() {
                    break;
                }
                if let Some(v) = t.strip_prefix("content-length:") {
                    content_length = v.trim().parse().unwrap_or(0);
                }
            }
            if content_length > 0 {
                let mut discard = vec![0u8; content_length];
                let _ = reader.read_exact(&mut discard);
            }
            let _ = stream.write_all(head.as_bytes());
            let _ = stream.write_all(body);
            let _ = stream.flush();
            let _ = stream.shutdown(std::net::Shutdown::Write);
        }
    });
    (port, hits)
}

/// proxies=Some 时租约 Client 全权替换注入 egress；被调用即接线错误。
struct NeverEgress;

impl Egress for NeverEgress {
    fn execute<'a>(
        &'a self,
        _url: &'a str,
        _headers: &'a [(String, String)],
        _body: Bytes,
        _timeouts: &'a Timeouts,
    ) -> std::pin::Pin<
        Box<
            dyn Future<Output = Result<ForwardedResponse, contract::error::NormalizedError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move { panic!("proxies=Some: injected egress must not be used") })
    }
}

// ---------- 快照 / ctx / stage 装配 ----------

fn http_node(id: i64, channel_key: &str, host: &str, port: u16) -> ProxyNode {
    ProxyNode {
        id,
        scheme: ProxyScheme::Http,
        host: host.to_string(),
        port,
        auth: None,
        opts: None,
        channel_keys: vec![channel_key.to_string()],
        priority: 0,
    }
}

fn unit(key: &str, channel_key: &str, priority: i32) -> RouteUnitRecord {
    RouteUnitRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        group: "g".to_string(),
        public_model: "m".to_string(),
        channel_key: channel_key.to_string(),
        key_index: 0,
        upstream_model: "m".to_string(),
        priority,
        weight: 10,
        status: 1,
    }
}

fn channel(key: &str, base_url: String) -> ChannelRecord {
    ChannelRecord {
        meta: SyncMeta {
            key: key.to_string(),
            schema_version: 1,
            logical_version: 1,
            origin: "test".to_string(),
            updated_at: chrono::Utc::now(),
        },
        name: key.to_string(),
        provider_type: "openai".to_string(),
        base_url,
        keys: vec![ChannelKey {
            index: 0,
            secret: "sk-test".to_string(),
            rpm_limit: 0,
        }],
        max_concurrency: 10,
        status: 1,
        groups: vec!["g".to_string()],
        settings: serde_json::Value::Null,
    }
}

/// u1 → ch-a（401 上游, priority 20 恒被先选），u2 → ch-b（200 上游, 兜底获胜者）。
fn snap(p401: u16, p200: u16) -> Snapshot {
    let mut channels = HashMap::new();
    channels.insert(
        "ch-a".to_string(),
        channel("ch-a", format!("http://127.0.0.1:{p401}")),
    );
    channels.insert(
        "ch-b".to_string(),
        channel("ch-b", format!("http://127.0.0.1:{p200}")),
    );
    Snapshot {
        units: vec![unit("u1", "ch-a", 20), unit("u2", "ch-b", 10)],
        channels,
    }
}

fn mk_ctx() -> RequestCtx {
    RequestCtx {
        request: RequestMeta {
            method: "POST".to_string(),
            path: "/v1/chat/completions".to_string(),
            headers: http::HeaderMap::new(),
            body: BodySource::InMemory(Bytes::from_static(b"{\"model\":\"m\"}")),
            client_ip: "127.0.0.1".parse().unwrap(),
            request_id: uuid::Uuid::now_v7(),
            inbound_protocol: ProtocolKind::OpenAI,
        },
        token: Some(TokenInfo {
            id: "tok-1".into(),
            group: "g".to_string(),
            enabled: true,
            allowed_models: None,
            auth_version: 1,
        }),
        requested_model: Some("m".to_string()),
        route: None,
        selected_channel_key: None,
        selected_channel_name: None,
        upstream: None,
        streamed: StreamedAccum::default(),
        error: None,
    }
}

fn build_stage(mgr: Arc<ProxyManager>, dispatcher: Arc<Dispatcher>) -> ForwardStage {
    let adaptors = Arc::new(gateway_protocol_bridge::adaptor::AdaptorRegistry::new());
    ForwardStage::new(Arc::new(NeverEgress), adaptors)
        .with_proxies(mgr)
        .with_retry(
            dispatcher,
            RetryPolicy {
                max_attempts: 3,
                ..RetryPolicy::default()
            },
        )
}

/// 跑一次请求：u1 先被选、失败后换 u2 成功；断言客户端可见终态 200。
async fn once(stage: &ForwardStage) -> u16 {
    let mut ctx = mk_ctx();
    let outcome = stage.handle(&mut ctx).await.expect("stage ok");
    assert!(
        matches!(outcome, StageOutcome::Continue),
        "非流式走 Continue"
    );
    ctx.upstream.expect("upstream settled").status
}

// ---------- 用例 ----------

/// 正向：两节点渠道的节点 A、B 全部冷却后，请求经直连回落打到 401 上游 →
/// 失败按 Retryable 记到 u1；连续 5 次（默认 cooldown_threshold）→ u1 进
/// 冷却，第 6 次请求 select 不再试它（401 mock 命中数不再增长），u2 独担流量。
#[tokio::test]
async fn cooled_channel_degrades_unit_health_until_eviction() {
    let (p401, hits401) = spawn_mock(401, b"{\"error\":\"bad_key\"}");
    let (p200, _) = spawn_mock(200, b"{\"ok\":true}");

    let mgr = Arc::new(ProxyManager::new());
    mgr.install(ProxySnapshot {
        nodes: vec![
            http_node(1, "ch-a", "n1.example", 8080),
            http_node(2, "ch-a", "n2.example", 8080),
        ],
    });
    // 节点账本判死：两个绑定节点都传输失败冷却（30s，测试时长内不会自然过期）。
    mgr.feedback(1, 502, true);
    mgr.feedback(2, 502, true);
    assert_eq!(mgr.node_cooldowns().len(), 2, "前置：两节点均在冷却");

    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(
        Some(Arc::new(snap(p401, p200))),
        health.clone(),
    ));
    let stage = build_stage(mgr, dispatcher);

    // 前 4 轮：直连回落失败按 Retryable 记在 route unit 上，streak 逐轮递增。
    for round in 1..=4u32 {
        let status = once(&stage).await;
        assert_eq!(status, 200, "第 {round} 轮：u1 降级失败后 u2 必须接住请求");
        let st = health.get("u1");
        assert_eq!(
            st.failure_streak, round,
            "第 {round} 轮：直连回落失败必须按 Retryable 记在 route unit 上"
        );
        assert_eq!(hits401.load(Ordering::Relaxed) as u32, round);
    }

    // 第 5 轮：streak 达阈值（5）→ 冷却激活。phase0 语义里 streak 在激活瞬间
    // 清零（冷却自身即失败证据），外部可观测的终态是 cooldown_until_ms。
    let status = once(&stage).await;
    assert_eq!(status, 200, "第 5 轮：u1 触发冷却，u2 接住请求");
    assert_eq!(hits401.load(Ordering::Relaxed) as u32, 5);
    let now_ms = chrono::Utc::now().timestamp_millis().max(0) as u64;
    let st = health.get("u1");
    assert!(
        st.cooldown_until_ms > now_ms,
        "连续 5 次降级失败后 u1 应处于冷却（达阈值 unit 进冷却）"
    );

    // 不再反复试死节点：u1 被 selector 排除，401 mock 零新增命中。
    let before = hits401.load(Ordering::Relaxed);
    let status = once(&stage).await;
    assert_eq!(status, 200);
    assert_eq!(
        hits401.load(Ordering::Relaxed),
        before,
        "冷却中的 u1 不应再被选中尝试"
    );
}

/// 反向钉①：渠道**本就无绑定节点** → 直连是预期路径而非降级兜底。
/// 同样的 401 失败只走 P1-B Neutral（report Err(Fatal)），6 次连续失败
/// 既不计 streak 也不进冷却——降级记账不得误伤无代理渠道。
#[tokio::test]
async fn unbound_direct_channel_is_not_degraded() {
    let (p401, hits401) = spawn_mock(401, b"{\"error\":\"bad_key\"}");
    let (p200, _) = spawn_mock(200, b"{\"ok\":true}");

    let mgr = Arc::new(ProxyManager::new()); // 空池：ch-a 无绑定
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(
        Some(Arc::new(snap(p401, p200))),
        health.clone(),
    ));
    let stage = build_stage(mgr, dispatcher);

    for round in 1..=6u32 {
        let status = once(&stage).await;
        assert_eq!(status, 200);
        let st = health.get("u1");
        assert_eq!(
            st.failure_streak, 0,
            "第 {round} 轮：无绑定的直连 401 是渠道相关 4xx（Neutral），不记 streak"
        );
        assert_eq!(st.cooldown_until_ms, 0, "更不应触发冷却");
        assert_eq!(hits401.load(Ordering::Relaxed) as u32, round);
    }
}

/// 反向钉②：绑定节点健康且被选中（直连回落**不存在**）→ 经节点出口的 401
/// 不触发降级记账：unit 不记 streak；401 同样不冷却节点（feedback 语义），
/// 两套账本各自干净。
#[tokio::test]
async fn healthy_bound_node_attempt_is_not_degraded() {
    let (p401, hits401) = spawn_mock(401, b"{\"error\":\"bad_key\"}");
    let (p200, _) = spawn_mock(200, b"{\"ok\":true}");

    // 节点 1 = 本地 401 mock 兼职 HTTP 代理（绝对形式请求同样被回 401），健康无冷却。
    let mgr = Arc::new(ProxyManager::new());
    mgr.install(ProxySnapshot {
        nodes: vec![http_node(1, "ch-a", "127.0.0.1", p401)],
    });

    let health = Arc::new(MemoryHealthTable::new());
    let mut snap = snap(p401, p200);
    // ch-a 的 base_url 换成不可直连域名：错误只可能产生于代理节点路径，归因无歧义。
    snap.channels.get_mut("ch-a").unwrap().base_url = "http://via-node-only.invalid".to_string();
    let dispatcher = Arc::new(Dispatcher::new(Some(Arc::new(snap)), health.clone()));
    let stage = build_stage(Arc::clone(&mgr), dispatcher);

    for round in 1..=2u32 {
        let status = once(&stage).await;
        assert_eq!(status, 200, "第 {round} 轮：u1 经节点失败后 u2 兜底");
        let st = health.get("u1");
        assert_eq!(st.failure_streak, 0, "节点被选中≠直连回落，不得降级记账");
        assert_eq!(st.cooldown_until_ms, 0);
        assert_eq!(hits401.load(Ordering::Relaxed) as u32, round);
    }
    assert!(mgr.node_cooldowns().is_empty(), "401 不冷却代理节点");
}
