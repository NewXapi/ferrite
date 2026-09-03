# `crates/harness`

## 功能 crate

- `core/` — Agent Run、Step、状态、取消和序列化类型。
- `prompt/` — 系统提示、角色资料、历史、变量和上下文裁剪。
- `tools/` — 工具 schema、调用、结果和参数校验。
- `runtime/` — 模型和工具循环、审批、持久化与步骤事件流。
- `ui/` — Agent 步骤、tool call、reasoning 和审批组件。

## 第一轮：core

### `core/src/lib.rs`

- `RunId`、`StepId`、`ToolCallId`。
- `RunStatus`：Running、WaitingForApproval、Completed、Aborted、Failed。
- `Step`：AssistantText、Reasoning、ToolCall、ToolResult、Error、Completed。
- `Run`：id、状态、步骤、模型、创建和更新时间。
- `AbortSignal`：中止标记和等待通知。
- `RunSnapshot`：Run 的 serde 序列化和反序列化。

### `core/tests/`

- 状态转移。
- ToolCallId 与 ToolResult 配对。
- 中止后不再接收步骤。
- Run JSON 往返。

### 验收

```sh
cargo test -p harness-core
cargo check --target wasm32-unknown-unknown -p harness-core
```

## 第二轮：prompt + tools

### `prompt/src/lib.rs`

- `PromptInput`：系统提示、角色资料、用户资料、历史、工具说明。
- `render`：输出 OpenAI messages。
- `expand_variables`：展开 `{{char}}`、`{{user}}`。
- `truncate_history`：按 token 预算从最旧历史开始裁剪。

### `tools/src/lib.rs`

- `ToolSpec`：name、description、parameters JSON Schema。
- `ToolCall`：call id、tool name、arguments JSON。
- `ToolResult`：call id、content、is_error。
- `ToolRegistry`：按 name 取 spec。
- `validate_arguments`：在执行前校验 arguments。

### 验收

```sh
cargo test -p harness-prompt -p harness-tools
cargo check --target wasm32-unknown-unknown -p harness-prompt -p harness-tools
```

## 第三轮：runtime + ui

### `runtime/src/lib.rs`

- `run`：模型请求 → tool call → 执行 → ToolResult → 下一次模型请求。
- `ToolExecutor`：按工具名分发实现。
- `Approval`：需要用户确认的执行请求。
- `RunStore`：追加保存步骤、加载 Run。
- `StepEventStream`：把 Step 发给前端。

### `ui/src/lib.rs`

- `RunTimeline`：按时间显示步骤。
- `ToolCallCard`：显示参数、执行状态和结果。
- `ReasoningCard`：折叠 reasoning。
- `ApprovalDialog`：批准或拒绝工具调用。

### 验收

```sh
cargo test -p harness-runtime
cargo check --target wasm32-unknown-unknown -p harness-ui
```

## 接入酒馆

- `tavern-web/page-chat` 把角色卡、历史传给 `harness-prompt`。
- `apps/api` 用 `harness-runtime` 请求 gateway。
- `tavern-web/page-chat` 用 `harness-ui` 显示 RunTimeline。
