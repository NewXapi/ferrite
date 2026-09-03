# `harness-prompt`

## 目录

```text
src/lib.rs
```

## 要实现

- 系统提示、角色上下文、历史消息和工具说明组装。
- `{{char}}`、`{{user}}` 等变量展开。
- 上下文顺序。
- 按历史顺序截断。
- 世界书和 lorebook 注入。

## 参考实现

| 能力 | 上游位置 | 机制 |
|------|---------|------|
| prompt 顺序 | `~/projects/SillyTavern/public/scripts/PromptManager.js` `PromptCollection` / `PromptManager` | prompt 按可排序集合组织，逐条 render |
| 宏展开 | `~/projects/SillyTavern/public/scripts/macros.js:610` `evaluateMacros` | `MacrosParser` 注册宏；新引擎见 `public/scripts/macros/macro-system.js` |
| 上下文来源 | `~/projects/SillyTavern/public/scripts/openai.js:1533` `prepareOpenAIMessages` | 角色卡字段、作者注、世界书、历史消息合并 |
| 上下文压缩 | `~/projects/harness/oh-my-pi/packages/agent/src/compaction/entries.ts` | compaction / branch_summary / reset_boundary 作为会话条目参与历史 |
