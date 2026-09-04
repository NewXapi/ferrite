# `gateway-pipeline`

gateway 编排核心 — 定义 Stage trait / RequestCtx / Pipeline / axum 集成，
是各 gateway crate（gate / dispatch / forward / protocol-bridge / proxy /
security）的公共依赖。反向不依赖任何具体 stage crate，依赖方向单向。

## 文件

- `src/lib.rs` — 模块声明 + re-export + 共享契约类型（`TokenInfo`）。
- `src/ctx.rs` — 请求上下文：`RequestCtx` / `RequestMeta` / `BodySource` /
  `PipeStream` / `ProtocolKind` / `SelectedRoute` / `UpstreamResponse` /
  `StreamedAccum`。
- `src/stage.rs` — `Stage` trait + `StageOutcome`（Continue / ShortCircuit /
  Stream）+ `StageError` / `UpstreamError`。
- `src/pipeline.rs` — 链式 `Pipeline` 编排器（`push` + `run`，遇
  ShortCircuit / Stream / Err 短路）。
- `src/router.rs` — axum 集成：`build_router`（fallback 接 Pipeline）+
  `error_to_response`（StageError → OpenAI 错误形状）。
- `src/error.rs` — 历史兼容路径 re-export（`gateway_pipeline::error::StageError`）。

## 边界（明确不做）

- 快照类型（token / user / pricing / quota / ip-policy / sensitive-words）
  由各自持有方实现，不在本 crate：
  - `gate` 的 `snapshot.rs` / `quota.rs` / `state.rs` / `ratelimit.rs`
  - `security` 的 `wordlist.rs`
  - `dispatch` 的 `health.rs` / `ratelimit.rs`
  - `metering` 的 `ledger.rs` / `pricing.rs`
- `PipeStream` 只做 opaque 包装（`gateway-forward` 的 SsePipe 构造）；
  流式帧扫描在 `protocol` 的 `SseScanner`。

## 验收

```sh
cargo test -p gateway-pipeline   # 7 个编排行为测试
cargo clippy -p gateway-pipeline --all-targets
```