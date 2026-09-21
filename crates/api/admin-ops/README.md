# admin-ops

运维域：系统信息诊断与网关热更 options。

## 职责

管理面的系统诊断页与运行时旋钮。`options` 注册的 key 由前端读写，但**注意**：部分
已注册的 key（如 `gateway.retry.max_attempts` / `gateway.timeout.first_byte_ms`）
在当前 `apps/api` 里没有消费方，改它们对运行进程无效。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/system_info.rs` | 系统信息诊断 |
| `src/options.rs` | 网关热更 options 注册表 |
| `src/lib.rs` | crate 导出面 |

## 验收

```bash
cargo check -p admin-ops
cargo test -p admin-ops                    # CI
```
