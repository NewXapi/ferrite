# tavern-state

角色/聊天/消息全局状态 + 生成中状态 + dock 布局状态。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | 全局状态 |
| `src/dock.rs` | dock 布局状态 |

## 依赖

`contract` + **`harness/prompt`**（web → harness 唯一越界边，改动时注意）。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p tavern-state
cargo check --target wasm32-unknown-unknown -p tavern-state
cargo test -p tavern-state                # CI（dock / state）
```
