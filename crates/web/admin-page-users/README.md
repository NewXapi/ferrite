# admin-page-users

用户管理面板。

## 文件清单

| 路径 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 用户数据请求 |
| `src/shared.rs` | 用户页文案常量 |
| `src/format.rs` | 用户页纯格式化函数 |
| `src/tab-page/` | 每 tab 一个页面文件；本 crate 目前只有 `users.rs` 编排层 |
| `src/components/` | 本 crate 独有的组件层（chips、cards、forms、filters 等） |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract` + `ui-components`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-users
cargo check --target wasm32-unknown-unknown -p admin-page-users
cargo test -p admin-page-users            # CI
```
