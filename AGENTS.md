# Ferrite 工作约定

## 开工前（按顺序读）

1. 读根 `AGENTS.md`（本文件）。
2. 读所属域目录的 `README.md`，确定当前 MVP 顺序和依赖。
3. 读自己功能 crate 的 `README.md`，按文件实现列表工作，按其验收命令验证后提交 conventional commit。

本文件按阅读优先级排布：开工动作 → 开发方式 → 域边界 → 参考知识 → 安全约定 → 硬约束。**过程性内容已全部抽到 `.agent/`**（三区：`playbook/` 规则与流程 / `tasks/` 任务书 / `skills/` omp 自动加载），本文件只留硬约束与**触发条件指针**——遇到什么事读哪份文档，见下方各节「→ 读」行；目录约定见 `.agent/README.md`。

## 开发方式

- `.wt/<name>/` 是开发工作目录：每个开发会话用 `git worktree add .wt/<name> -b <branch>` 挂独立分支，worktree 目录名与分支名尾段一致（`.wt/admin-api` ↔ `feat/admin-api`）；仓库根目录只读（除根 `Cargo.toml` 的 workspace member 变更）。
- **创建 worktree 的硬规则（防嵌套递归）**：
  1. **必须先 `cd /home/hathaway/projects/ferrite`（仓库根）再执行** `git worktree add .wt/<name> -b <branch>`。`git worktree add` 的相对路径是相对**当前 cwd** 解析的——若 cwd 在某个 worktree 内部，`.wt/<name>` 会落进 worktree 里形成嵌套（历史事故：13 层嵌套 + 321G 重复编译产物）。
  2. 执行后**必须自检**：`git worktree list` 中新条目的路径必须是 `/home/hathaway/projects/ferrite/.wt/<name>`。路径含第二个 `.wt/` 即嵌套，立即 `git worktree remove` 撤销重来。
  3. gate 有 `checklist_no_nested_worktree` 护栏（pre-commit/pre-push/merge 扫描 `.wt` 下 mindepth≥2 的 `.wt` 目录与深层 `.git` stub），命中即 FAIL。
  4. 子代理一律用维护者在 prompt 里给的**全局绝对路径**（如 `/home/hathaway/projects/ferrite/.wt/<name>/`），禁止相对路径推导。

## `.wt/` 工作目录（硬约束）

`.wt/<name>/` 是开发工作目录，是每个独立的开发隔离环境。**任何会话严禁在未经确认的情况下删除整个 `.wt/` 目录或他人 worktree 分支目录。** 违规删除 = 丢失他人整个开发会话，等同于删库。

- 只能删除**自己负责的 PR 对应的 worktree 目录**，且必须满足全部条件：
  1. PR 已 squash merge 到 upstream main；
  2. 维护者明确确认可以清理；
  3. 删除前 `git worktree list` 确认目标目录对应当前会话分支，不影响其他 worktree。
- 合并流程结束时：通过 `git worktree remove <自己目录>` + `gh pr merge --delete-branch` 正常释放，严禁使用 `rm -rf .wt/`、`rm -rf .wt/*` 或 `git clean` 进行任何批量/暴力删除。
- 发现 `.wt/` 目录意外丢失时，立即告知维护者并尝试用 `git worktree prune` + `git checkout -b <branch> <merge-commit>` 恢复。

## 域目录并发与越界

- `crates/<domain>/` 是高内聚的开发单元：一个会话接手某域目录即**独占**它——其他会话不会来干扰，它也**不准越界**改动其他域目录下的任何 crate。
- 唯一例外是重构开发需要跨域时：开工前在 PR 报备涉及的域目录清单，确认无在跑会话冲突再动。
- 跨域共享只有 `crates/contract`（共享 API 契约）：需要新 DTO 先声明变更，由一个会话统一修改。
- 粒度分层：域目录 = 大功能；域内 crate = 大功能开发单元；每个文件 = 小功能开发。`lib.rs` 尽量只放共用结构体和 trait，实现在各文件里。

## 目录术语（参考）

```text
crates/api/<prefix-feature>/
crates/web/<prefix-feature>/
```

| 术语 | 位置 | 含义 |
|---|---|---|
| **后端域** | `crates/api/` | 全部后端服务平铺大容器，包含 `auth`（通用账号中心）、`admin-*`（管理服务）、`tavern-*`（酒馆服务）。 |
| **前端域** | `crates/web/` | 全部前端组件与界面平铺大容器，包含 `ui-components`（跨端通用组件）、`admin-page-*`、`tavern-page-*`。 |
| **共享契约** | `crates/contract/` | 跨端共享的独立数据传输对象 (DTO) 与纯协议错误定义。 |
| **网关与执行** | `crates/gateway/`、`crates/harness/` | 渠道调度转发引擎与 Agent 运行时。 |
| **功能 crate** | `crates/<domain>/<name>/` | 独立 Cargo Library Crate，各自拥有独立的 `Cargo.toml`、`src/lib.rs` 与 `tests/`。 |
| **应用** | `apps/<name>/` | 有 `main.rs` 的可执行单体程序，负责配置、状态和路由组装。例：`apps/api`、`apps/admin-web`、`apps/tavern-web`。 |
| **集成测试** | `tests/` | 项目顶层跨 Crate 端到端集成测试套件。 |

## 依赖和组装（参考）

- 功能 crate 只提供 library API；不定义进程入口。
- 域间禁止直接私有依赖：跨端数据交互必须基于 `crates/contract` DTO。
- `apps/api` 统一组装 `crates/api/*`、`crates/gateway/*` 和 `crates/harness/runtime`。
- `apps/admin-web` 组装 `crates/web/admin-page-*` 与 `ui-components`。
- `apps/tavern-web` 组装 `crates/web/tavern-page-*` 与 `ui-components`。
- 每个功能 crate 都有独立的 `Cargo.toml`、`src/lib.rs` 与 workspace member。

## 多会话文件所有权

- 会话所有权以域目录为边界（见上「域目录并发与越界」）；`crates/contract/` 是唯一跨域共享点。
- 根 `Cargo.toml` 只有新增或移动功能 crate 的会话修改；改完说明新增的 workspace member。
- `crates/contract/` 是共享 API 契约；需要新 DTO 时先声明变更，再由一个会话统一修改。
- `apps/api/src/` 只由 API 组装会话修改。
- `apps/admin-web/` 和 `apps/tavern-web/` 只由各自应用组装会话修改。
- 每个功能 crate 的 README 与实现同步更新。

## 安全与环境约定

### 文件删除与清理保护（硬约束）

**核心原则：代码与工作区任何非版本控制的清理，都必须保留后悔药（可恢复），绝对禁止不可逆抹除。**

1. **严禁使用 `git clean`**：
   - **禁止擅自运行 `git clean`**：任何开发会话严禁在未经维护者明确同意的情况下运行任何形式的 `git clean`（包括 `git clean -f`、`git clean -fd`、`git clean -fx`、`git clean -fX` 等）。
   - **危害**：`git clean` 会直接物理擦除未跟踪的改动、临时脚本、本地环境配置文件（如 `.env*`、`config/config.toml`）、跨会话挂载点与实验分支，完全绕过回收站且不可撤销。
2. **严禁使用 Shell 原生永久删除命令（全平台跨 Shell 禁令）**：
   - **Bash / Linux / Unix**：严禁执行 `rm`、`rm -rf`、`rmdir`、`unlink`。
   - **PowerShell (pwsh / Windows)**：严禁执行 `Remove-Item` 以及其所有内置别名 `rm`、`rmdir`、`del`、`erase`、`ri`。
   - **严禁直接永久删除工具**：严禁调用 `gio remove` 或 `gio rm`（它们同样是物理直接抹除，不进回收站）。
3. **唯一合规的文件删除与清理方式**：
   - **本地非 Git 跟踪文件/目录**：一律使用 `gio trash <path>`，将目标安全移入 FreeDesktop 规范的用户标准回收站（`~/.local/share/Trash/`）。
     - **找回与恢复**：若误删或需找回，读取 `~/.local/share/Trash/info/<filename>.trashinfo` 获取原路径，再从 `~/.local/share/Trash/files/<filename>` 移回。
     - **清空回收站**：`gio trash --empty`（属于危险破坏性操作，必须获得维护者明确指令后方可执行）。
   - **Git 跟踪文件的版本控制移除**：允许且必须使用常规版本控制命令 `git rm <path>`。

### 密钥与敏感信息

- 禁止提交：真实 IP 地址、上游/内网地址、API key、token、密钥、密码。提交前扫一眼 diff。
- 本地配置放 `config/config.toml`（已 gitignore，模板见 `config/config.toml.example`）或 `.env*`（已 gitignore）；文档与示例用占位符（`<API_KEY>`、`127.0.0.1`）。

### cpulimit

- CPU-heavy 命令必须套 `cpulimit -l 65 -i --`：编译、测试、装包类（`cargo build` / `cargo test` / `cargo clippy`、`npm` / `bun` 等）以及子代理产出的编译/测试/运行验证，一律不许裸跑；`git`、`grep`、文件读写等轻量命令不需要。

// DONE(2026-09-18): 原 TODO「把大部分内容收进文档，留简略内容 + 触发条件」→ 已抽到 `.agent/playbook/dev-env.md`（四段式），本节只留触发条件与红线。

### 本机 dev 服务与进程卫生（硬约束）

**→ 读**：起后端 / 数据库 / 前端，或遇到 500「Connection refused」、卡片 404 但 curl 200、构建卡死、dx 不重建、`Failed to find binary package` → `.agent/playbook/dev-env.md`（含实测过的命令、症状表、九个坑）。

- **多会话后端/DB 分两种情况**：**共享（默认）**——所有 `.wt/` 会话前端代理指向 `127.0.0.1:3211`，共用 dev DB `uf-local-postgres/ferrite_smoke`，生命周期只走 `dev-backend.sh`；**隔离（独立校验）**——`FERRITE_DEV_LISTEN=127.0.0.1:<port>` 起独立端口 + 本 worktree 的 `config/config.toml` 指向另一库。**红线：严禁停共享 3211、严禁对共享库跑 `db-reset`（清全体数据，跑前报备）。**
- **长跑服务禁止用 `nohup ... &`**：工具调用结束回收进程组 → 服务静默死。用会话的持久后台任务机制起，起后 `ss -ltn` 验证端口再交付。
- **禁止宽匹配 `pkill -f cargo` / `pkill -f rustc`**：那是别的会话在跑的构建（**T 态 ≠ 死进程**）。清理前 `readlink /proc/<pid>/cwd` 认归属，只动无主残留。
- **dev 起停/种子/体检一律走 `justfile` 配方**（`just dev-check` / `dev-backend` / `db-seed` / `db-reset` / `verify`）；开工前先 `just dev-check` 自检。**免登录调试**：`just dev-web <port> debug`（自动登录 dev 种子 `admin_dev`，打开 `#login` 仍可手动调登录页）。

### 测试分层与 CI 驱动原则（摘要）

**→ 读**：决定本地跑什么 / 理解 CI 选包 / **确认新写的测试是否真在跑** → `.agent/playbook/testing-ci.md`。

- 本地只做 `cargo check -p <crate>` 与极小单例调试（≤3s）；一切 `cargo test` 交 PR 的 CI 按 `git diff` 动态选包。
  本机常年 <2GB 内存，**严禁**本地 `cargo test --all` / 全 workspace 构建（会假死）。
- 「通过」= CI 全绿；CI 未全绿不得 closeout / merge。本地 clippy 须与 CI 同版本（`rustup update stable`）。
- ⚠️ **三个静默失效模式**（新增测试前必看 `testing-ci.md` §3.3-3.5）：feature-gated 测试 CI 不开启该 feature（`running 0 tests` 但全绿）、e2e 无 PG 时 30s 超时跳过仍记 passed、`#[ignore]` 在 CI 中永不运行（仓库现有 61 个）。

### gate（`.githooks/`，摘要）

**→ 读**：提交/推送被拦、`gh` 操作被拒、要查规则 → `.agent/playbook/gates.md`；
规则总览在 `.githooks/GATE_HANDBOOK.md`（三层 SLA + 16 条规则表），对照清单在 `.githooks/spec/SPEC_OVERVIEW.md`。

- 拦截信息**逐条读完再修根因**：禁止 `--no-verify`、禁止截断后忽略；**FAIL 必须清零**，WARN 说明理由可放行。
- `gh` 操作在**创建时**即走校验：FAIL 直接拦截（逐条修到清零），WARN 逐条处理（能补就补）；操作前先跑预检，**完整读输出不截断**。
- 占位/TODO 一律 `todo!("TODO(#<issue>): ...")` 或 `unimplemented!(...)`；TODO 注释必须带 issue 号。

### 代码约定（UI / Rust / 调查审查工具）

**→ 读**：写 UI（Dioxus）、写 Rust 公共 API、调查或审查代码 → `.agent/playbook/conventions.md`。

- **UI 验证**：交互元素加 `data-testid`（取 `name` 属性值），容器加 `role` + `aria-label`；每页 `specs/ui/<page>.yaml` 契约；PR smoke 用 `tab.ariaSnapshot()`。禁区：只靠截图肉眼判断、用 class 选择器、不写 ui-spec 直接 PR。细节在 `.agent/skills/ui-validation/SKILL.md`。
- **Rust**：函数名动宾结构见名知目的（`parse_channel_config` 不是 `do_config`）；公共 API 必须写 `///` rust doc（用途/参数/错误/示例），模块头 `//!`。无 doc = 不完整交付。
- **调查/审查**：先 `code-review-graph update` 建图谱再查调用关系，不逐文件翻；审查两层——CRG（结构）+ `ocr review`（规范，按模块分批，禁止全 repo 一次喂）。**OCR 是截图工具，`ocr` 命令是代码审查工具**，别混。

## PR 开发流程（主控 / 子代理编排）

**→ 读**：开 PR / 派子代理编排开发前必读 `.agent/playbook/pr-workflow.md`（八阶段剧本 setup → … → report，含按阶段阅读索引）；派任务给 agent 用 `.agent/tasks/`（任务书模板，见 `tasks/README.md`）。

你是主控 agent：编排任务、派子代理执行、审查子代理产出，**不要亲自把核心实现写完**。

### 硬性门禁（摘要，详版在 playbook）

- **PR-only**：一切工作面以 PR 登记；禁止新建 GitHub issue、禁止改 epic 结构。仅当用户 prompt 明确要求建 issue 时，先报备标题与 done when，批准后才建。
- **工作目录门禁**：子代理必须在 `.wt/<branch>` 工作；prompt 写**全局绝对路径**与所属分支，禁止在仓库根或其他 worktree 落文件。
- **任务量门禁**：单个子任务 ≤ 5 个文件、单一主题、单一修改范围。
- **登记处**：suspect area 写进 PR body；子任务 checkbox 登记进 PR body 清单。每轮「审查+修复」一条 comment，smoke 单独一条。
- 不绕过 `.githooks/` 拦截门，不绕过 `hooks/merge --dry-run` 预检。
- `todo` / goal 登记与 PR body 任务清单全程同步；`base_sha` 记牢，CRG / review 用 `--base <base_sha>`。

## 目标约束

- `harness/core`、`harness/prompt`、`harness/tools` 必须支持 `wasm32-unknown-unknown`。
- `tavern-web/*` 和 `admin-web/*` 必须支持 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 使用 `#[cfg(test)]`。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members` 和对应域目录的 `README.md`。
