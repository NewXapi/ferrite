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
| `src/tab-page/` | 每 tab 一个页面文件；本 crate 目前只有 `auth.rs` 编排层 |
| `src/components/` | 本 crate 独有的组件层（auth_form / auth_state / auth_view） |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract` + `ui-components`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-auth
cargo check --target wasm32-unknown-unknown -p admin-page-auth
cargo test -p admin-page-auth             # CI
```
