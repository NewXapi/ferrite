# `tavern-chats`

## src/lib.rs

- `Message` — 用户/角色消息、`is_system`、`swipes`、`swipe_id`、未知顶层字段。
- `MessageExtra` — Agent 元数据（`api` / `model` / `reasoning` /
  `reasoning_duration` / `token_count` / `gen_started` / `gen_finished` /
  `title` / `force_avatar`），平铺在 Message 顶层，并通过 `additional`
  flatten 保留未声明字段。
- `save` — 整份消息数组覆盖写为 JSONL。
- `load` — 逐行读取 JSONL，跳过坏行。
- `recent` — 按 mtime 返回聊天文件名和首条消息预览。
- `delete/rename` — 聊天文件删除和重命名。

## src/http.rs

- `router` — `GET /tavern/chats/{character}`，以及单个聊天 GET/PUT/DELETE。
- `ChatsState` — 当前用户 UserDirs。

## tests/jsonl.rs

- 顺序、覆盖写、坏行、未知顶层字段、路径穿越。
- 老格式 JSONL 反序列化（无 `is_system` / 无 MessageExtra 字段）。
- `MessageExtra` 顶层 round-trip。
- `is_system` 序列化 / 默认值。
- 未知顶层字段透传到 `MessageExtra.additional`。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/endpoints/chats.js:457` — trySaveChat。
- `/home/hathaway/projects/SillyTavern/src/endpoints/chats.js:359` — getChatInfo 首行预览。

## 验收

```sh
cargo test -p tavern-chats
cargo check -p tavern-chats
cargo fmt -p tavern-chats -- --check
```