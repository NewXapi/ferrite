# `tavern-page-chat`

## src/lib.rs

- `ChatPage` — 加载角色聊天、渲染消息、发送和保存。
- `build_messages` — 角色卡字段和聊天历史转换成 OpenAI messages。
- `StreamView` — 显示流式内容和中止按钮。
- `SwipePicker` — 切换同一条角色消息的 swipes。

## 参考实现

- `/home/hathaway/projects/SillyTavern/public/scripts/openai.js:1533` — prepareOpenAIMessages。
- `/home/hathaway/projects/SillyTavern/public/scripts/swipe-picker.js:52` — openSwipePicker。
