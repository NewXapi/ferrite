# `harness-prompt`

把前端物化好的 OpenAI-compatible prompt 转成 Rust 侧可序列化、可裁剪的内部
类型。纯数据层，零 runtime 依赖；`wasm32-unknown-unknown` 友好。

## 模块

| 模块 | 内容 |
|---|---|
| `types` | `AgentModelMessage` / `AgentModelContentPart` / `AgentModelRequest` / `PromptInput` / `AgentModelRole` / `AgentModelToolSpec` |
| `prompt_snapshot` | Rust 端边界；校验 `_ferrite_agent_prompt_marker`（兼容 `_tauritavern_agent_prompt_marker`），把 OpenAI JSON 反序列化成内部消息类型，校验 `AgentContextPolicy` |
| `variables` | `expand_variables`：兜底展开 `{{char}}` / `{{user}}`；未识别宏原样保留（前端 macro engine 主责） |
| `render` | `PromptInput → AgentModelRequest`，按 system → messages → tools 顺序拼装 |
| `truncate` | `truncate_history`：从最新到最旧按 token budget 裁剪，system 永保留，tool-call/result 组原子保留 / 丢弃 |
| `reasoning` | `ReasoningTemplate`（默认 Think XML）+ `wrap_reasoning` + `inject_reasoning`：把模型推理回填为额外 system 消息（无 UI） |
| `world_info` | `WorldInfoEntry` / `WorldInfoPosition` / `compute_world_info_budget` / `inject_world_info`，World Info 激活与三路注入纯函数库 |

## 关键常量

- `AGENT_PROMPT_MARKER_FIELD = "_ferrite_agent_prompt_marker"`
- `LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD = "_tauritavern_agent_prompt_marker"`（兼容）

## 设计边界

- **宏展开在前端（Dioxus / WASM）**：所有 `{{time}}` / `/roll` / `setvar` 等在
  前端执行；Rust 侧只接受已物化 snapshot。
- **拒绝未物化 snapshot**：顶层 marker 值非空字符串 / message content 内残留
  `{{` / 任何带 `_ferrite_agent_prompt_marker` 的 part → `PromptSnapshotError::UnfinalizedMarker`。
- **context policy 校验**：snapshot 顶层 `contextPolicy` 与 `ResolvedAgentProfile.context`
  不一致 → `PromptSnapshotError::ContextPolicyMismatch`。
- **不实现**：宏引擎、provider HTTP 调用、Dioxus UI、runtime/repository 抽象。

## 验收命令

```sh
# 单元 + 集成测试（tests/ 同层）
cargo test -p harness-prompt

# WASM 兼容性
cargo check --target wasm32-unknown-unknown -p harness-prompt

# 本 crate fmt 检查
cargo fmt -p harness-prompt -- --check
```

## 依赖

仅 `harness-core`（共享类型 `AgentContextPolicy`）+ `serde` + `serde_json` + `thiserror`。
无 `tokio` / `reqwest` / `axum`，可在 WASM 中编译。

## 公开 API 一览

```rust
use harness_prompt::{
    // 数据类型
    AgentModelMessage, AgentModelContentPart, AgentModelRequest, AgentModelRole,
    AgentModelToolSpec, PromptInput,
    // 解析
    messages_from_payload, message_from_openai_value, content_parts_from_openai_value,
    // 校验
    reject_unfinalized_snapshot, validate_prompt_snapshot_context_policy,
    snapshot_kind, is_prompt_snapshot,
    AgentModelSnapshotKind, PromptSnapshotError,
    AGENT_PROMPT_MARKER_FIELD, LEGACY_TAURITAVERN_PROMPT_MARKER_FIELD,
    // 渲染
    render, RenderError,
    // 变量
    expand_variables, VariableContext,
    // 裁剪
    truncate_history, TruncationDropReason,
    // reasoning 回灌
    ReasoningTemplate, wrap_reasoning, inject_reasoning,
};
```
