# `tavern-state`

## `src/lib.rs`

- `TavernState`：当前角色 file_name、当前聊天名、`Vec<Message>`。
- `GenerationState`：是否生成中、当前累积文本、中止句柄。
- `load_chat`：选角色后读取聊天历史。
- `append_delta`：把 SSE 文本追加到最后一条角色消息。
- `finish_turn`：生成结束后调用 client 保存完整聊天。
- `abort`：取消进行中的生成请求。

## `tests/`

- 追加 delta 后消息顺序。
- abort 后不再写入新文本。
- finish_turn 保存完整 `Message[]`。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-state
cargo test -p tavern-state
```
