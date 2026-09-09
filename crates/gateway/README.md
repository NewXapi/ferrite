# `crates/gateway`

## 功能 crate

- `pipeline/` — 请求上下文、Stage 接口、链执行器和 HTTP 路由。
- `gate/` — API Key、状态、配额、频率、模型和并发准入。
- `dispatch/` — 候选渠道、健康状态、权重选择和失败重试。
- `forward/` — 上游 URL、头、请求体、响应体和 SSE 流。
- `proxy/` — 出口节点解析、按 channel_key 租 HTTP/SOCKS5 Client、SSRF；vless/vmess 未实现。
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

## MVP：单机 standalone 组装

`apps/gateway` 从 `config/config.toml` 直接构造数据面快照，不依赖 Postgres、
`admin-sync` 与计费。渠道与本地 key 写在配置里，进程启动即可转发。

- `[[channels]]` → `dispatch::Snapshot`（`ChannelRecord` + `RouteUnitRecord`）。
- `[[keys]]` → `gateway_gate::snapshot::TokenSnapshot` / `UserSnapshot`。
- `SelectedRoute` 就是 `dispatch::Candidate`（后者是前者的别名）：secret /
  upstream_model / provider_type / settings 随选路一次解析完，`forward` 不查快照。
- `stream` 取自请求体的 `stream` 字段，不按 URL 路径猜。
- 公开别名经 `forward::pipeline::rewrite_upstream_model` 换成上游真名。
- 请求与响应各查自己方向的 codec：`Codec` 有向，跨协议渠道两头都要转。
- `QuotaGate` 只在 `[metering.prices]` 非空时挂上：额度快照为空时它会恒判 402。
- `/healthz` 绕开 gate 链，否则健康检查也会被判 401。

### 验收

```sh
cargo check -p gateway
cargo test -p gateway --test config_wiring
```

## MVP：proxy

`ProxyManager::acquire(channel_key)`（`channel_key` = `RouteUnitRecord.channel_key`）：
- direct/http/socks5 → 返回已注入 `reqwest::Proxy` 的 `reqwest::Client`（`ProxyClient::Reqwest`）
- ss/trojan/vless/vmess → 返回 meow `ProxyAdapter`（`ProxyClient::Adapter`），协议握手在拨号时完成

`ForwardStage::with_proxies` 把租约接到模型请求；二态分派在 `forward::stage`。
节点来自 `config.toml` 的 `[[proxy_nodes]]`（`build_proxy_snapshot` → `ProxyManager::install`）。

### 验收

```sh
cargo check -p gateway-proxy -p forward
```

## MVP：meow 适配器出口

复杂协议出口（SS/Trojan/VLESS/VMess）由 [`meow-proxy`](https://crates.io/crates/meow-proxy)
（MIT）提供：`[[proxy_nodes]]` 经 `gateway_proxy::adapter::adapter_for` 映射成
`ProxyAdapter`，`dial_tcp` 内完成协议握手；`forward::adapter_egress` 把拨出的流
桥成 hyper connector（HTTPS 由 rustls 叠加），`ForwardStage` 不再对协议节点 502。
Reality 由 `meow-transport` 的 `reality` feature 提供（待节点配置扩展接入）。

### 验收

```sh
cargo test -p gateway --test egress_wiring
cargo run --release --example adapter_dial_smoke -p forward
```

