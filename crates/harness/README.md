# `crates/harness`

## 功能 crate

- `core/` — Agent Run、Step、状态、取消和序列化类型。
- `prompt/` — 系统提示、角色资料、历史、变量和上下文裁剪。
- `tools/` — 工具 schema、调用、结果和参数校验。
- `runtime/` — 模型和工具循环、审批、持久化与步骤事件流。
- `ui/` — Agent 步骤、tool call、reasoning 和审批组件。

## 开发顺序

- `core/` — 先定义 Run 和 StreamEvent。
- `prompt/` — 接入角色卡和聊天历史。
- `tools/` — 接入 function calling schema。
- `runtime/` — 接入 gateway 和一个只读工具。
- `ui/` — 在 tavern chat 页面显示步骤。

