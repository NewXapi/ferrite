# `crates/tavern-web`

## 目录

```text
tavern-web/
├── client/
├── state/
├── ui/
├── page-characters/
├── page-chat/
└── page-settings/
```

## 要实现

- `client` 调用 tavern-api 并消费 SSE。
- `state` 保存当前角色、聊天、消息和流式状态。
- `ui` 提供消息气泡、输入框、弹窗和加载组件。
- `page-characters` 提供角色列表和角色卡编辑。
- `page-chat` 提供聊天、生成、swipe、消息编辑和历史恢复。
- `page-settings` 提供连接配置、采样参数和模型选择。
