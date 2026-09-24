# admin-page-admin

管理操作页：渠道 / 分组 / 别名 / 兑换 / 网络 / 系统等 10+ 个 tab（`tab-page-*/` 目录化）。

## 文件清单

| 路径 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 管理面数据请求 |
| `src/state.rs` | 页面状态 |
| `src/drawer_write.rs` | 抽屉写操作 |
| `src/tab-page-channels/` | 渠道 tab：`page` / `list` / `modal` / `toolbar` / `stats` / `shared` |
| `src/tab-page-groups/` | 分组 tab：同上结构 |
| `src/tab-page-aliases/` | 别名 tab：同上结构 |
| `src/tab-page-redemptions/` | 兑换码 tab：`page` / `list` / `card` / `modal` / `toolbar` / `stats` / `shared` |
| `src/tab-page-subscriptions/` | 订阅 tab：`page` / `card` / `modal` / `parse-url-key` / `shared` |
| `src/tab-page-currency/` | 币种 tab：`page` / `list` / `form` / `shared` |
| `src/tab-page-entities/` | 实体 tab：`page` / `cards` / `channels` / `shared` |
| `src/tab-page-gateway/` | 网关 tab：`page` / `row` / `shared` |
| `src/tab-page-network/` | 网络拓扑 tab：`page` / `data` / `drawer` / `inspector` / `physics` / `ui` / `shared` |
| `src/tab-page-system/` | 系统 tab：`page` / `overview` / `options` / `proxy-nodes` / `proxy-runtime` / `shared` |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-admin
cargo check --target wasm32-unknown-unknown -p admin-page-admin
cargo test -p admin-page-admin            # CI（16 个测试文件）
```
