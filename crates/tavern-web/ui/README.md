# `tavern-ui`

## `src/lib.rs`

- `MessageBubble`：名字、内容、用户/角色样式、Markdown。
- `ChatInput`：多行输入、回车发送、生成中禁用。
- `Dialog`：确认弹窗。
- `Loading`、`EmptyState`。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-ui
```
