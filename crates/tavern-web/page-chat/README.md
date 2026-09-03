# `tavern-page-chat`

## 目录

```text
src/lib.rs
```

## 要实现

- 消息列表和自动滚动。
- 发送、流式渲染和中止。
- 重新生成和 swipe 切换。
- 消息编辑和删除。
- 角色卡与聊天历史 prompt 组装。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| prompt 组装 | `~/projects/SillyTavern/public/scripts/openai.js:1533` `prepareOpenAIMessages` | 前端拼 messages，后端不参与 |
| 生成参数 | `~/projects/SillyTavern/public/scripts/openai.js:2645` `createGenerationParameters` | 采样参数与 `chat_completion_source` 一起提交 |
| 流式增量 | `~/projects/SillyTavern/public/scripts/openai.js:3128` `getStreamingReply` | 累积 delta 与 reasoning |
| swipe | `~/projects/SillyTavern/public/scripts/swipe-picker.js:52` `openSwipePicker` | `swipe_id` 是 `swipes` 数组下标 |
