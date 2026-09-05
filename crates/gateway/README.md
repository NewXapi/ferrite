# `crates/gateway`

## 功能 crate

- `pipeline/` — 请求上下文、Stage 接口、链执行器和 HTTP 路由。
- `gate/` — API Key、状态、配额、频率、模型和并发准入。
- `dispatch/` — 候选渠道、健康状态、权重选择和失败重试。
- `forward/` — 上游 URL、头、请求体、响应体和 SSE 流。
- `proxy/` — 直连、HTTP CONNECT、SOCKS5 和 SSRF 校验。
- `protocol/` — OpenAI、Claude、Claude、Gemini 请求、响应、错误和 SSE 编解码。
- `protocol-bridge/` — 将 pipeline 上下文转换为 protocol codec 输入输出。
- `metering/` — 预扣额度、token 获取与估算、价格和结算。
- `security/` — 词库、输入替换、跨 chunk 扫描和审核结果。

## MVP：pipeline

先修 `pipeline/`，这是 gate、dispatch、forward 的共同依赖。

### 验收

```sh
cargo check -p gateway-pipeline
```

## MVP：gate

依赖 pipeline。

### 验收

```sh
cargo test -p gateway-gate
```

## MVP：dispatch

依赖 pipeline。

### 验收

```sh
cargo test -p dispatch
```

## MVP：forward + protocol-bridge

依赖 pipeline、dispatch。

### 验收

```sh
cargo test -p forward -p gateway-protocol-bridge
```

## MVP：metering

依赖 contract。

### `metering/src/estimate.rs`

- 按字符类加权估算 token (CJK ≈ 0.6, Latin ≈ 0.25, digit ≈ 0.3)。
- 请求体 prompt 侧预扫：提取 `content` 字段文本长度，按 4 chars/token 估算。

### `metering/src/pricing.rs`

- `ModelPrice` 结构体：input/output/cache ($/M tokens) + group_multiplier。
- `price_of` 函数：TokenCounts → 内部单位 (500_000 = $1)。

### `metering/src/ledger.rs`

- `MemoryLedger`：内存 HashMap + per-user Mutex，prehold/settle/release 原子操作。
- `BalanceLedger` trait：available = quota - used - held。

### `metering/src/scanner.rs`

- `StreamScanner`：从 SSE data 行提取 usage (OpenAI/Claude 格式)，无 usage 时按字符数估算。

### `metering/src/settle.rs`

- `settle_event`：扫描结果 + 定价 → UsageEventRecord (含 UUIDv7 meta.key)。

### 验收

```sh
cargo test -p metering
```

## 后续 crate

- `proxy/`：渠道出口代理、SSRF、HTTP CONNECT、SOCKS5。
- `security/`：敏感词、跨 chunk 扫描和审核。
