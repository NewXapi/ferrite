# admin-client

管理 API 客户端：Bearer 注入、`Envelope<T>` 解码、401 回调 refresher。

## 职责

所有管理端 page crate 的 HTTP 底座。统一处理Bearer 注入、`{"items":..,"total":..}`
信封解码，以及 401 时静默 refresh 后重试。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/setup_client.rs` | reqwest client 装配 |
| `src/wire.rs` | `Envelope<T>` 与线格式 |
| `src/manage_auth_token.rs` | 管理端 auth token 管理 |

## 依赖

只依赖 `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-client
cargo check --target wasm32-unknown-unknown -p admin-client
cargo test -p admin-client                # CI
```
