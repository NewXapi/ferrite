# `tavern-state`

## 目录

```text
src/lib.rs
```

## 要实现

- 当前角色与聊天。
- 消息、swipe 和流式缓冲。
- 生成中止句柄。
- 聊天保存与历史恢复。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| SSE 分帧 | `~/projects/SillyTavern/public/scripts/sse-stream.js:10` `EventSourceStream` | `parseStreamData` 按厂商解析 delta |
| 流式显示与停止 | `~/projects/SillyTavern/public/scripts/streaming-display.js:35` `StreamingDisplay` | 内建 stop 按钮与 `onStop` 回调 |
| 消息形状与保存 | `~/projects/SillyTavern/public/script.js` `saveChat` | 整份 chat 数组 POST 给 `/api/chats/save` |
