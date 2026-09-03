# `tavern-chats`

## 目录

```text
src/
├── lib.rs
└── http.rs
```

## 要实现

- Message 和 swipe 数据结构。
- JSONL 保存、读取、删除和重命名。
- 最近聊天列表和首条消息预览。
- 聊天导入导出。
- 聊天备份。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 保存 | `~/projects/SillyTavern/src/endpoints/chats.js:457` `trySaveChat` | 整份数组序列化成 JSONL 覆盖写，再触发节流备份 |
| 完整性校验 | `~/projects/SillyTavern/src/endpoints/chats.js:316` `checkChatIntegrity` | 首条消息 `chat_metadata.integrity` 对不上则拒绝写入 |
| 首行预览 | `~/projects/SillyTavern/src/endpoints/chats.js:359` `getChatInfo` | 最近列表只读首行 |
| 路由集 | `~/projects/SillyTavern/src/endpoints/chats.js:470` | save / get / rename / delete / export / import / search / recent |
