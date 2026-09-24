# admin-page-overview

总览 / 模型 / 排行榜三个 tab（`tab-page-*/` 目录化）。

## 文件清单

| 路径 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 总览数据请求 |
| `src/shared.rs` | tab 间共享 |
| `src/tab-page-overview/` | 总览 tab：`page` / `stats` / `summary` / `trend` / `histogram` / `sparkline` / `top_lists` / `health` / `errors` / `tooltip` |
| `src/tab-page-models/` | 模型 tab：`page` / `card` / `shared` |
| `src/tab-page-leaderboard/` | 排行榜 tab：`page` / `cards` / `charts` / `data` / `demo_board` / `insights` / `movers` / `prev_window` / `rank_board` / `vendors` / `toolbar` |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-overview
cargo check --target wasm32-unknown-unknown -p admin-page-overview
cargo test -p admin-page-overview         # CI
```
