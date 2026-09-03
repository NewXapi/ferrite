# `gateway-pipeline`

## 文件

- `src/lib.rs` — 公开 RequestCtx、Stage、Pipeline、GatewayShared 和 router。
- `src/ctx.rs` — 定义请求元数据、请求体来源、协议类型和跨 stage 可变字段。
- `src/stage.rs` — 定义 Stage trait、StageOutcome、StageError、UpstreamError。
- `src/pipeline.rs` — 按顺序调用 Stage 并处理 Continue、ShortCircuit、Stream。
- `src/router.rs` — 把 Pipeline 接到 Axum fallback，输出 OpenAI 错误体。

