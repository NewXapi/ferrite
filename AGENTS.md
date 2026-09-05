# Ferrite 工作约定

## 开工前（按顺序读）

1. 读根 `AGENTS.md`（本文件）。
2. 读所属域目录的 `README.md`，确定当前 MVP 顺序和依赖。
3. 读自己功能 crate 的 `README.md`，按文件实现列表工作，按其验收命令验证后提交 conventional commit。

本文件按阅读优先级排布：开工动作 → 开发方式 → 域边界 → 参考知识 → 安全约定 → 编排流程 → 硬约束。

## 开发方式

- `.wt/<name>/` 是开发工作目录：每个开发会话用 `git worktree add .wt/<name> -b <branch>` 挂独立分支，worktree 目录名与分支名尾段一致（`.wt/admin-api` ↔ `feat/admin-api`）；仓库根目录只读（除根 `Cargo.toml` 的 workspace member 变更）。
- 提交前用 `cargo check` 验证编译（本地环境没有 LSP，check 就是类型错误的兜底）；CPU-heavy 套 cpulimit（见下）。

## `.wt/` 工作目录保护（硬约束）

`.wt/<name>/` 是开发工作目录，也是其他会话的代码容器。**任何会话严禁在未经确认的情况下删除整个 `.wt/` 目录或他人 worktree 分支目录。** 违规删除 = 丢失他人整个开发会话，等同于删库。

- 只能删除**自己负责的 PR 对应的 worktree 目录**，且必须满足全部条件：
  1. PR 已 squash merge 到 upstream main；
  2. 用户明确确认可以清理；
  3. 删除前 `git worktree list` 确认目标目录对应当前会话分支，不影响其他 worktree。
- 合并流程结束时：`git worktree remove <自己目录>` + `gh pr merge --delete-branch`，禁止 `rm -rf .wt/` 或 `rm -rf .wt/*` 批量删除。
- 发现 `.wt/` 目录意外丢失时，立即告知用户并尝试用 `git worktree prune` + `git checkout -b <branch> <merge-commit>` 恢复。

`.wt/<name>/` 是开发工作目录，也是其他会话的代码容器。**任何会话严禁在未经确认的情况下删除整个 `.wt/` 目录或他人 worktree 分支目录。** 违规删除 = 丢失他人整个开发会话，等同于删库。

- 只能删除**自己负责的 PR 对应的 worktree 目录**，且必须满足全部条件：
  1. PR 已 squash merge 到 upstream main；
  2. 用户明确确认可以清理；
  3. 删除前 `git worktree list` 确认目标目录对应当前会话分支，不影响其他 worktree。
- 合并流程结束时：`git worktree remove <自己目录>` + `gh pr merge --delete-branch`，禁止 `rm -rf .wt/` 或 `rm -rf .wt/*` 批量删除。
- 发现 `.wt/` 目录意外丢失时，立即告知用户并尝试用 `git worktree prune` + `git checkout -b <branch> <merge-commit>` 恢复。



## 域目录并发与越界

- `crates/<domain>/` 是高内聚的开发单元：一个会话接手某域目录即**独占**它——其他会话不会来干扰，它也**不准越界**改动其他域目录下的任何 crate。
- 唯一例外是重构开发需要跨域时：开工前在 PR 报备涉及的域目录清单，确认无在跑会话冲突再动。
- 跨域共享只有 `crates/contract`（共享 API 契约）：需要新 DTO 先声明变更，由一个会话统一修改。
- 粒度分层：域目录 = 大功能；域内 crate = 大功能开发单元；每个文件 = 小功能开发。`lib.rs` 尽量只放共用结构体和 trait，实现在各文件里。

## 目录术语（参考）

```text
crates/<domain>/<feature>/
```

| 术语 | 位置 | 含义 |
|------|------|------|
| **域目录** | `crates/<domain>/` | 业务能力集合，只放 `README.md` 和功能 crate 目录；自身不是 crate。例：`gateway`、`admin-api`、`admin-web`、`tavern-api`、`tavern-web`、`harness`。 |
| **功能 crate** | `crates/<domain>/<feature>/` | 有 `Cargo.toml` 和 `src/lib.rs` 的 library crate。例：`tavern-api/chats`、`gateway/dispatch`、`harness/tools`。提需求时可直接说"做 `tavern-api/chats`"。 |
| **共享 crate** | `crates/<name>/` | 跨域共享的独立 crate。当前只有 `crates/contract`。 |
| **应用** | `apps/<name>/` | 有 `main.rs` 的可执行程序，负责配置、状态和路由组装；应用是入口，功能开发发生在域目录的 crate 里。例：`apps/api`、`apps/admin-web`、`apps/tavern-web`。 |
| **模块** | `src/<name>.rs` 或 `src/<name>/` | 功能 crate 内部实现文件，不单独进 workspace。 |

## 依赖和组装（参考）

- 功能 crate 只提供 library API；不定义进程入口。
- 域间禁止直接依赖：跨域引用只允许依赖 `crates/contract`；域内功能 crate 之间可以依赖（如 `admin-web/page-*` → `client`）。
- `apps/api` 组装 `gateway/*`、`admin-api/*`、`tavern-api/*` 和 `harness/runtime`。
- `apps/admin-web` 组装 `admin-web/*`。
- `apps/tavern-web` 组装 `tavern-web/*` 和 `harness/ui`。
- 域目录不放 `Cargo.toml`。
- 每个功能 crate 都要有 `README.md`：目录树、要实现的功能、参考实现。
- 每个功能 crate 都要有 `Cargo.toml`、`src/lib.rs` 和 workspace member。

## 多会话文件所有权

- 会话所有权以域目录为边界（见上「域目录并发与越界」）；`crates/contract/` 是唯一跨域共享点。
- 根 `Cargo.toml` 只有新增或移动功能 crate 的会话修改；改完说明新增的 workspace member。
- `crates/contract/` 是共享 API 契约；需要新 DTO 时先声明变更，再由一个会话统一修改。
- `apps/api/src/` 只由 API 组装会话修改。
- `apps/admin-web/` 和 `apps/tavern-web/` 只由各自应用组装会话修改。
- 每个功能 crate 的 README 与实现同步更新。

## 安全与环境约定

### 文件删除

- 删除用 `gio trash <path>`（进回收站，可恢复）；`gio remove` / `gio rm` / `rm` 是**永久删除**，禁止直接用。git 跟踪文件的删除可以用 `git rm`。

### 密钥与敏感信息

- 禁止提交：真实 IP 地址、上游/内网地址、API key、token、密钥、密码。提交前扫一眼 diff。
- 本地配置放 `config/config.toml`（已 gitignore，模板见 `config/config.toml.example`）或 `.env*`（已 gitignore）；文档与示例用占位符（`<API_KEY>`、`127.0.0.1`）。

### cpulimit

- CPU-heavy 命令必须套 `cpulimit -l 70 -i --`：编译、测试、装包类（`cargo build` / `cargo test` / `cargo clippy`、`npm` / `bun` 等）以及子代理产出的编译/测试/运行验证，一律不许裸跑；`git`、`grep`、文件读写等轻量命令不需要。

### 测试分层

- 本地只跑针对性测试：`cargo test -p <crate>`（受影响的 crate 及其验收命令）。
- 全量测试（`cargo test --all`、全 workspace 构建）放 PR CI，不在本地裸跑。

### gate（`.githooks/`）

- pre-commit / pre-push / merge 的拦截信息必须逐条读完再修根因；禁止 `--no-verify`、禁止 `| head -5` 之类截断后忽略。FAIL 条目（`checklist.*` / `WS-*` 格式）必须清零，WARN 说明理由后可放行。
- gate 会检查 GitHub 侧规范（issue 关联、PR 结构）；`gh` 操作前先跑对应检查，不要等 push 才发现。
- 占位用 Rust 原生宏：未实现的函数/trait 写 `todo!("TODO(#<issue>): 说明")` 或 `unimplemented!(...)`；TODO/FIXME 注释必须带 issue 号（`TODO(#123): ...`），这是 `rust_todo_needs_issue` 检查项。

### Rust 编码风格

- 函数命名讲究动宾结构，见名知目的：`parse_channel_config` 而不是 `do_config`；类型/结构体名说清角色。
- 公共 API 必须写 rust doc（`///`）：用途、参数语义、错误情况、示例；模块头写 `//!` 说明职责。写注释是交付的一部分，不是可选装饰。
- OCR 是**截图工具**（图片识别）；`ocr` 命令是**代码审查工具**（OpenCodeReview）。审查语境下说的是后者，别混淆。

### 调查与审查工具

- 调查代码先用 `code-review-graph update` 建增量图谱，再查调用关系与全局结构；不要直接逐文件翻。
- 审查两层：先 `code-review-graph detect-changes`（结构层 CRG），再 `ocr review`（规范层）。ocr 是 LLM 审查，必须按文件/模块分批跑，禁止一次性全 repo 喂入（限流）。

## PR 开发流程（主控 / 子代理编排）

你是主控 agent：编排任务、派子代理执行、审查子代理产出，**不要亲自把核心实现写完**。

### 硬性门禁

- **PR-only**：一切工作面以 PR 登记（见上「开发方式」）；禁止新建 GitHub issue、禁止改 epic 结构（挂/摘 sub-issue）。仅当用户 prompt 明确要求建 issue 时，先报备标题与 done when，批准后才建。
- **工作目录门禁**：所有子代理必须在 `.wt/<branch>` 工作；子代理 prompt 必须写明**全局绝对路径**（如 `/home/hathaway/projects/ferrite/.wt/<name>/`）与所属分支，限定其只在该目录内读写、编译、提交；禁止在仓库根目录或其他 worktree 落文件。
- **任务量门禁**：单个子任务 ≤ 5 个文件、单一主题、单一修改范围；能按文件 / 范围 / 主题 / 调用链 / 测试拆就拆，不把半个模块丢给一个子代理。
- **登记处**：suspect area 与风险点写进 PR body 对应字段（不进 done when）；子任务以 checkbox 形式登记到 PR body 任务清单，完成勾回。
- 每轮「审查 + 修复」写 **一条** PR comment（含修复 commit SHA）；smoke 验证再单独写 **一条** comment，说明验证手段与结果。两种留言可能多次出现。
- 不绕过 `.githooks/` 拦截门，不绕过 `hooks/merge --dry-run` 的预检。

### workflow（按阶段执行）

#### 0. setup

- 从目标 base 拉 `<branch>`，工作树放 `.wt/<branch>`；不在仓库根目录改。
- 开 draft PR：conventional title，body 含目标 / 范围 / 任务清单 checklist / 验收命令。
- 开工前查最近 24h 内相关在跑 PR / 会话；工作面重叠时停下问用户。
- 用 `todo` / `goal` slash command 登记开发目标与阶段，主控和子代理全程对照确认，偏航即纠正；PR body 任务清单与之同步。
- 记录 `base_sha`，后续 CRG / diff review 用 `--base <base_sha>`，不要写死 `main`。

#### 1. scope

- 跑一次 `code-review-graph update` / 取图谱。
- 修改导出符号前必须查引用（用 codegraph 图谱查调用方，本地没有 LSP）。
- 找到本次要动的模块、调用方、被调用方、相邻边界。
- 输出：suspect area（写进 PR body）、风险点、可能波及的文件清单。

#### 2. break down

- 先按文件拆；同文件内再按修改范围拆；仍然太大就按主题 / 调用链 / 测试拆。
- 每个子任务必须写清：全局绝对路径 cwd、允许修改的文件、禁止触碰的文件、goal、非目标、验收命令（哪条命令跑通 = 完成）。
- **开发必须带测试**：新功能/修复的同一 PR 里补测试，尽可能覆盖完整场景（正常路径、边界、错误输入、并发/重入）；当前无法覆盖的场景用占位宏 `todo!("TODO(#N): 场景说明")` 显式声明纰漏。测试代码同样要写注释：说明测的是什么行为、为什么是这个预期。
- 不相信子代理会自动完成：每个子任务都要有主控可复验的 diff 边界和验收证据。
- 子任务太大、文件边界不清、或需要跨模块协调 → 继续拆；禁止"一个子代理干完半个模块"。
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

- 全部子任务通过 audit 后，本地跑针对性测试（`cargo test -p <crate>`，CPU-heavy 加 `cpulimit -l 70 -i --`）；全量测试交 PR CI。
- **重型测试**（>2 min、需要容器 / 网络 / 大数据）放 PR CI；CI 未跑完前不得 closeout / merge。
- test failed → 回 loop1，把这个失败当成新的子任务重新走 dev → audit。

#### 5. tool review

- 先 CRG（结构层）：`code-review-graph detect-changes --base <base_sha>`。
- 再 ocr（规范层）：`ocr review --from <base_sha> --to <ref>`（OpenCodeReview 代码审查，非截图 OCR）；**按模块、按 PR diff 分块喂**，不要一次性 send all（限流）。
- 发现 bug / problem → 回 loop1 修复 → 重新 review，直到干净。
- 每轮（review + fix）→ 1 条 PR comment（含每轮发现、修复 commit、验证命令）。

#### 6. smoke

- 真实用户路径跑一遍：CLI 命令 / 真实 URL / 真实进程；UI 截图或 OCR 对比。
- 发现问题 → 更新 PR 任务清单 → 回 loop1 做二次修复。
- 通过 → 在 PR 写一条「smoke 验证通过 / 用的方法 / 结果」comment。

#### 7. tidy

- **gate 复检**：跑 `gate pre-commit` / `gate pre-push` 全量规范检查，作为 tidy 的第一道清单；FAIL 清零再进下面各项。
- **file/dir**：检查分支目录里有没有跟本次开发无关的杂物（旧脚本、临时文件、废弃产物），要么 `gio trash`、要么加 `.gitignore`。
- **code**：测试代码没放 `tests/` 的挪过去；`cargo fmt` / `prettier` / 项目对应 formatter 跑一遍；无调试 log、commented-out code、调试 surrogate；rust doc 与实现不一致的更新掉；formatter 如修改文件，必须重跑最小验收命令、tool review、smoke，并更新 PR comment。
- **docs**：同步改动的代码注释、`AGENTS.md` / `README.md` / `docs/` 里过期的段落，引用跟新增要一致。

#### 8. report

- report: 改了哪些文件、跑了哪些测试、CRG / ocr / smoke 的结果、PR 链接、剩余风险。
- **收尾报备**：列出本会话新建/修改的全部 PR；有未报备的新建即违规。

## 目标约束

- `harness/core`、`harness/prompt`、`harness/tools` 必须支持 `wasm32-unknown-unknown`。
- `tavern-web/*` 和 `admin-web/*` 必须支持 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 使用 `#[cfg(test)]`。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members` 和对应域目录的 `README.md`。
