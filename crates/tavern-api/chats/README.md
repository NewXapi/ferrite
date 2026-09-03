# `tavern-chats`

## src/lib.rs

- `Message` — 用户/角色消息、swipes、swipe_id 和未知字段。
- `save` — 整份消息数组覆盖写为 JSONL。
- `load` — 逐行读取 JSONL，跳过坏行。
- `recent` — 按 mtime 返回聊天文件名和首条消息���览。
- `delete/rename` — 聊天文件删除和重命名。

## src/http.rs

- `router` — `GET /tavern/chats/{character}`，以及单个聊天 GET/PUT/DELETE。
- `ChatsState` — 当前用户 UserDirs。

## tests/jsonl.rs

- `JSONL 测试` — 覆盖写、顺序、未知字段、坏行、最近列表、路径穿越。

## 参考实现

- `/home/hathaway/projects/SillyTavern/src/endpoints/chats.js:457` — trySaveChat。
- `/home/hathaway/projects/SillyTavern/src/endpoints/chats.js:359` — getChatInfo 首行预览。
