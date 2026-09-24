# tavern-page-chat

聊天与流式生成互动界面（dock 布局）。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 聊天主界面 |
| `src/dock.rs` | dock 容器 |
| `src/dock_panels.rs` | dock 面板 |
| `src/layout.rs` | 布局 |

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-page-chat
cargo check --target wasm32-unknown-unknown -p tavern-page-chat
```
