# admin-page-auth

登录/注册认证页。

## 职责

管理端唯一公开入口。state context 持有登录态，`init_auth()` 由 `apps/admin-web` 调用
注册 401 静默刷新。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 登录/注册请求 |
| `src/form.rs` | 表单 |
| `src/state.rs` | 登录态 context |
| `src/view.rs` | 页面视图 |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-auth
cargo check --target wasm32-unknown-unknown -p admin-page-auth
cargo test -p admin-page-auth             # CI
```
