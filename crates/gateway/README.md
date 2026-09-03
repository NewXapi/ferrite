# `crates/gateway`

## 功能 crate

- `pipeline/` — 请求上下文、Stage 接口、链执行器和 HTTP 路由。
- `gate/` — API Key、状态、配额、频率、模型和并发准入。
- `dispatch/` — 候选渠道、健康状态、权重选择和失败重试。
- `forward/` — 上游 URL、头、请求体、响应体和 SSE 流。
- `proxy/` — 直连、HTTP CONNECT、SOCKS5 和 SSRF 校验。
- `protocol/` — OpenAI、Claude、Gemini 请求、响应、错误和 SSE 编解码。
- `protocol-bridge/` — 将 pipeline 上下文转换为 protocol codec 输入输出。
- `metering/` — 预扣额度、token 获取与估算、价格和结算。
- `security/` — 词库、输入替换、跨 chunk 扫描和审核结果。

## MVP：pipeline

先修 `pipeline/`，这是 gate、dispatch、forward 的共同依赖。

### `pipeline/src/lib.rs`

- 声明 `pub mod ctx`、`stage`、`pipeline`、`router`。
- 从正确模块 re-export `RequestCtx`、`RequestMeta`、`BodySource`、`ProtocolKind`、`Stage`、`StageOutcome`、`StageError`、`Pipeline`、`build_router`。

### `pipeline/src/ctx.rs`

- `BodySource` 实现 Debug；内存 body 输出长度，磁盘 body 输出路径。
- `RequestCtx` 保存请求方法、路径、头、body、认证信息、选中路由、上游响应和流状态。

### `pipeline/src/stage.rs`

- `Stage` trait：`name` 和 `handle`。
- `StageOutcome`：Continue、ShortCircuit、Stream。
- `StageError` 和 `UpstreamError`。

### `pipeline/src/pipeline.rs`

- `Pipeline::push` 添加 Stage。
- `Pipeline::run` 按顺序执行；Continue 进入下一阶段；ShortCircuit 和 Stream 结束链。

### `pipeline/src/router.rs`

- `build_router` 用 Axum 读取 Request，构造 RequestCtx，运行 Pipeline。
- `error_to_response` 输出 OpenAI 错误形状。

### 验收

```sh
cargo check -p gateway-pipeline
```

## MVP：gate

依赖 pipeline。

### `gate/src/auth.rs`

- 从 `Authorization: Bearer`、`x-api-key`、`x-goog-api-key` 提取 key。
- sha256 key 后从 TokenSnapshot 查找。

### `gate/src/state.rs`

- 校验 Token 启用和过期。

### `gate/src/model.rs`

- 校验请求模型在 Token 或用户组白名单内。

### `gate/src/snapshot.rs`

- `ArcSwap<TokenSnapshot>` 保存启动时或管理 API 刷新的 Token 快照。

### `gate/src/chain.rs`

- 顺序执行 auth、state、model。

### 验收

```sh
cargo test -p gateway-gate
```

覆盖 key 提取、禁用 Token、过期 Token、模型白名单。

## MVP：dispatch

依赖 pipeline。

### `dispatch/src/candidate.rs`

- 按 `(group, model)` 从 RouteUnit 快照取候选渠道。

### `dispatch/src/selector.rs`

- 先选最低 priority 层。
- 层内按 weight 随机。

### `dispatch/src/health.rs`

- 保存成功、失败、EWMA 延迟、失败连击和冷却截止时间。
- 连续 5 次失败后冷却 30 秒。

### `dispatch/src/retry.rs`

- 每次失败排除已试渠道。
- 最大三次 attempt。

### 验收

```sh
cargo test -p gateway-dispatch
```

覆盖 priority、weight、冷却、已试渠道排除和最大重试。

## MVP：forward + protocol

依赖 pipeline、dispatch。

### `forward/src/adapter.rs`

- OpenAI 渠道 URL：`{base_url}/v1/chat/completions`。
- 注入渠道 API key 和渠道自定义 header。

### `forward/src/egress.rs`

- 发起上游 HTTP 请求。

### `forward/src/stream.rs`

- SSE 按帧传给客户端。
- 记录 first token 时间。
- 从 usage 帧提取 prompt_tokens、completion_tokens。

### `forward/src/pipeline.rs`

- 使用 RequestCtx 的 SelectedRoute 执行一次上游 attempt。

### `protocol/src/codec/openai.rs`

- OpenAI 请求、非流式响应、SSE 流和错误编解码。

### 验收

```sh
cargo test -p gateway-forward -p gateway-protocol
```

## MVP 后续 crate

- `proxy/`：渠道出口代理、SSRF、HTTP CONNECT、SOCKS5。
- `protocol-bridge/`：Claude、Gemini 协议接入。
- `metering/`：预扣、token 估算和结算。
- `security/`：敏感词、跨 chunk 扫描和审核。
