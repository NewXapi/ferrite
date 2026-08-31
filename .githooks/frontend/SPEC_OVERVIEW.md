# ferrite frontend 规范总览

本目录是 ferrite 前端（Dioxus web）的开发规范。规则**只在**本目录的
yaml 参数 + 本文档中；未来接入 gate 工具时按 `dispatch.yaml` 挂到钩子。

```
.githooks/frontend/
├── SPEC_OVERVIEW.md                  # 本文件
├── dispatch.yaml                     # 钩子调度（gate 接入时启用）
├── frontend_structure.yaml           # FR-01 crate 分层与数据边界
├── frontend_shared_components.yaml   # FR-02 共享组件归属
├── frontend_copy_constants.yaml      # FR-03 文案常量（反硬编码）
└── frontend_tests.yaml               # FR-04 测试代码划分
```

## 背景

2026-09-01 落地（commit 见 NewXapi/ferrite main）：

1. 合并 `ui/console`：新增 `page-account` / `page-users` / `crates/shared/mock`
   （零依赖静态数据）；所有 page 拥有 `src/api.rs` 薄壳
2. 高频复用文案抽为常量：跨组件的放模块级（`const BTN_CANCEL: &str = "取消"`）,
   组件内的放函数顶部（`const SEC_STATS: &str = "用量统计"`）
3. 测试布局:单元测试留在 `src/` 的 `#[cfg(test)] mod tests`;
   集成测试在 `crates/<name>/tests/`（`api_shapes.rs` 数据形状不变量）

## 规则清单（复刻指南）

### FR-01 crate 分层与数据边界

- **分层**:
  - `crates/page/<name>/` — 页面 crate（admin/auth/overview/models/leaderboard/account/users）
  - `crates/shared/{client,session,mock}/` — HTTP/会话/静态 mock 数据
  - `crates/ui/` — 跨页组件（Field/CodeField/SubmitButton/ScrollSpyNav/SegmentedCapsule）
- **数据流**（硬性）:
  - mock 数据只放 `crates/shared/mock/`,面板禁止直接 `use mock::`
  - 面板只从本页 `src/api.rs` 取数（`fetch_*()` 薄壳）
  - 接真后端只改 api.rs,面板零改动
- **例外**:leaderboard 数据留 `leaderboard::data`（`asset!()` 宏需要 dioxus 上下文,
  进 mock 会拖依赖）,通过 api.rs 重导出保持同一入口
- **检测**:`grep -rE 'use mock::' crates/page/*/src/ --include='*.rs' | grep -v api.rs` 应无结果

### FR-02 共享组件归属

- 被 ≥2 个 page 使用的组件放 `crates/ui/src/<name>.rs`,lib.rs re-export
- page 内禁止 `src/ui.rs` 子模块
- **检测**:`ls crates/page/*/src/ui.rs` 应无结果

### FR-03 文案常量（反硬编码）

- **规则**:同一字面量复用 2+ 次 → 抽 const;单次使用保留在 rsx
- **放置位置**（按共享范围就近）:
  - 单组件内复用 → 该组件函数顶部 `const SEC_STATS: &str = "用量统计";`
  - 同文件跨组件复用 → 模块级 `const BTN_CANCEL: &str = "取消";`
  - 跨 page 复用（当前无此类）→ `crates/ui/`
- **rsx 语法**（易错）:
  - rsx **子节点**表达式必须包花括号:`button { ..., {BTN_CANCEL} }`、
    `if x { {A} } else { {B} }`
  - **属性值**可直接写常量:`InputCell { label: FIELD_DISPLAY, ... }`
  - 带引号文本子节点:`p { ..., "{FOOTER_HINT}" }` 是字面插值;
    含运行时变量的文案**不能抽 const**（`"折合 {quota_hint}"` 是 format 插值）
- **数据不是文案**:人名、渠道名、mock 数值留在 mock crate,不抽
- **i18n 推迟**:单语言阶段不上 fluent/rust-i18n;const 即最终形态

### FR-04 测试代码划分

- **单元测试**:私有函数留在源文件 `#[cfg(test)] mod tests`
  （如 `page-admin/src/network.rs` 的 bezier/fit_view 物理函数）
- **集成测试**:公共入口放 `crates/<name>/tests/<topic>.rs`
  - `page-admin/tests/api_shapes.rs` — `EntityStore::seed()` 形状不变量
  - `page-account/tests/api_shapes.rs` — api 薄壳数据自洽性（id 唯一/时间戳非零）
  - `page-overview/tests/api_shapes.rs` — 统计/分布数据占比越界检查
- **Dioxus runtime 陷阱**:`Signal::new` 需要 runtime;集成测试用
  headless `VirtualDom::new(TestRoot)` + thread_local 槽传递测试闭包
  （见 admin/tests/api_shapes.rs 的 `with_runtime`）
- **必须覆盖**:数学/格式化/物理函数（bezier、ease、fit_view）;api 数据形状

## 维护

- 新增规则:加 `frontend_*.yaml` + 本文档追加 FR-XX
- gate 接入:omenic gate 二进制 + dispatch.yaml 挂 frontend 主题
