# gateway 韧性层路线图 —— api-hub / new-api / sub2api 四方对照后的增强计划

> 依据：api-hub 全文精读（896 行，`~/projects/dotfiles/bin/api-hub`）、new-api 代码图谱调查、
> sub2api 代码调查、生产网关 web 调查（LiteLLM Router / Bifrost / OrcaRouter / Portkey /
> flo2 buffer-before-forward）。本文只覆盖**网关韧性层**（dispatch / forward / gate），
> 出口代理见 `gateway-proxy.md`。工作目录 `.wt/<name>/`。最近更新：2026-09-12。

## 〇、现状基线：ferrite 已经有什么（勿重复建设）

- **健康状态机** `crates/gateway/dispatch/src/health.rs`（423 行）：new-api phase0
  `track_health.go` 完整移植且更强——outcome 五分类（`ChannelOutcome` :37-46）+ EWMA
  （`HealthState.ewma_score`）+ 递增冷却（`cooldown_duration_ms` :401）+ slow-start ramp
  + max-ejection（new-api 实际代码没有后两者）+ 401 升级阈值=3。
- **重试循环** `dispatch/src/retry.rs::run_retry_loop`（:157）+ `Failover` 排除集/预算
  （:83，对标 sub2api `FailoverState`）。`AttemptOutcome` 三态（:32）。
- **选点** `dispatch/src/selector.rs`：priority 分层 + 层内 fallthrough + EWMA×slow-start
  加权随机；affinity 留 V2 接口（:11）。
- **接线** `forward/src/stage.rs::with_retry` 双模式（:30-48），两 app 已接线
  （apps/gateway:52、apps/api:138）。
- **状态码分类** `forward/src/egress.rs::classify_status`（:181-195）：401/403/4xx 不可
  重试、429/5xx 可重试。
- **流式结算** `forward/src/stream.rs::finish`（:71）：断流以"已累计 token + 500"结算
  不丢账单；`AbortGuard`（:113）客户端断开取消。
- 出口代理侧独立健康：`proxy/src/manager.rs::NodeHealth` + 软亲和 + `probe_channel`。

生产共识（web 调查）：缓冲窗口透明重试（flo2）、TTFT 监控防破坏性重启（Bifrost）、
mid-stream failover（OrcaRouter 卖点）、按失败模式分策略（qveris）、共享熔断（LiteLLM
Redis）。**方向是行业演进，不是个人脚本土法**。

---

## P0 — 404 no_route 诊断与钉死（阻塞一切，先做）

**现状**：无头 gateway 冒烟 `POST /v1/chat/completions`（model=mock-gpt、token
group=default、channel `models=["mock-gpt"]`）→ 404 `no_route`，未定位。

**过滤链实读**：`dispatch/src/lib.rs::candidates_from_snapshot`（:60-69）只按
`status==1 && u.group == group && u.public_model == model` 过滤；group 来自
`stage.rs:295` 的 `ctx.token.group`。

**诊断步骤**：
1. `apps/gateway/tests/route_resolution.rs`（新）：起 mock 上游 + #151 冒烟同款配置，
   断言 `POST /v1/chat/completions` 端到端 200（mock 上游零外网，CI 常驻）。
2. 复现 404 时：`Dispatcher::select`（lib.rs:135）加 debug 日志（group/model/候选数），
   定位是 group 提取错、快照空、还是 `gate::model` 未提升 `ctx.requested_model`。
3. 修复点视诊断结果落在 `gate/src/state.rs`（group 提取）或 `gate/src/model.rs`。

**文件**：`apps/gateway/tests/route_resolution.rs`（新）、`dispatch/src/lib.rs`、
`gate/src/{state,model}.rs`（视诊断）。
**验收**：集成测试 200；CI 绿。

---

## P1 — 失败分类升级：差异化冷却 + 4xx 降层 + 双账本打通

**补强理由**：`FailureClass` 只有 `Retryable`/`Fatal` 两态（health.rs:28-33），三个问题：
(a) 401（凭据坏）与 5xx（上游崩）走同一条冷却曲线——key 失效是持续态应长冷却，
5xx 是暂态应短冷却；(b) 4xx 一律 Fatal **不降层**（stage.rs:344 → retry.rs:187 终止），
api-hub/new-api/sub2api 三家都降层——同一模型在 B 渠道可能就是好的；(c) `proxy::manager::NodeHealth`
与 `dispatch::health` 两套账本互不感知——代理节点冷却后 dispatch 照旧选中该 route unit，
forward 在 `client_for` 才发现并回落直连，多绕一跳且"该渠道代理全挂"被直连兜底掩盖
（sub2api 的 account 冷却直接参与选点）。

### P1-A outcome 驱动的差异化冷却

- `dispatch/src/health.rs`
  - `HealthState`（:57-74）加字段 `last_cooling_outcome: Option<ChannelOutcome>`；
    `start_cooldown`（:415）写入。
  - `cooldown_duration_ms`（:401）：签名不变，内部分档——401-run 升级的 Fatal →
    `max` 档（10min）；5xx streak → 现行曲线；429 → `base` 短冷却。
  - `classify`（:344-387）：401/403 从 Neutral 拆出独立分支：**孤立 401 保持不改分**
    （Neutral 语义保留），但计入 `unauthorized_run`（现只 401 计）。
- 验收：health.rs 测试扩 3 例（401-run 冷却 > 5xx 冷却；429 短冷却；Neutral 不进冷却）。

### P1-B 4xx 按渠道相关性降层（api-hub 确定错误降层的健康化版）

- `forward/src/egress.rs::classify_status`（:181）：拆出**渠道相关 4xx**（401/403/404——
  换渠道可能成立）与**请求相关 4xx**（400/413/422——换谁都是失败）；
  `contract::error::NormalizedError` 加 `channel_scoped: bool`。
- `forward/src/stage.rs::handle_with_retry` attempt 闭包（:338-346）：
  `channel_scoped` → 新 variant `AttemptOutcome::FatalButSwitchable`。
- `dispatch/src/retry.rs::run_retry_loop`（:187）match 加臂：FatalButSwitchable →
  `report(Err(Neutral 对应分类))` + `mark_tried` + continue；最后一个候选才透传
  （:364 分支扩展）。
- `dispatch/src/health.rs`：该类失败经 report 走 Neutral，不污染 EWMA（health.rs:44
  语义已就位，无需改 classify）。
- 验收：retry.rs 单测（404 首候选 → 第二候选被选中；400 单候选 → 直接透传）；
  `route_resolution.rs` 扩双渠道降层用例。

### P1-C 双健康账本打通

- `proxy/src/manager.rs`：新增 `pub fn node_cooldowns(&self) -> Vec<(i64, Instant)>`
  （读私有 health 表，与 `node_stats` 同款锁纪律）。
- 桥接点选在 **forward**（❌ 弃"dispatch 轮询 node_cooldowns"：跨 crate 轮询引入时序耦合）：
  `stage.rs::handle_with_retry` 的 attempt 闭包里，`Lease.node_id == 0`（直连回落）且该
  candidate 原本绑定了代理节点时，本次 `NormalizedError` 附加 `degraded: true`；
  循环看到标记 → `report(Err(Retryable))` 强制换候选并把健康记在 route unit 上。
  连续 N 次后该 unit 进冷却，不再反复试死节点。
- 验收：单测——两节点渠道，节点 A 冷却后回落直连 → route unit 健康被记，达阈值进冷却。

**文件**：`health.rs`、`egress.rs`、`stage.rs`、`retry.rs`、`proxy/{manager,pool}.rs`、
两 app 组装、各自 tests。

## P2 — 流式韧性：smart_stream（最大单点落差，行业方向）

**补强理由**：`forward/src/stream.rs` 只有扫描链+结算（183 行），无恢复能力。断流时
客户端拿到 TCP 截断（SillyTavern 直接报错）。api-hub 的 smart_stream 三级设计已被
Bifrost/OrcaRouter/flo2 验证为行业方向；new-api 恰恰没有（历史包袱）。ferrite 的
`SseContext` 扫描链（token 计数已在管道内）是比 api-hub 字节级 hack 更好的挂点。
健康化参数：窗口 2s（非 60s）、重试 max 3（非 100）、心跳 10-15s。

对标：api-hub `_smart_stream`（:618-723）/`_drain_buffered`（:556-615）。

### P2-A 缓冲窗口 + 晚期断流事件（`forward/src/stream.rs`）

- 新增 `pub struct StreamResilience { window_ms, heartbeat_ms, max_retries }`
  （默认 2000/15000/3；apps 从 config `[stream]` 段注入）。
- 新增 `pub enum ResilientChunk` 与
  `pub fn pipe_resilient(ctx, buf: &mut StreamBuf, chunk: Option<&Bytes>, cfg) -> ResilientChunk`：
  - **Window 相**（首字节前）：chunk 累积进 buf 不外发；EOF-without-finish →
    `Retryable`（零字节已发，重试对客户端无感 = api-hub buffer_stream；完整判据
    照抄 `_drain_buffered`：`[DONE]` 或 `finish_reason` 二选一收尾）。
  - **Live 相**：按 32KB/150ms 阈值外发；上游静默超 heartbeat → 发
    `: keepalive\n\n` SSE 注释帧（new-api/sub2api 都有 Ping 保活）；EOF-without-finish →
    `Broken { flushed }`。
- `crates/gateway/forward/src/stage.rs::commit_forwarded`（:419 unfold 流）：
  - Window 相内 `Retryable` → `commit_forwarded` 返回特殊 StageOutcome，由
    `handle_with_retry` 的循环**在流式下也能换候选重试**（现状：attempt 返回 Done 后
    循环已退出，流式断流无法重试——这是缓冲窗口存在的意义）。
  - `Broken` → 刷已缓冲字节 + `event: error` SSE 帧（api-hub `do_error_flush`
    同款，Bifrost「tokens 已发就不破坏性重启」），并经 `stream::finish(500, Some(..))`
    结算已发生 token。
- `apps/{gateway,api}`：config 注入 `[stream]` 段（window/heartbeat/max_retries）。

### P2-B 验收

- `stream.rs` 单测：窗口内断流→Retryable、窗口后静默→keepalive 帧、EOF 无 DONE
  有 finish_reason→补帧、BREAK 后已发字节计数正确。
- 集成测试：mock 上游「流式两帧后断连」→ 非流式 client 拿到完整缓冲回包
  （window 模式）/ 流式 client 拿到 error 帧（live 模式）。

## P3 — 每模型重试策略与预算（api-hub 配置面，ferrite 参数健康化）

**补强理由**：`RetryPolicy` 只有一个全局 `max_attempts`（retry.rs:47-50）。api-hub 的
`[retry."<model>"]`（max/no_retry/retry_all/retry_codes/total_budget/per_req_timeout）
是它的正确配置面——但默认值要健康化（max 3 非 100；budget 120s 非 900s）。

- `crates/gateway/dispatch/src/retry.rs`
  - `RetryPolicy` 扩字段：`total_budget: Duration`（候选链封顶，run_retry_loop 每轮
    检查剩余）、`per_req_timeout: Duration`（透传给 attempt 闭包——需要
    `Attempt` 结构体加字段或闭包签名加 `&AttemptCtx`）。
  - 新增 `pub struct ModelRetryPolicy` 解析：`pub fn policy_for(model: &str) -> &RetryPolicy`
    （默认档 + 覆盖档；api-hub `_retry_policy` 的 merge 语义：default 先、model 后）。
- `crates/gateway/dispatch/src/lib.rs`：`Dispatcher` 加
  `policies: ArcSwap<HashMap<String, RetryPolicy>>`（`set_policies` 对齐 `set_snapshot`
  模式）；`select` 不变，`run_retry_loop` 调用方从 dispatcher 取档。
- `apps/gateway/src/config.rs`：`[retry."<model>"]` TOML 段 → `build_retry_policy`
  返回模型档表；`config.toml.example` 补样例（默认 max=3/budget=120s，注释说明
  「长文/慢模型才显式调高」）。
- 验收：retry.rs 单测（budget 耗尽提前终止；不同 model 命中不同档）。

## P4 — 渠道级 inject / rename（api-hub 个人语义 → 渠道配置）

**补强理由**：api-hub 的 `[inject]`（全局注入 reasoning_effort + 按上游 drop）与
`[rename]`（client 别名→真名）是个人语义；中转站对应物是**渠道级设置**（渠道 A 需要
注入 thinking 参数是渠道属性）。ferrite 已有渠道级 `upstream_models`（rename 的候选级
半边），缺 client 别名层与 body 注入。

- `contract` / `apps/gateway/src/config.rs`：`ChannelConfig` 加
  `inject: Option<HashMap<String, Value>>`（合并进上游 body 的字段）与
  `drop_fields: Vec<String>`（该渠道不支持、发出前摘除）；`KeyConfig` 不动。
- 新增 `crates/gateway/gate/src/rewrite.rs`（或 forward 内 body 改写步）：
  `pub fn rewrite_body(body: &Bytes, inject: &HashMap<String,Value>, drop: &[String]) -> Bytes`
  ——client 显式传的字段优先（api-hub `_inject_body` 语义），drop 最后执行。
- 挂点：`forward/src/stage.rs::build_task`（候选级——inject/drop 是渠道属性，
  随 `SelectedRoute` 走；`SelectedRoute` 需透传这两个字段）。
- **client 别名**：暂不做全局 rename 表——P3 的模型档 + 现有 `upstream_models`
  已覆盖真实需求（omp/ZCode 直接用真名）。有真实别名诉求再进 gate::model。
- 验收：rewrite_body 单测（注入不覆盖 client 显式值、drop 摘除、两者组合）。

## P5 — 粘性会话与多 key 轮询（sub2api / new-api 对标，最后做）

- **粘性会话**（sub2api `BindStickySession` 1h TTL）：selector.rs:11 已标注 V2、
  session_hash 提取规则未定。实现路径：`Dispatch::select` 加可选 `session: &str`
  参数（`gate` 层从 `x-session-id`/user+model 组合提取），候选命中上会话记录
  且健康可选 → 原候选。价值：Claude cache-read 计费。
- **多 key 轮询**（new-api `MultiKeyMode`）：`candidate.rs`/`resolve_candidate` 现在
  `key_index: 0` 恒定（apps/gateway/src/config.rs:321）。实现：轮询模式在
  `resolve_candidate` 按渠道维度轮转 `ChannelKey.index`；key 级失败（401/403）经
  P1-B 降层时**记 key 级健康**（`HealthTable` key 从 unit.key 扩展 `unit.key#idx`），
  单 key 禁用不连坐整渠道。
- 验收：轮询分布单测（rng 注入确定性）；粘性会话单测（同 session 两次 select 同候选，
  候选冷却后让位）。

## 不做清单（含理由）

- **订阅自动刷新**：grok2api-sing 也没有（只有 sharelink 手工导入）——见 gateway-proxy.md。
- **report 前端面板**：维护者 2026-09-12 决定不做。
- **`max=100`/`total_budget=900s` 这类暴力默认**：api-hub 个人特化，中转站用
  QuotaGate + 租户档位替代；显式配置可以调，默认不给。
- **全局 `[inject]` + force 覆盖语义**：个人脚本语义，渠道级（P4）才是中转站形态。
- **分布式熔断（Redis）**：单机不需要；`HealthTable` 已是 trait，多实例时换实现即可。

## 验证总纲

- 单测全部进 `tests/`（gate 约定），mock 上游零外网；clippy/test 交 CI（本地资源不足）。
- 每阶段合并后用 CI 产物（test-latest release）在本地跑一遍 api-hub 同款场景：
  双渠道分层 + 慢上游 + 断流，对比 api-hub 行为。
