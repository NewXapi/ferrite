# tavern-client

`/tavern/*` 请求 + `generate` SSE 分帧解析。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 请求 + SSE 分帧 |

## 依赖

只依赖 `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-client
cargo check --target wasm32-unknown-unknown -p tavern-client
cargo test -p tavern-client               # CI（sse）
```
