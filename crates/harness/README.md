# `crates/harness`

## 目录

```text
harness/
├── core/
├── prompt/
├── tools/
├── runtime/
└── ui/
```

## 要实现

- `core` 定义 Agent Run、Step、状态转移和中止信号。
- `prompt` 组装系统提示、角色上下文、历史消息和变量。
- `tools` 定义 ToolSpec、ToolCall、ToolResult 和参数 schema。
- `runtime` 驱动模型调用、工具执行、结果回灌和步骤事件流。
- `ui` 展示步骤、工具调用、结果和 reasoning。
