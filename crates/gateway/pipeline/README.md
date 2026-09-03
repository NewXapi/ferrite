# `gateway-pipeline`

## 目录

```text
src/
├── lib.rs
├── ctx.rs
├── stage.rs
├── pipeline.rs
└── router.rs
```

## 要实现

- RequestCtx、RequestMeta 和请求体来源。
- Stage trait、StageError 和 StageOutcome。
- Pipeline 编排。
- GatewayShared 跨请求状态。
- Axum 路由和错误响应。
