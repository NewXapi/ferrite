# `harness-ui`

## 目录

```text
src/lib.rs
```

## 要实现

- 步骤列表。
- 工具调用参数与结果卡片。
- reasoning 折叠展示。
- 运行状态和中止按钮。
- 工具审批交互。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| 步骤事件来源 | `~/projects/harness/jcode/crates/jcode-message-types/src/lib.rs` `StreamEvent` | UI 直接消费与后端同一套事件枚举 |
| 调用渲染 | `~/projects/harness/oh-my-pi/packages/ai/src/types.ts` `renderCall` / `intent` | 工具自带展示用的调用摘要 |
