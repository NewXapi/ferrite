//! `stage` — ForwardStage: 把转发管道接入 pipeline
//!
//! 两种驱动模式：
//! 1. **单次转发**（默认，DispatchStage → ForwardStage 链路）：从 ctx.route
//!    (DispatchStage 写入) 组装 ForwardTask，调 forward_once 发一次上游；
//! 2. **重试循环**（`with_retry` 注入 dispatch + RetryPolicy 后）：handle
//!    忽略 ctx.route，自己驱动 `dispatch::retry::run_retry_loop`
//!    （选候选 → 尝试 → 健康回报 → 排除已试 → 再选），获胜候选的响应经
//!    `commit_forwarded` 写入 ctx.upstream（非流式）或直接回流（流式）。
//!
//! 重试编排与健康回报都在 dispatch::retry 循环内完成；本 stage 只提供
//! 单次尝试闭包（纯 IO）与结果落 ctx。

use crate::ForwardTask;
use crate::egress::ReqwestEgress;
use crate::stream::{self, pipe_chunk};
use async_trait::async_trait;
use bytes::Bytes;
use contract::error::NormalizedError;
use dispatch::retry::{AttemptOutcome, run_retry_loop};
use dispatch::{Dispatch, DispatchError, FailureClass, RetryPolicy};
use gateway_pipeline::ctx::{BodySource, SelectedRoute, UpstreamResponse};
use gateway_pipeline::{PipeStream, RequestCtx, Stage, StageError, StageOutcome};
use gateway_protocol_bridge::format_codec::FormatRegistry;
use gateway_proxy::ProxyManager;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

/// 转发 stage — 依赖 egress（测试 mock / 无代理）或 [`ProxyManager`] 租约 Client。
pub struct ForwardStage {
    egress: Arc<dyn crate::egress::Egress>,
    timeouts: crate::egress::Timeouts,
    /// 单格式协议注册表（`FormatRegistry`）：请求/响应双向转换共用同一份 codec。
    formats: Arc<FormatRegistry>,
    /// 生产路径注入；`None` 时使用 `egress`（测试 mock）。
    proxies: Option<Arc<ProxyManager>>,
    /// `with_retry` 注入的调度器：`Some` 时 handle 驱动完整重试循环并忽略
    /// `ctx.route`；`None` 时保持单次转发语义（读 DispatchStage 写入的 route）。
    dispatch: Option<Arc<dyn Dispatch>>,
    /// 重试预算，仅 `dispatch` 为 `Some` 时生效。
    retry_policy: RetryPolicy,
    /// `with_price_table` 注入的定价表：`Some` 时 commit 点产出结算事件；
    /// `None` = 不计费（保持未接计费时代的行为）。
    price_table: Option<Arc<dyn metering::pricing::PriceTable>>,
    /// 结算产物落地通道（apps 实现，写 usage_logs / 扣内存 quota）；
    /// 与 `price_table` 成对注入。
    sink: Option<Arc<dyn metering::SettleSink>>,
    /// 全局并发闸（v2 挂载）：`with_concurrency` 注入的整体并发上限信号量。
    /// `None` = 不限（行为与未挂载时完全一致）。这是**全局**上限，不分渠道——
    /// 分渠道挂载属后续项，需 per-channel `DashMap<channel_key, Semaphore>`；
    /// gate 的 stage 版本（`gate::concurrency`）即 per-channel 设计，但它为
    /// pipeline 挂载而写，`with_retry` 模式下 `ctx.route` 永不写回，pipeline
    /// 层拿不到选路结果，故并发闸只能挂在 ForwardStage 内部 attempt 路径。
    concurrency: Option<Arc<Semaphore>>,
}

/// meow `ProxyAdapter` → [`StreamDialer`] 适配。
///
/// `dial(host, port)` 语义对齐：meow 的 `Metadata.host/dst_port` 就是拨号目标，
/// 协议握手在 `dial_tcp` 内完成，返回流直接承载上层流量。
struct AdapterDialer(Arc<dyn gateway_proxy::ProxyAdapter>);

impl std::fmt::Debug for AdapterDialer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdapterDialer").finish_non_exhaustive()
    }
}

#[async_trait::async_trait]
impl crate::adapter_egress::StreamDialer for AdapterDialer {
    async fn dial(
        &self,
        host: &str,
        port: u16,
    ) -> std::io::Result<Box<dyn crate::adapter_egress::DialedStream>> {
        let meta = gateway_proxy::tcp_metadata(host, port);
        let conn = self
            .0
            .dial_tcp(&meta)
            .await
            .map_err(std::io::Error::other)?;
        Ok(Box::new(MeowConn(conn)))
    }
}

/// `Box<dyn ProxyConn>` → `DialedStream`：两个 trait bound 都是
/// AsyncRead + AsyncWrite + Unpin + Send + Sync，透传即可。
struct MeowConn(Box<dyn gateway_proxy::ProxyConn>);

impl tokio::io::AsyncRead for MeowConn {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl tokio::io::AsyncWrite for MeowConn {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

/// 单次尝试失败的内部载体：[`NormalizedError`] + P1-C 降级标记。
///
/// 不进 contract：degraded 是 forward 租约与节点账本的局部事实，只有
/// `handle_with_retry` 的尝试闭包消费，客户端侧不可见。
#[derive(Debug)]
struct AttemptError {
    error: NormalizedError,
    /// 本次尝试的租约回落直连（`Lease.node_id == 0`），且该渠道绑定的代理节点
    /// 全部处于节点冷却——直连是降级兜底而非预期路径（P1-C 双账本桥接）。
    degraded: bool,
}

impl ForwardStage {
    /// 使用注入的 [`crate::egress::Egress`]（测试或无代理池）。
    pub fn new(egress: Arc<dyn crate::egress::Egress>, formats: Arc<FormatRegistry>) -> Self {
        Self {
            egress,
            timeouts: crate::egress::Timeouts::default(),
            formats,
            proxies: None,
            dispatch: None,
            retry_policy: RetryPolicy::default(),
            price_table: None,
            sink: None,
            concurrency: None,
        }
    }

    /// 模型请求按 `SelectedRoute.channel_key` 租出口 Client。
    pub fn with_proxies(mut self, proxies: Arc<ProxyManager>) -> Self {
        self.proxies = Some(proxies);
        self
    }

    /// 启用 failover 重试循环：handle 不再读 `ctx.route`（可与 DispatchStage
    /// 共存，但不依赖它），改为自己驱动 `dispatch::retry::run_retry_loop` —
    /// 每次尝试后向 `dispatch` 做健康回报，可重试失败换候选再来一轮。
    pub fn with_retry(mut self, dispatch: Arc<dyn Dispatch>, policy: RetryPolicy) -> Self {
        self.dispatch = Some(dispatch);
        self.retry_policy = policy;
        self
    }

    /// 挂全局并发闸：整个 ForwardStage 共享一个 `Semaphore(max)`（MVP：整体
    /// 并发上限，不分渠道；分渠道挂载属后续项，需 per-channel DashMap）。
    ///
    /// acquire 点在每条转发路径的 `forward_task` 之前，`try` 语义不排队：
    /// 重试模式在 attempt 闭包内（permit 随 attempt 结束释放），单次模式在
    /// `handle` 内（permit 持到 `commit_forwarded` 完成）。槽满立即以
    /// `rate_limited`(429, retryable, degraded) 拒绝本 attempt——健康表不记
    /// 真实失败、不落 `submit_failed` 观测事件。
    ///
    /// 注意：`max = 0` 的信号量会拒绝一切请求；装配侧约定 0 = 不限并发，
    /// 即不调用本 builder（`concurrency` 保持 `None`）。
    pub fn with_concurrency(mut self, max: usize) -> Self {
        self.concurrency = Some(Arc::new(Semaphore::new(max)));
        self
    }

    /// 挂上库层结算通道：`pt` 算价、`sink` 落地 [`contract::records::UsageEventRecord`]。
    ///
    /// 两字段成对注入（同一 `commit_forwarded` 结算点消费），`Some` 时流式与
    /// 非流式响应在提交点产出结算事件；不调用本 builder 则保持不计费行为。
    /// 权威扣费仍归 apps（usage.rs），本通道只产出事件——是否同时扣本地余额
    /// 由 sink 实现方裁决，避免双扣。
    pub fn with_price_table(
        mut self,
        pt: Arc<dyn metering::pricing::PriceTable>,
        sink: Arc<dyn metering::SettleSink>,
    ) -> Self {
        self.price_table = Some(pt);
        self.sink = Some(sink);
        self
    }

    /// 按候选 + 已读请求体组装一次尝试的 [`ForwardTask`]。
    ///
    /// 字段组装逻辑与原单次路径一致：headers 留空（gate 已清洗，透传头由
    /// apps/api 组装），provider_type/extra_headers 从候选自带的完整解析取。
    fn build_task(
        candidate: &SelectedRoute,
        path: String,
        body: Bytes,
        stream: bool,
        inbound_format: gateway_pipeline::ctx::ProtocolKind,
    ) -> ForwardTask {
        ForwardTask {
            candidate: candidate.clone(),
            path,
            headers: vec![],
            body,
            stream,
            provider_type: candidate.provider_type.clone(),
            extra_headers: crate::adapter::extra_headers_from_settings(&candidate.settings),
            // 客户端说的协议：请求方向据此 decode、响应方向据此 encode。
            inbound_format,
        }
    }

    async fn forward_task(&self, task: &ForwardTask) -> Result<crate::Forwarded, AttemptError> {
        let Some(proxies) = self.proxies.as_ref() else {
            return crate::pipeline::forward_once(
                task,
                &*self.egress,
                &self.formats,
                &self.timeouts,
            )
            .await
            .map_err(|error| AttemptError {
                error,
                degraded: false,
            });
        };

        let lease = proxies.acquire(&task.candidate.unit.channel_key);
        let node_id = lease.node_id;
        // 出口二态：reqwest Client（direct/http/socks5）或协议适配器
        // （SS/Trojan/VLESS/VMess——meow dial_tcp 经 adapter_egress 桥成 HTTP）。
        let result = if let Some(adapter) = lease.adapter() {
            let egress = crate::adapter_egress::AdapterEgress::new(
                Arc::new(AdapterDialer(adapter)),
                Duration::from_secs(5),
            );
            crate::pipeline::forward_once(task, &egress, &self.formats, &self.timeouts).await
        } else if let Some(client) = lease.reqwest_client() {
            let egress = ReqwestEgress::with_client((*client).clone(), Duration::from_secs(5));
            crate::pipeline::forward_once(task, &egress, &self.formats, &self.timeouts).await
        } else {
            proxies.feedback(node_id, 502, false);
            return Err(AttemptError {
                error: contract::error::NormalizedError {
                    code: contract::error::code::UPSTREAM_ERROR,
                    status: 502,
                    retryable: true,
                    channel_scoped: false,
                    message: "lease produced neither reqwest client nor adapter".into(),
                },
                // 选出了节点却产不出出口 = 节点账本无责（配置/装配错误），不记降级。
                degraded: false,
            });
        };
        match &result {
            Ok(forwarded) => proxies.feedback(node_id, forwarded.status, false),
            Err(err) => {
                let transport_err = err.status == 502 || err.status == 504;
                proxies.feedback(node_id, err.status, transport_err);
            }
        }
        // P1-C：仅失败路径读两份账本快照（成功直连不记——见规格 P1-C 的挂点）。
        result.map_err(|error| AttemptError {
            degraded: node_id == 0 && self.channel_proxy_degraded(&task.candidate.unit.channel_key),
            error,
        })
    }

    /// 渠道绑定的代理节点是否全部处于节点冷却（且非「本就无绑定」）。
    ///
    /// 全冷却 ⇒ [`ProxyManager::acquire`] 必然回落直连（`node_id == 0`），
    /// 此时该渠道的「直连成功/失败」都在掩盖代理账本已判死的事实；调用方
    /// 据此把失败记到 route unit 健康上（见 `handle_with_retry` 尝试闭包）。
    fn channel_proxy_degraded(&self, channel_key: &str) -> bool {
        let Some(proxies) = self.proxies.as_ref() else {
            return false;
        };
        let cooled: std::collections::HashSet<i64> = proxies
            .node_cooldowns()
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        let bound = proxies.channel_node_ids(channel_key);
        !bound.is_empty() && bound.iter().all(|id| cooled.contains(id))
    }
}

#[async_trait]
impl Stage for ForwardStage {
    fn name(&self) -> &'static str {
        "forward"
    }

    async fn handle(&self, ctx: &mut RequestCtx) -> Result<StageOutcome, StageError> {
        // 读 body (可重放: InMemory/OnDisk 都给出新 reader)。
        let body = match &ctx.request.body {
            BodySource::InMemory(b) => b.clone(),
            BodySource::OnDisk { path, len } => {
                let mut f = tokio::fs::File::open(path)
                    .await
                    .map_err(|e| StageError::Internal(anyhow::anyhow!("open body: {e}")))?;
                use tokio::io::AsyncReadExt;
                let mut buf = vec![0u8; *len as usize];
                f.read_exact(&mut buf)
                    .await
                    .map_err(|e| StageError::Internal(anyhow::anyhow!("read body: {e}")))?;
                Bytes::from(buf)
            }
        };
        // 流式由请求体的 `stream` 字段决定 —— 路径里的 "stream" 子串不是协议信号。
        let stream = body_wants_stream(&body);

        // with_retry 接线后自己驱动整段重试循环, 完全忽略 ctx.route。
        if let Some(dispatch) = self.dispatch.clone() {
            return self.handle_with_retry(ctx, dispatch, body, stream).await;
        }

        // 单次模式: 候选由 DispatchStage 写入, 已解析完整 (secret / upstream_model /
        // provider_type / settings), forward 不查快照也不自造。
        let candidate = match &ctx.route {
            Some(r) => r.clone(),
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "dispatch stage did not run before forward"
                )));
            }
        };
        let task = Self::build_task(
            &candidate,
            ctx.request.path.clone(),
            body,
            stream,
            ctx.request.inbound_protocol,
        );

        // 失败观测事件的计时起点（单次模式）：含并发闸等待与上游请求，
        // 成功路径的 duration_ms 由响应侧统计，这里只补失败路径。
        let started = std::time::Instant::now();
        // 并发闸（v2 挂载）：forward_task 之前 try_acquire，permit 随本函数
        // 作用域存活——成功路径持到 commit_forwarded 完成（覆盖响应体读取/
        // 结算窗口），失败提前 return 时立即 drop。槽满：不 submit_failed
        // （并发拒绝不是上游错误，无 status_code 可报），直接按 429 语义短路。
        let _permit = match try_acquire_concurrency(&self.concurrency) {
            Ok(permit) => permit,
            Err(e) => return Err(normalized_to_stage_error(e)),
        };

        let forwarded = match self.forward_task(&task).await {
            Ok(f) => f,
            Err(a) => {
                // 上游已应答/传输失败且有候选：先落一条零成本观测事件再短路。
                // （P1-C：AttemptError 载体带 degraded 标记，单次模式不进重试
                // 闭包，只消费其内层 NormalizedError。）
                self.submit_failed(ctx, &candidate, &a.error, stream, started.elapsed());
                return Err(normalized_to_stage_error(a.error));
            }
        };

        self.commit_forwarded(ctx, forwarded, &task.candidate, task.stream, &task.body)
            .await
    }
}

impl ForwardStage {
    /// 重试模式主循环：调 `dispatch::retry::run_retry_loop` 驱动
    /// 「选候选 → 单次尝试 → 健康回报 → 排除已试 → 再选」。
    ///
    /// 尝试闭包是纯 IO：成功把 `Forwarded` 存进 `result_slot`（循环结束后由
    /// 本函数取出提交），失败把 `NormalizedError` 暂存进 `error_slot` 并按
    /// `retryable` 返回 Retryable/Fatal。回报闭包把同一结果转给
    /// `Dispatch::report`，驱动 dispatch 侧健康状态机。
    async fn handle_with_retry(
        &self,
        ctx: &mut RequestCtx,
        dispatch: Arc<dyn Dispatch>,
        body: Bytes,
        stream: bool,
    ) -> Result<StageOutcome, StageError> {
        // 失败观测事件的计时起点（重试模式）：从进入重试循环起累计，
        // 每条失败事件的 duration_ms = "进入重试 → 该次失败" 的墙钟毫秒
        // （含此前尝试与选路耗时）。
        let started = std::time::Instant::now();
        let group = ctx
            .token
            .as_ref()
            .map(|t| t.group.clone())
            .unwrap_or_default();
        // 模型名由 gate::model 提升到 ctx; 未解析出来无法选候选, 属装配错误。
        let model = match ctx.requested_model.clone() {
            Some(m) => m,
            None => {
                return Err(StageError::Internal(anyhow::anyhow!(
                    "model not resolved before forward"
                )));
            }
        };
        let path = ctx.request.path.clone();
        // 入站协议在闭包外先取出：闭包 move 走 `path` 后 ctx 不可再借。
        let inbound_format = ctx.request.inbound_protocol;

        // 获胜尝试的 Forwarded 与最近一次失败, 短临界区 std Mutex (不跨 await 持锁)。
        let result_slot: Arc<Mutex<Option<crate::Forwarded>>> = Arc::new(Mutex::new(None));
        let error_slot: Arc<Mutex<Option<NormalizedError>>> = Arc::new(Mutex::new(None));
        // 失败观测需要 ctx 的归因字段：循环期间共享借用 ctx（循环 future
        // 结束即释放），之后 commit_forwarded 才能继续用 &mut ctx。
        let ctx_ref: &RequestCtx = ctx;

        let dsel = Arc::clone(&dispatch);
        let drep = Arc::clone(&dispatch);
        let slot = Arc::clone(&result_slot);
        let eslot = Arc::clone(&error_slot);
        // 尝试闭包 move 捕获 body；克隆一份留给循环结束后的 commit（结算要用）。
        let commit_body = body.clone();

        let loop_result = run_retry_loop(
            &group,
            &model,
            &self.retry_policy,
            move |g, m, exclude| dsel.select(g, m, exclude),
            move |candidate| {
                let task = Self::build_task(
                    candidate,
                    path.clone(),
                    body.clone(),
                    stream,
                    inbound_format,
                );
                let slot = Arc::clone(&slot);
                let eslot = Arc::clone(&eslot);
                async move {
                    // 并发闸（v2 挂载）：forward_task 之前 try_acquire；permit
                    // 留在闭包局部变量，本 attempt 结束（Ok 或 Err）即随作用域
                    // drop 释放槽位——**不进 ctx.drop_guards**（attempt 结束就
                    // 该释放，不是整个响应结束）。槽满：并发拒绝不是上游错误，
                    // 不 submit_failed；健康按 degraded 语义走 Retryable 臂
                    // （同 P1-C：驱动换候选、健康表不记真实上游失败）；错误
                    // 暂存 eslot，预算耗尽时透传 429。
                    let _permit = match try_acquire_concurrency(&self.concurrency) {
                        Ok(permit) => permit,
                        Err(e) => {
                            let retryable = e.retryable;
                            *lock(&eslot) = Some(e);
                            if retryable {
                                return AttemptOutcome::Retryable(FailureClass::Retryable);
                            }
                            // 理论不可达臂（信号量 Closed）：按 Fatal 终止。
                            return AttemptOutcome::Fatal(FailureClass::Fatal);
                        }
                    };
                    match self.forward_task(&task).await {
                        Ok(forwarded) => {
                            let status = forwarded.status;
                            *lock(&slot) = Some(forwarded);
                            AttemptOutcome::Done { status }
                        }
                        Err(AttemptError { error: e, degraded }) => {
                            let retryable = e.retryable;
                            let switchable = e.channel_scoped;
                            // 每次失败尝试各落一条零成本观测事件：预算耗尽 /
                            // Fatal 终止时，排障侧也要有"请求发生过"的痕迹。
                            // 必须在 e 移入 error_slot 之前调用。
                            self.submit_failed(
                                ctx_ref,
                                &task.candidate,
                                &e,
                                stream,
                                started.elapsed(),
                            );
                            *lock(&eslot) = Some(e);
                            if retryable || degraded {
                                // P1-C 双账本桥: degraded = 渠道绑定的代理节点全在
                                // 节点冷却、本次实为直连兜底失败。健康按 Retryable 记到
                                // route unit(驱动失败 streak,连续 N 次后 unit 进冷却,
                                // select 不再反复选中"代理全挂"的渠道),并强制换候选。
                                AttemptOutcome::Retryable(FailureClass::Retryable)
                            } else if switchable {
                                // 渠道相关 4xx (P1-B 降层): 健康载荷走 Fatal ——
                                // health::classify(Err(Fatal)) 落 Neutral, 不改分不记 streak。
                                AttemptOutcome::FatalButSwitchable(FailureClass::Fatal)
                            } else {
                                AttemptOutcome::Fatal(FailureClass::Fatal)
                            }
                        }
                    }
                }
            },
            move |key, outcome| drep.report(key, outcome),
        )
        .await;

        match loop_result {
            Ok((attempt, outcome)) => {
                // 归因回写：retry 可能换候选，ctx 还带着 DispatchStage 的初选；
                // Pipeline::run 据此打包 RouteAttribution，不回写会把 usage 记到
                // 失败的初选渠道上。名字回查快照，查不到降级 key-only（与
                // DispatchStage 同款纪律）。
                let channel_key = attempt.candidate.unit.channel_key.clone();
                let channel_name = dispatch.channel_name(&channel_key);
                if channel_name.is_none() {
                    tracing::debug!(
                        channel_key = %channel_key,
                        "channel name not found in dispatch snapshot; retry attribution carries key only"
                    );
                }
                ctx.route = Some(attempt.candidate.clone());
                ctx.selected_channel_key = Some(channel_key);
                ctx.selected_channel_name = channel_name;

                let forwarded = lock(&result_slot).take();
                match (forwarded, outcome) {
                    (Some(f), _) => {
                        self.commit_forwarded(ctx, f, &attempt.candidate, stream, &commit_body)
                            .await
                    }
                    // Fatal (请求相关问题) 与 FatalButSwitchable (最后一个候选,
                    // 已无可换渠道——retry 循环把它作为终态带回) 都透传暂存的
                    // NormalizedError 上游状态码, 没有成功响应可提交。
                    (None, AttemptOutcome::Fatal(_) | AttemptOutcome::FatalButSwitchable(_)) => {
                        let e = lock(&error_slot).take().ok_or_else(|| {
                            StageError::Internal(anyhow::anyhow!("forward produced no result"))
                        })?;
                        Err(normalized_to_stage_error(e))
                    }
                    (None, _) => Err(StageError::Internal(anyhow::anyhow!(
                        "retry loop finished without a forwarded response"
                    ))),
                }
            }
            // 预算耗尽 / 无候选 / 快照未就绪 / 全限流的映射 (E1, 与 DispatchStage 的
            // 单次映射区分: 这里 RetriesExhausted → 502, RateLimited → 429)。
            Err(e) => Err(match e {
                DispatchError::RateLimited { .. } => StageError::RateLimited,
                DispatchError::RetriesExhausted { .. } => {
                    // 带上最后一个上游错误诊断 (ocr: 静态串丢失真实状态码, 排障困难)。
                    StageError::Upstream(match lock(&error_slot).take() {
                        Some(last) => gateway_pipeline::UpstreamError::Status {
                            code: last.status,
                            body_preview: format!(
                                "retry budget exhausted; last upstream: {}",
                                last.message
                            )
                            .into_bytes(),
                        },
                        None => gateway_pipeline::UpstreamError::Status {
                            code: 502,
                            body_preview: b"retry budget exhausted".to_vec(),
                        },
                    })
                }
                DispatchError::NoCandidate { .. } => StageError::NoRoute,
                DispatchError::SnapshotNotReady => StageError::NotReady,
            }),
        }
    }

    /// 提交一次成功尝试的上游响应。
    ///
    /// 非流式 → 收全 body 写入 `ctx.upstream`；流式 → 经 SseScanner +
    /// StreamScanner 扫描链 unfold 成 [`StageOutcome::Stream`] 直接回流。
    /// `candidate` 是获胜候选（流式上下文要它的 channel_key/upstream_model）。
    ///
    /// `with_price_table` 挂载了定价表时，两条路径都在本函数（唯一提交点）
    /// 产出 UsageEvent 交给 sink：流式在流读完 / 中途出错时经
    /// [`stream::finish`] 结算；非流式收全 body 后走 [`settle_non_stream`]。
    async fn commit_forwarded(
        &self,
        ctx: &mut RequestCtx,
        forwarded: crate::Forwarded,
        candidate: &SelectedRoute,
        stream: bool,
        req_body: &Bytes,
    ) -> Result<StageOutcome, StageError> {
        if stream {
            // 流式路径经 SseScanner + StreamScanner 扫描链，流结束时自动结算。
            // user_key 必须取 AuthGate 写入的用户 UUID（ctx.user_key）——曾误取
            // token.id：settle 按用户键扣 user_balances，token 键查无行 → 钱包
            // 永远扣不到、usage_logs.user_key 也错（e2e 实锤）。
            let user_key = ctx
                .token
                .as_ref()
                .map(|t| t.user_key.clone())
                .unwrap_or_default();
            let token_key = ctx
                .token
                .as_ref()
                .map(|t| t.id.to_string())
                .unwrap_or_default();
            let channel_key = candidate.unit.channel_key.clone();
            let public_model = candidate.unit.public_model.clone();
            let upstream_model = candidate.upstream_model.clone();
            let group = ctx
                .token
                .as_ref()
                .map(|t| t.group.clone())
                .unwrap_or_default();

            let mut sse_ctx = stream::SseContext::new();
            sse_ctx.user_key = user_key;
            sse_ctx.token_key = token_key;
            sse_ctx.channel_key = channel_key;
            sse_ctx.public_model = public_model;
            sse_ctx.upstream_model = upstream_model;
            sse_ctx.group = group;
            // group ratio 由 apps 注入，库层默认 1.0
            sse_ctx.group_ratio = 1.0;
            // 定价表未挂载（None）时 finish 不结算，行为与不计费时代一致。
            sse_ctx.price_table = self.price_table.clone();

            // 上游的 content-type 原样回给客户端: SSE 客户端靠它判定按事件流读。
            let content_type = forwarded.content_type.clone();
            let sink = self.sink.clone();
            // 扫描上下文用 Option 承载：流读完或中途出错时 take() 出且仅出
            // 一次结算，防止错误项之后重入重复计费。
            let mapped = futures_util::stream::unfold(
                (forwarded.body, Some(sse_ctx)),
                move |(mut s, mut ctx_slot)| {
                    let sink = sink.clone();
                    async move {
                        use futures_util::StreamExt;
                        match s.next().await {
                            Some(Ok(chunk)) => {
                                let out = match ctx_slot.as_mut() {
                                    Some(c) => pipe_chunk(c, &chunk).passthrough,
                                    // 已结算丢弃扫描上下文：字节继续原样透传
                                    None => chunk,
                                };
                                Some((Ok::<Bytes, std::io::Error>(out), (s, ctx_slot)))
                            }
                            Some(Err(e)) => {
                                // 上游断流：用已累积计数结算（status=500），
                                // 否则流中途失败会泄漏已产生的 token 账单；
                                // 错误项照旧回给客户端。
                                if let Some(c) = ctx_slot.take() {
                                    let (_end, _counts, event) =
                                        stream::finish(c, 500, Some(&e.to_string()));
                                    submit_event(&sink, event);
                                }
                                Some((Err(e), (s, ctx_slot)))
                            }
                            None => {
                                if let Some(c) = ctx_slot.take() {
                                    let (_end, _counts, event) = stream::finish(c, 200, None);
                                    submit_event(&sink, event);
                                }
                                None
                            }
                        }
                    }
                },
            );
            let stream: futures_util::stream::BoxStream<'static, Result<Bytes, std::io::Error>> =
                Box::pin(mapped);
            Ok(StageOutcome::Stream(PipeStream::with_content_type(
                axum::body::Body::from_stream(stream),
                content_type,
            )))
        } else {
            // 读取全部 body (非流式响应体通常较小)。
            use futures_util::TryStreamExt;
            let mut buf = Vec::new();
            let mut body_stream = forwarded.body;
            while let Some(chunk) = body_stream
                .try_next()
                .await
                .map_err(|e| StageError::Internal(anyhow::anyhow!("read upstream body: {e}")))?
            {
                buf.extend_from_slice(&chunk);
            }
            // 非流式结算点（写入 ctx.upstream 之前；未挂 pt 时直接跳过）。
            self.settle_non_stream(ctx, req_body, &buf, forwarded.status, candidate);
            ctx.upstream = Some(UpstreamResponse {
                status: forwarded.status,
                body: Bytes::from(buf),
            });
            Ok(StageOutcome::Continue)
        }
    }

    /// 非流式结算 — 从完整响应体提取 usage（缺 usage 则估算兜底），算价后
    /// 把 UsageEvent 交给 sink。
    ///
    /// 定价表或 sink 未挂载（`with_price_table` 未调）时直接返回，保持
    /// 现有不计费行为。估算口径：prompt 用请求体的 JSON 结构感知估算
    /// (`metering::estimate::estimate_prompt_tokens`)，completion 按响应体
    /// 字节数 / 4 粗估（与 `StreamScanner` 无 usage 兜底同量级）。
    fn settle_non_stream(
        &self,
        ctx: &RequestCtx,
        req_body: &Bytes,
        resp_body: &[u8],
        status: u16,
        candidate: &SelectedRoute,
    ) {
        let (Some(pt), Some(sink)) = (self.price_table.as_ref(), self.sink.as_ref()) else {
            return;
        };
        let counts =
            metering::extract_usage(resp_body).unwrap_or_else(|| metering::scanner::TokenCounts {
                prompt: metering::estimate::estimate_prompt_tokens(req_body),
                completion: resp_body.len() as u64 / 4,
                cached: 0,
            });
        // user_key = AuthGate 写入的用户 UUID（ctx.user_key）；token_key 才是
        // token.id。两者曾同为 token.id → settle 按错误键扣钱包（e2e 实锤）。
        let (user_key, token_key, group) = match ctx.token.as_ref() {
            Some(t) => (t.user_key.clone(), t.id.clone(), t.group.clone()),
            // 归因缺失（理论上 gates 已保证 Some）：跳过结算，绝不产出无主账单。
            None => return,
        };
        // group ratio 由 apps 注入，库层默认 1.0
        let group_ratio = 1.0;
        // 库层不做预扣：amount=0 表"事后结算非预扣"（同 stream::finish 的说明）。
        let hold = metering::ledger::Hold {
            id: 0,
            amount: 0,
            user_key,
            token_key,
        };
        let event = metering::settle_event(
            counts,
            // 本函数只在非流式提交点调用（流式经 stream::finish 结算）。
            false,
            &group,
            group_ratio,
            &hold,
            pt.as_ref(),
            &candidate.unit.channel_key,
            &candidate.unit.meta.key,
            &candidate.unit.public_model,
            &candidate.upstream_model,
            0,
            0,
            status,
            None,
        );
        sink.submit(event);
    }

    /// 失败结算 — 上游已应答（4xx/5xx）或传输失败但**有候选**时，产出一条
    /// **零成本观测事件**交给 sink，让"这次请求发生过、被哪个候选拒了"在
    /// usage_logs 留下痕迹。
    ///
    /// `elapsed` 是失败事件的耗时量纲：调用方计时起点到本调用的墙钟时长
    /// （单次模式 = `forward_task` 调用前起算；重试模式 = 进入
    /// `handle_with_retry` 起累计），内部转毫秒填事件的 `duration_ms`，
    /// 超过 u32 毫秒上限（约 49.7 天）时饱和截断。
    ///
    /// 纯观测不参与扣费：counts 全 0 → cost 必为 0；status/error 取自
    /// [`NormalizedError`]；`is_stream` 记录请求意图（与成功路径同源）。
    /// 归因键（user/token/group）与 `settle_non_stream` 一致，均来自
    /// `ctx.token`。pt/sink 未挂载或归因缺失时静默跳过——绝不产出无主账单，
    /// 也不改变调用方照常返回 [`StageError`] 的控制流。
    fn submit_failed(
        &self,
        ctx: &RequestCtx,
        candidate: &SelectedRoute,
        err: &NormalizedError,
        stream: bool,
        elapsed: Duration,
    ) {
        let (Some(pt), Some(sink)) = (self.price_table.as_ref(), self.sink.as_ref()) else {
            return;
        };
        // 同 settle_non_stream：user_key 是用户 UUID，不是 token.id。
        let (user_key, token_key, group) = match ctx.token.as_ref() {
            Some(t) => (t.user_key.clone(), t.id.clone(), t.group.clone()),
            // 归因缺失（理论上 gates 已保证 Some）：跳过观测，同成功路径语义。
            None => return,
        };
        // 库层不做预扣：amount=0 表"事后结算非预扣"（同 settle_non_stream）。
        let hold = metering::ledger::Hold {
            id: 0,
            amount: 0,
            user_key,
            token_key,
        };
        // counts 全 0 → price_of 结果为 0，观测事件不产生账单。
        // 失败耗时 → duration_ms（u32 毫秒，饱和截断）。
        let duration_ms = u32::try_from(elapsed.as_millis()).unwrap_or(u32::MAX);
        let event = metering::settle_event(
            metering::scanner::TokenCounts::default(),
            stream,
            &group,
            // group ratio 由 apps 注入，库层默认 1.0（同 settle_non_stream）。
            1.0,
            &hold,
            pt.as_ref(),
            &candidate.unit.channel_key,
            &candidate.unit.meta.key,
            &candidate.unit.public_model,
            &candidate.upstream_model,
            0, // first_token_ms：失败观测无首字计时
            duration_ms,
            err.status,
            Some(&err.message),
        );
        sink.submit(event);
    }
}

/// 把结算产物交给 sink（sink 未挂载或无事件时静默跳过）。
fn submit_event(
    sink: &Option<Arc<dyn metering::SettleSink>>,
    event: Option<contract::records::UsageEventRecord>,
) {
    if let (Some(event), Some(sink)) = (event, sink.as_ref()) {
        sink.submit(event);
    }
}

/// 取 Mutex 值； poisoning 只可能来自 panic 的传播路径，恢复内部值即可
/// （两个 slot 都是纯赋值，不存在逻辑不一致状态）。
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// NormalizedError → StageError::Upstream（透传上游状态码与消息预览）。
fn normalized_to_stage_error(e: NormalizedError) -> StageError {
    StageError::Upstream(gateway_pipeline::UpstreamError::Status {
        code: e.status,
        body_preview: e.message.into_bytes(),
    })
}

/// 并发闸 acquire（v2 挂载点）：`concurrency` 未挂载 → `Ok(None)`，行为不变。
///
/// 槽满（`NoPermits`）→ `Err(NormalizedError)`：`rate_limited`/429/`retryable`
/// /非渠道相关——调用方据此走 degraded 语义（同 P1-C：健康表不记真实失败），
/// 且**不调 `submit_failed`**（并发拒绝不是上游错误，没有 status_code 可报）。
/// permit 是 owned（持有 `Arc<Semaphore>`），可安全跨 await 持有、随作用域
/// drop 归还槽位。
fn try_acquire_concurrency(
    concurrency: &Option<Arc<Semaphore>>,
) -> Result<Option<OwnedSemaphorePermit>, NormalizedError> {
    let Some(sem) = concurrency.as_ref() else {
        return Ok(None);
    };
    match sem.clone().try_acquire_owned() {
        Ok(permit) => Ok(Some(permit)),
        Err(TryAcquireError::NoPermits) => Err(NormalizedError {
            code: contract::error::code::RATE_LIMITED,
            status: 429,
            retryable: true,
            channel_scoped: false,
            message: "concurrency limit reached".into(),
        }),
        // 本模块从不 close() 信号量，Closed 理论不可达；兜底按内部错误终止。
        Err(e) => Err(NormalizedError {
            code: contract::error::code::INTERNAL,
            status: 500,
            retryable: false,
            channel_scoped: false,
            message: format!("semaphore closed: {e}"),
        }),
    }
}

/// 请求体是否要求流式响应 —— 读顶层 `stream` 布尔字段。
///
/// OpenAI / Claude / Gemini(OpenAI 兼容层) 都用这个字段表达流式意图。
/// 非 JSON 或缺字段 → false（按非流式处理，整体收 body 再回）。
///
/// 不能用 URL 路径判断：`/v1/chat/completions` 带 `"stream": true` 是流式，
/// 而路径里出现 "stream" 子串并不代表客户端要 SSE。
fn body_wants_stream(body: &Bytes) -> bool {
    #[derive(serde::Deserialize)]
    struct StreamFlag {
        #[serde(default)]
        stream: bool,
    }
    serde_json::from_slice::<StreamFlag>(body)
        .map(|f| f.stream)
        .unwrap_or(false)
}
