# `tavern-state`

## src/lib.rs

- `TavernState` — 当前角色、当前聊天和消息列表。
- `GenerationState` — 流式文本、运行状态和中止控制。
- `load_chat` — 选角色后加载聊天历史。
- `save_chat` — 一轮生成结束后保存完整 JSONL。
- `append_delta` — 把 SSE delta 累积到当前角色消息。

