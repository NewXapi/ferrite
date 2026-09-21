# admin-page-users

用户管理面板。

## 文件清单

| 路径 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 用户数据请求 |
| `src/data.rs` | 数据整形 |
| `src/tab-page-users/` | `page` / `user_card` / `user_form` / `modal` / `topup_form` / `role_chips` / `group_chips` / `badge` / `shared` |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-users
cargo check --target wasm32-unknown-unknown -p admin-page-users
cargo test -p admin-page-users            # CI
```
