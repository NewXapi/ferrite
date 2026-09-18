# 代码约定（UI 验证 / Rust 风格 / 调查与审查工具）

> **本文按「问题 → 意图 → 情况 → 约束」四段组织**；写 UI、写 Rust、调查或审查代码前读对应小节。

## UI 验证（Dioxus web）

**遇到什么问题**：截图肉眼判断不可复现、误判多；无契约时 PR smoke 无法自动化验证交互正确性。

**维护者希望做什么事**：
- 交互元素加 `data-testid`（用 `name` 属性值）；容器加 `role` + `aria-label`
- 每页一份 `specs/ui/<page>.yaml` 契约，列出 role / name / testid / action
- PR smoke 用 `tab.ariaSnapshot()` 验证 role + name + testid；**截图仅作辅助**（视觉风格/品牌），失败时附带

**可能的情况**：
- 新增/改动交互元素 → 补 `data-testid` + 更新该页 `specs/ui/<page>.yaml`
- 页面级改动 → 跑 `ariaSnapshot` 断言
- 视觉回归（配色/布局）→ 截图辅助对比

**约束（简略）**：禁区——只用截图肉眼判断、用 class 选择器、不写 ui-spec.yaml 直接 PR。
实操细节在 skill：`.agent/skills/ui-validation/SKILL.md`。

## Rust 编码风格

**遇到什么问题**：`do_config` 式命名看不出目的；公共 API 无注释导致契约漂移。

**维护者希望做什么事**：
- 函数命名**动宾结构**，见名知目的：`parse_channel_config` 而不是 `do_config`；类型/结构体名说清角色
- 公共 API 必须写 rust doc（`///`）：用途、参数语义、错误情况、示例；模块头 `//!` 说明职责

**可能的情况**：
- 新增公共函数/类型 → 先写 rust doc 再实现
- 重构改名 → 保持动宾结构，调用方同步改

**约束（简略）**：公共 API 无 rust doc = 不完整交付。
另：**OCR 是截图工具（图片识别）；`ocr` 命令是代码审查工具（OpenCodeReview）**——审查语境说的是后者。

## 调查与审查工具

**遇到什么问题**：逐文件翻代码慢且看不到调用关系；一次性全 repo 喂 LLM 审查触发限流、信噪比差。

**维护者希望做什么事**：
- 调查：先 `code-review-graph update` 建增量图谱，再查调用关系与全局结构；不直接逐文件翻
  （本地没有 LSP，查调用方靠图谱）
- 审查两层：先 `code-review-graph detect-changes`（结构层 CRG），再 `ocr review`（规范层）；
  ocr 按文件 / 模块分批跑

**可能的情况**：
- 改公共符号前 → 图谱查调用方，列全改点
- PR 收尾 review → CRG（`--base <base_sha>`）+ ocr 分块
- 只读了某模块 → ocr 只喂该模块的 diff

**约束（简略）**：ocr 禁止一次性全 repo 喂入（限流）；CRG / ocr 结论与修复记录写进 PR comment
（gate 的 `pr_crg_review` 检查项要求）。
