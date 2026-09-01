# ferrite frontend 规范总览

本目录是 ferrite 前端（Dioxus web）的开发规范。规则**只在**本目录的
yaml 参数 + 本文档中；未来接入 gate 工具时按 `dispatch.yaml` 挂到钩子。

```
.githooks/frontend/
├── SPEC_OVERVIEW.md                    # 本文件 (复刻清单)
├── dispatch.yaml                       # 钩子调度 (gate 接入时启用)
├── frontend_structure.yaml             # FR-01 crate 分层与数据边界
├── frontend_shared_components.yaml     # FR-02 共享组件归属
├── frontend_copy_constants.yaml        # FR-03 文案常量 (反硬编码)
├── frontend_tests.yaml                 # FR-04 + FR-07 测试代码划分
├── frontend_no_nested_items.yaml       # FR-05 禁止 fn 内嵌套定义
└── frontend_constants_layers.yaml      # FR-06 常量分层策略
```

## 背景

2026-09-01 落地（commit 见 NewXapi/ferrite main）：

1. 合并 `ui/console`：新增 `page-account` / `page-users` / `crates/mock`
   （零依赖静态数据）；所有 page 拥有 `src/api.rs` 薄壳（admin 例外：走 state.rs）
2. 平铺目录结构：`crates/page/<name>/` → `crates/page-<name>/`，`crates/shared/<name>/` → `crates/<name>/`
3. `ScrollSpyNav` 从 `admin-page/src/ui.rs` 迁到 `crates/ui/src/scroll_spy.rs`
4. 文案常量按共享范围就近：组件内 → 函数顶部；同文件跨组件 → 模块级
5. 测试布局：单元测试留 `src/` 的 `#[cfg(test)] mod tests`；集成测试在 `crates/<name>/tests/<topic>.rs`（7/7 page 都有）

## 规则清单（复刻指南）

### FR-01 crate 分层与数据边界

- **分层**:
  - `crates/page-<name>/` — 页面（account/admin/auth/leaderboard/models/overview/users）
  - `crates/{client,session,mock}/` — HTTP / 会话 / 静态 mock 数据
  - `crates/ui/` — 跨页组件（Field/CodeField/SubmitButton/ScrollSpyNav/SegmentedCapsule）
  - `apps/{api,web}/` — 二进制壳
- **数据流**（硬性）:
  - mock 数据**只**放 `crates/mock/`，面板禁止 `use mock::`
  - 面板只从本页 `src/api.rs` 取数（`fetch_*()` 薄壳）
  - 接真后端只改 api.rs，面板零改动
- **admin 例外**：暂无 api.rs（admin 走 state.rs / entities API 接后端；当前用 mock 写死）
- **leaderboard 例外**：数据留 `leaderboard::data`（`asset!()` 宏需要 dioxus 上下文），通过 api.rs 重导出保持同一入口
- **检测**：`grep -rE '\buse\s+mock\b' crates/page-*/src/ --include='*.rs' | grep -v '/api.rs'` 应无结果

### FR-02 共享组件归属

- 被 ≥2 个 page 使用的组件放 `crates/ui/src/<name>.rs`，lib.rs `pub use` re-export
- page 内禁止 `src/ui.rs` 子模块
- **检测**：`ls crates/page-*/src/ui.rs` 应无结果

### FR-03 文案常量（反硬编码）

- **规则**：同一字面量复用 **2+ 次** → 抽 const；单次使用保留在 rsx
- **位置**（按共享范围就近）：
  - 单组件内复用 → 该组件函数顶部 `const SEC_STATS: &str = "用量统计";`
  - 同文件跨组件复用 → 模块级 `const BTN_CANCEL: &str = "取消";`
  - 跨 page 复用（当前无）→ `crates/ui/src/locale.rs`
- **rsx 语法**（易错）:
  - rsx **子节点**表达式必须包花括号：`button { ..., {BTN_CANCEL} }`、
    `if x { {A} } else { {B} }`
  - **属性值**可直接写常量：`InputCell { label: FIELD_DISPLAY, ... }`
  - 动态插值文案（`"折合 {quota_hint}"`）**不能抽 const**（是 format 字符串）
- **数据不是文案**：人名/渠道名/mock 数值留在 mock crate，不抽
- **i18n 推迟**：单语言阶段不上 fluent / rust-i18n；const 即最终形态

### FR-04 + FR-07 测试代码划分

- **单元测试**：源文件内 `#[cfg(test)] mod tests`（私有函数 + 纯函数）
  - mod 块 < 500 行；超出则拆到 `src/<file>_tests.rs` 或 `tests/<topic>.rs`
- **集成测试**：`crates/<name>/tests/<topic>.rs`（公共入口: api.rs / state.rs）
  - **一文件一主题**：`api_shapes.rs` / `network_smoke.rs` / `auth_flow.rs`
  - **不**要大锅烩 `tests/integration.rs`
  - 单文件 < 200 行；超出按主题再拆
- **覆盖率门槛**：
  - 数学/格式化/物理函数：边界 + 单元
  - api.rs 每个 `pub fn fetch_*()` / `pub struct`：至少 1 个断言
  - mock 数据：id 唯一 / 时间戳非零 / 引用下标有效
- **Dioxus runtime 陷阱**：`Signal::new` 需要 runtime
  - 集成测试用 headless `VirtualDom::new(TestRoot)` + thread_local 槽
  - 例子：`page-admin/tests/api_shapes.rs` 的 `with_runtime`
- **helpers / fixture**：`tests/common/mod.rs` 或 `tests/helpers/<name>.rs`（按需加）

### FR-05 禁止函数体内嵌套定义

- function 体内禁止 `struct` / `enum` 定义（**FAIL**）
- 反例：`ActivityGrid` 内 `struct DayCell { ... }`（**真实存在**——刚修了）
- 理由：阻挡 rust-analyzer 提示；阻挡复用；阻挡命名空间清晰
- 全部提到 fn 之上（mod 级）或文件顶部
- fn 内 `const` 是 **WARN** 而非 FAIL：
  - 组件顶部 `const`（FR-03 模式）**是合规位置**——跟 fn 内不同
  - 1 次使用 + 抽 const 是无理由抽象（**真违规**）
  - 物理算法魔数应归 mod 顶部

 ### FR-06 常量分层策略

按"性质"归类，不混（详见 `frontend_constants_layers.yaml`）：

| 性质 | 抽到哪 | 检测 |
|---|---|---|
| 文案（中/英） | 组件内/模块级 const | FR-03 |
| class 串 | （不抽，留 rsx） | — |
| mock 数据 | `crates/mock/<域>.rs` | FR-01 禁止面板直连 |
| 业务枚举 | 模块级 const | FR-03 同 |
| magic number | 模块级 const | FR-05 fn 内禁 const |
| DOM id | 模块级 const | — |
| endpoint/URL | lib.rs 集中（api 客户端） | — |

**触发升级**：
- 跨 page 复用 → `crates/ui/`
- 主题色板切换 → `crates/ui/src/class.rs`
- i18n 多语言 → `fluent` / `rust-i18n`

## 不写进规范的东西（明确边界）

- ❌ 抽 `<PrimaryButton>` 包装组件（三种变体三种用法，组件接口比 class 串脏）
- ❌ 引入 i18n 库（单语言阶段 const 够用）
- ❌ 独立 `constants` crate（class/strings 都跟组件同居）
- ❌ `class.rs` 共享文件（class 跟 component 住，留 rsx）
- ❌ 切其他 CSS-in-Rust 方案（Tailwind 已是原子化最强解）
- ❌ `tests/integration.rs` 大锅烩（一文件一主题）

## 维护

- 新增规则：加 `frontend_*.yaml` + 本文档追加 FR-XX
- gate 接入：omenic gate 二进制 + `dispatch.yaml` 挂 frontend 主题
- yaml schema 必须符合 omenic：`ignore_paths` + `forbidden_patterns[]`（含 `path_regex`/`pattern`/`reason`/`suggestion`/`severity`/`paths_include`/`paths_exclude`/`exemption_markers`）+ `expected_locations[]`（含 `file_pattern`/`expected_dir`）