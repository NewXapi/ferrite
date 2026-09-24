# proxy

出口节点解析、HTTP/SOCKS5 Client 租借与 SSRF 防护。

## 职责

按 channel_key 解析出口节点（DB 里的 `proxy_nodes` 经 admin-proxy 写侧维护），
池化租借 HTTP/SOCKS5 Client，并在发出请求前做 SSRF 检查。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/manager.rs` | 出口管理器：节点解析 + Client 租借 |
| `src/node.rs` | 出口节点模型 |
| `src/pool.rs` | Client 连接池 |
| `src/probe.rs` | 节点探活 |
| `src/ssrf.rs` | SSRF 防护（禁内网/回环/metadata 地址） |
| `src/sharelink.rs` | 分享链接解析成节点 |
| `src/adapter.rs` | 出口适配 |
| `src/lib.rs` | crate 导出面 |

## 依赖

只依赖 `contract`。

## 验收

```bash
cargo check -p gateway-proxy
cargo test -p gateway-proxy                # CI（ssrf / affinity / node_cooldowns / ws_transport）
```
