# admin-observe

观测聚合：用量日志、统计聚合、渠道探活监控。

## 职责

把 dispatch 的健康读数和 metering 的流水暴露成管理面可看的数据。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/logs.rs` | 用量日志查询 |
| `src/monitor.rs` | 统计聚合 |
| `src/gateway_health.rs` | 渠道探活 / 健康读数 |
| `src/lib.rs` | crate 导出面 |

## 依赖

`gateway/dispatch`（api → gateway 越界边之一）。

## 验收

```bash
cargo check -p admin-observe
cargo test -p admin-observe                # CI
```
