# Ferrite 工作约定

## 目录术语

```text
crates/<domain>/<feature>/
```

| 术语 | 位置 | 含义 |
|------|------|------|
| **域目录** | `crates/<domain>/` | 业务能力集合，只放 `README.md` 和功能 crate 目录；自身不是 crate。例：`gateway`、`admin-api`、`admin-web`、`tavern-api`、`tavern-web`、`harness`。 |
| **功能 crate** | `crates/<domain>/<feature>/` | 有 `Cargo.toml` 和 `src/lib.rs` 的 library crate。例：`tavern-api/chats`、`gateway/dispatch`、`harness/tools`。提需求时可直接说“做 `tavern-api/chats`”。 |
| **共享 crate** | `crates/<name>/` | 跨域共享的独立 crate。当前只有 `crates/contract`。 |
| **应用** | `apps/<name>/` | 有 `main.rs` 的可执行程序，负责配置、状态和路由组装。例：`apps/api`、`apps/admin-web`、`apps/tavern-web`。 |
| **模块** | `src/<name>.rs` 或 `src/<name>/` | 功能 crate 内部实现文件，不单独进 workspace。 |

## 依赖和组装

- 功能 crate 只提供 library API；不定义进程入口。
- `apps/api` 组装 `gateway/*`、`admin-api/*`、`tavern-api/*` 和 `harness/runtime`。
- `apps/admin-web` 组装 `admin-web/*`。
- `apps/tavern-web` 组装 `tavern-web/*` 和 `harness/ui`。
- 域目录不放 `Cargo.toml`。
- 每个功能 crate 都要有 `README.md`：目录树、要实现的功能、参考实现。
- 每个功能 crate 都要有 `Cargo.toml`、`src/lib.rs` 和 workspace member。

## 多会话文件所有权

- 一个会话只改自己负责的功能 crate。
- 根 `Cargo.toml` 只有新增或移动功能 crate 的会话修改；改完说明新增的 workspace member。
- `crates/contract/` 是共享 API 契约；需要新 DTO 时先声明变更，再由一个会话统一修改。
- `apps/api/src/` 只由 API 组装会话修改。
- `apps/admin-web/` 和 `apps/tavern-web/` 只由各自应用组装会话修改。
- 每个功能 crate 的 README 与实现同步更新。

## 每个会话开工前

1. 读根 `AGENTS.md`。
2. 读所属域目录的 `README.md`，确定当前 MVP 顺序和依赖。
3. 读自己功能 crate 的 `README.md`，按文件实现列表工作。
4. 按该 README 的验收命令验证，再提交一个 conventional commit。

## 目标约束

- `harness/core`、`harness/prompt`、`harness/tools` 必须支持 `wasm32-unknown-unknown`。
- `tavern-web/*` 和 `admin-web/*` 必须支持 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 使用 `#[cfg(test)]`。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members` 和对应域目录的 `README.md`。


## 开发环境约定

### worktree 与子代理

- `.wt/<name>/` 是开发工作目录：每个开发会话用 `git worktree add .wt/<name> -b <branch>` 挂独立分支；仓库根目录只读（除根 `Cargo.toml` 的 workspace member 变更）。
- 默认 **PR-only 开发**：不开 issue，worktree 起手就开 draft PR，任务清单与验收登记在 PR body；用户 prompt 明确要求时才建 issue。
- 开子代理时，prompt 必须写明**全局绝对路径**（如 `/home/hathaway/projects/ferrite/.wt/<name>/`）与所属分支，限定其只在该目录内读写、编译、提交；禁止在仓库根目录或其他 worktree 落文件。

### cpulimit

- CPU-heavy 命令必须套 `cpulimit -l 70 -i --`：编译、测试、装包类（`cargo build` / `cargo test` / `cargo clippy`、`npm` / `bun` 等），以及跑子代理产出的编译/测试/运行验证，一律不许裸跑；`git`、`grep`、文件读写等轻量命令不需要。

### gate（`.githooks/`）

- pre-commit / pre-push / merge 的拦截信息必须逐条读完再修根因；禁止 `--no-verify`、禁止 `| head -5` 之类截断后忽略。FAIL 条目（`checklist.*` / `WS-*` 格式）必须清零，WARN 说明理由后可放行。
- gate 会检查 GitHub 侧规范（issue 关联、PR 结构）；`gh` 操作前先跑对应检查，不要等 push 才发现。
- 占位用 Rust 原生宏：未实现的函数/trait 写 `todo!("TODO(#<issue>): 说明")` 或 `unimplemented!(...)`；TODO/FIXME 注释必须带 issue 号（`TODO(#123): ...`），这是 `rust_todo_needs_issue` 检查项。

### 调查与审查工具

- 调查代码先用 `code-review-graph update` 建增量图谱，再查调用关系与全局结构；不要直接逐文件翻。
- 审查两层：先 `code-review-graph detect-changes`（结构层 CRG），再 `ocr review`（规范层）。ocr 是 LLM 审查，必须按文件/模块分批跑（如 `ocr review --from <base> --to <ref>` 后按块拆），禁止一次性全 repo 喂入，避免限流。


## PR 开发流程（主控 / 子代理编排）

你是主控 agent：编排任务、派子代理执行、审查子代理产出，**不要亲自把核心实现写完**。

### 硬性门禁

- **PR-only**：一切工作面以 PR 登记（见上「开发环境约定」）；禁止新建 GitHub issue、禁止改 epic 结构（挂/摘 sub-issue）。仅当用户 prompt 明确要求建 issue 时，先报备标题与 done when，批准后才建。
- **工作目录门禁**：所有子代理必须在 `.wt/<branch>` 工作；子代理 prompt 必须写明全局绝对路径与分支，禁止在仓库根目录写入。
- **任务量门禁**：单个子任务 ≤ 5 个文件、单一主题、单一修改范围；能按文件 / 范围 / 主题 / 调用链 / 测试拆就拆，不把半个模块丢给一个子代理。
- **登记处**：suspect area 与风险点写进 PR body 对应字段（不进 done when）；子任务以 checkbox 形式登记到 PR body 任务清单，完成勾回。
- 每轮「审查 + 修复」写 **一条** PR comment（含修复 commit SHA）；smoke 验证再单独写 **一条** comment，说明验证手段与结果。两种留言可能多次出现。
- 不绕过 `.githooks/` 拦截门，不绕过 `hooks/merge --dry-run` 的预检。

### workflow（按阶段执行）

#### 0. setup

- 从目标 base 拉 `<branch>`，工作树放 `.wt/<branch>`；不在仓库根目录改。
- 开 draft PR：conventional title，body 含目标 / 范围 / 任务清单 checklist / 验收命令。
- 开工前查最近 24h 内相关在跑 PR / 会话；工作面重叠时停下问用户。
- 记录 `base_sha`，后续 CRG / diff review 用 `--base <base_sha>`，不要写死 `main`。

#### 1. scope

- 跑一次 `code-review-graph update` / 取图谱。
- 修改导出符号前必须 LSP references。
- 找到本次要动的模块、调用方、被调用方、相邻边界。
- 输出：suspect area（写进 PR body）、风险点、可能波及的文件清单。

#### 2. break down

- 先按文件拆；同文件内再按修改范围拆；仍然太大就按主题 / 调用链 / 测试拆。
- 每个子任务必须写清：全局绝对路径 cwd、允许修改的文件、禁止触碰的文件、goal、非目标、验收命令（哪条命令跑通 = 完成）。
- 不相信子代理会自动完成：每个子任务都要有主控可复验的 diff 边界和验收证据。
- 子任务太大、文件边界不清、或需要跨模块协调 → 继续拆；禁止“一个子代理干完半个模块”。
- 子任务登记到 PR body 任务清单，方便后续 closeout 勾回。

#### 3. dev → audit

```
loop1:
  dev   → 派子代理按划分任务做，最多并行 2 个互不冲突子任务；同文件 / 同模块写入必须串行
  audit → 子代理完成后，主控（你）独立校验：
          - 跑子代理提供的验收命令（真跑，不只看输出）
          - diff 看改动是否只落在声明的文件
          - 检查 root cause、调用方、边界输入
          - 必要时再派一个校验子代理做交叉 confirm
失败 → 重拆或回 dev
```

#### 4. test

- 全部子任务通过 audit 后，本地跑针对性测试；CPU-heavy 加 `cpulimit -l 70 -i --`。
- **重型测试**（>2 min、需要容器 / 网络 / 大数据）放 PR CI；CI 未跑完前不得 closeout / merge。
- test failed → 回 loop1，把这个失败当成新的子任务重新走 dev → audit。

#### 5. tool review

- 先 CRG（结构层）：`code-review-graph detect-changes --base <base_sha>`。
- 再 ocr（规范层）：`ocr review --from <base_sha> --to <ref>`；**按模块、按 PR diff 分块喂**，不要一次性 send all（限流）。
- 发现 bug / problem → 回 loop1 修复 → 重新 review，直到干净。
- 每轮（review + fix）→ 1 条 PR comment（含每轮发现、修复 commit、验证命令）。

#### 6. smoke

- 真实用户路径跑一遍：CLI 命令 / 真实 URL / 真实进程；UI 截图或 OCR 对比。
- 发现问题 → 更新 PR 任务清单 → 回 loop1 做二次修复。
- 通过 → 在 PR 写一条「smoke 验证通过 / 用的方法 / 结果」comment。

#### 7. tidy

- **file/dir**：检查分支目录里有没有跟本次开发无关的杂物（旧脚本、临时文件、废弃产物），要么删、要么加 `.gitignore`。
- **code**：测试代码没放 `tests/` 的挪过去；`cargo fmt` / `prettier` / 项目对应 formatter 跑一遍；无调试 log、commented-out code、调试 surrogate；formatter 如修改文件，必须重跑最小验收命令、tool review、smoke，并更新 PR comment。
- **docs**：同步改动的代码注释、`AGENTS.md` / `README.md` / `docs/` 里过期的段落，引用跟新增要一致。

#### 8. report

- report: 改了哪些文件、跑了哪些测试、CRG / ocr / smoke 的结果、PR 链接、剩余风险。
- **收尾报备**：列出本会话新建/修改的全部 PR；有未报备的新建即违规。
