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

## MVP：组装（apps/api，feature 门）

`apps/api` 是唯一的数据面组装入口，cargo feature 决定装配形态：
`default`（tavern + billing，全功能）或 `--no-default-features`（个人形态）。
渠道/模型/密钥**全部来自 Postgres**（`api_channels` / `api_tokens` …），
`config/config.toml` 只放进程级参数——原 `apps/gateway` 的文件配置路线已废除。

- `api_channels.models` JSONB（`{alias, upstream}`）→ `dispatch::Snapshot`
  （`ChannelRecord` + `RouteUnitRecord`），展开见 `apps/api/src/snapshot.rs`。
- `api_channels.settings` JSONB：`headers` 子对象 → 出口头覆盖
  （`forward::extra_headers_from_settings`）；`fallback: true` → 未知模型兜底
  （`dispatch::fallback_units`）。
- `SelectedRoute` 就是 `dispatch::Candidate`（后者是前者的别名）：secret /
  upstream_model / provider_type / settings 随选路一次解析完，`forward` 不查快照。
- 公开别名经 `forward::pipeline::rewrite_upstream_model` 换成上游真名。
- 请求与响应各查自己方向的 codec：`Codec` 有向，跨协议渠道两头都要转。
- `QuotaGate` 只在计费 feature 下挂（价格行来自 `model_prices`）。

### 验收

```sh
cargo check -p api                              # 全功能形态
cargo check -p api --no-default-features        # 个人形态
cargo check -p api --no-default-features --features tavern
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


## 当前进度（2026-09-11）

- 数据面全链路可用：gates（认证/配额/分组）→ dispatch（健康度+加权随机+
  failover）→ forward（SSE 流式透传）→ metering（写 usage_logs 平表）。
- 出站代理：VMess / VLESS / Shadowsocks / Trojan 客户端连接器 +
  WebSocket 传输 + uTLS 指纹可选；节点池（`proxy_nodes`）已接入数据面，
  支持运行中热更新。
- `POST /api/gateway/reload` 热更 token/user/quota 快照与渠道路由，
  无需重启进程。
