# pipeline

网关编排核心：请求上下文、Stage 接口、链执行器与 HTTP 路由。

## 职责

定义一次网关请求从进来到转发的完整数据流骨架。`ctx` 是贯穿全链的请求上下文（渠道、
模型、用户、trace、计量句柄等）；`stage` 是 Stage trait；`pipeline` 按序执行链；
`router` 把 HTTP 入口（`/v1/*`）映射到链上。

本 crate 只依赖 `contract`，是 gateway 域里被其他 crate 依赖最多的底座。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/ctx.rs` | `GatewayCtx` 请求上下文——贯穿全链的共享状态 |
| `src/stage.rs` | `Stage` trait：`async fn execute(&self, ctx: &mut Ctx) -> Result<()>` |
| `src/pipeline.rs` | 链执行器：按注册序跑 Stage，遇错短路 |
| `src/router.rs` | axum 路由装配 |
| `src/error.rs` | 编排层错误类型 |
| `src/lib.rs` | crate 导出面 |

## 依赖

只依赖 `contract`（+ tokio/axum 等基础设施）。

## 验收

```bash
cargo check -p gateway-pipeline
cargo test -p gateway-pipeline            # CI
```
