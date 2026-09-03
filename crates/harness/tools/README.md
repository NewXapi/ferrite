# `harness-tools`

## 目录

```text
src/lib.rs
```

## 要实现

- ToolSpec、ToolCall 和 ToolResult。
- 工具名称、描述和 JSON Schema。
- 工具注册表。
- 工具参数校验。
- 上游 function calling 格式转换。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 工具注册 | `~/projects/SillyTavern/public/scripts/tool-calling.js:269` `registerFunctionTool` | 名字 + 描述 + JSON Schema 参数 |
| 解析工具调用 | `~/projects/SillyTavern/public/scripts/tool-calling.js:427` `parseToolCalls` | 从流式与非流式响应里抽 tool_calls |
| 协议转换 | `~/projects/new-api/apps/api/modules/relaykit/relayconvert` | OpenAI / Claude / Gemini 的 tool_use 与 tool_calls 互转，含流式 tool delta |
| 契约与执行分离 | `~/projects/harness/oh-my-pi/packages/ai/src/types.ts` `Tool` + `AgentTool` | `Tool` 只有 name / description / parameters；`AgentTool` 才带 execute |
| Tool trait | `~/projects/harness/jcode/crates/jcode-tool-core/src/lib.rs` `Tool` | name / description / parameters_schema / execute |
| schema 声明 | `~/projects/harness/deepseek-harness/packages/core/tools/src/schema.ts` `defineTool` | typed spec 编译成 JSON Schema |
| 参数校验前置 | `~/projects/harness/oh-my-pi/packages/agent/src/agent-loop.ts` `validateToolArguments` | 执行前先按 schema 校验模型给的参数 |
| 并发类别 | `~/projects/harness/deepseek-harness/packages/core/tools/src/index.ts` `isConcurrencySafe` | 工具自己声明能否并行 |
