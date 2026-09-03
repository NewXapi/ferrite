# `harness-prompt`

## src/lib.rs

- `PromptInput` — 系统提示、角色上下文、历史消息和工具说明。
- `render` — 按顺序生成模型 messages。
- `expand_variables` — 展开 `{{char}}`、`{{user}}`。
- `truncate_history` — 按上下文预算裁剪最旧历史。

