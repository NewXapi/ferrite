# admin-proxy

出口代理节点管理：`[[proxy_nodes]]` 的 DB 化写侧。

## 职责

管理 `proxy_nodes` 表（增删改查、探活上报、订阅导入），供 `gateway/proxy` 读取。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/subscription.rs` | 节点订阅导入 |
| `src/lib.rs` | crate 导出面 |

## 依赖

`gateway/proxy`（api → gateway 越界边之一）。

## 验收

```bash
cargo check -p admin-proxy
cargo test -p admin-proxy                  # CI
```
