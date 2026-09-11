# `tavern-client`

## `src/lib.rs`

- `Character`、`CharacterSummary`、`ChatSummary`、`Message`：字段对齐 `tavern-api/characters` 和 `tavern-api/chats`（`ChatSummary` = 会话列表项,仅 `file_name` + `preview`）。
- `list_characters`、`get_character`、`create_character`、`update_character`、`delete_character`。
- `recent_chats`（返回 `Vec<ChatSummary>`）、`load_chat`、`save_chat`、`delete_chat`。
- `load_settings`、`save_settings`。
- `secrets_state`、`put_secret`。
- `generate`：POST `/tavern/generate`，解析 `data:` SSE 行，读取 `choices[0].delta.content`，`[DONE]` 结束；无 `delta.content` 的合法 JSON chunk(如 role-only 头帧)丢弃,非 JSON 裸文本行透传。
- `ApiError`：网络、HTTP 状态、JSON 解码。

## `tests/`

- SSE `data:` 分帧。
- `[DONE]` 结束。
- 多段 delta 累积。
- 未知 JSON 字段保留。

## 验收

```sh
cargo check --target wasm32-unknown-unknown -p tavern-client
cargo test -p tavern-client
```
