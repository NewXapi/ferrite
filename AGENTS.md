# Ferrite 工作约定

## 开工前（按顺序读）

1. 读根 `AGENTS.md`（本文件）。
2. 读所属域目录的 `README.md`，确定当前 MVP 顺序和依赖。
3. 读自己功能 crate 的 `README.md`，按文件实现列表工作，按其验收命令验证后提交 conventional commit。

本文件按阅读优先级排布：开工动作 → 开发方式 → 域边界 → 参考知识 → 安全约定 → 编排摘要 → 硬约束。过程性剧本已抽离到 `.agent/`（`pr-workflow.md` / `testing-ci.md` / `gates.md`），本文件留摘要与指针；`.agent/skills/` 仍是 omp 项目 skill 发现目录，勿混淆。

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

### 本机 dev 服务与进程卫生（硬约束）

- **多会话的后端/DB 分两种情况**：
  1. **共享（默认）**：所有 `.wt/` 会话的前端代理都指向 `127.0.0.1:3211`（`dev-backend.sh` 注释原文），共用同一 dev DB（`uf-local-postgres/ferrite_smoke`）。生命周期只走 `dev-backend.sh` / `just dev-backend`；`db-seed` 幂等可重灌；**`db-reset` 清的是全体会话共享的数据，跑之前必须报备**。
  2. **隔离（独立校验）**：校验需要独占种子数据 / 破坏性迁移时，起独立实例——`FERRITE_DEV_LISTEN=127.0.0.1:<port> scripts/dev-backend.sh start` + 本 worktree 的 `config/config.toml`（gitignored、各 worktree 独立）DSN 指向另一个库（新建库/容器后用 just 变量覆盖调用，如 `just PG_DB=<你的库> db-seed`，见 justfile「PG 连接参数」注）。**严禁停共享 3211 后端、严禁对共享库跑 db-reset**。

- **长跑服务禁止用 `nohup ... &` 在 Bash 工具调用里启动**：工具调用结束会回收整个进程组，服务静默死亡（典型症状：页面 500 "Connection refused"、dx 日志消失）。dx serve / 共享后端一律用会话的持久后台任务机制启动，启动后必须 `ss -ltn` 验证端口在监听再交付。
- **禁止宽匹配 `pkill -f cargo` / `pkill -f rustc` 清进程**：多会话并行时这些是别人正在跑的构建（cpulimit 节流下进程任意瞬间都是 T 态，**T 态 ≠ 死进程**），误杀会让对方会话卡在 cargo 全局锁上、构建假死。清理前必须 `readlink /proc/<pid>/cwd` 确认归属；只处理无主残留。
- **共享 dev 后端（127.0.0.1:3211）生命周期只走 `scripts/dev-backend.sh`**（start / update / stop / status，或 `just dev-backend <args>`）：发现 404/502 先 `just dev-check` 或 `dev-backend.sh status` 判断死活，重启对前端透明（登录态不丢）。
- **「用户侧报错但 curl / 无缓存浏览器实测全 200」→ 先怀疑浏览器 HTTP 缓存重放**：IAB 有独立缓存，代理误配期毒化的错误响应会被本地重放且**不出网**（dx 代理日志 grep 该路径查无请求 = 实锤）。诊断顺序：dx 日志 → IAB 内直接导航该 API URL 看渲染。服务端无法驱逐已毒化条目（只能用户清缓存/重启 webview）；后端 `/api`、`/tavern` 已加 `Cache-Control: no-store` 防复发。
- **dev 起停/种子/体检一律走 `justfile` 配方**，命令清单与使用场景见 `justfile` 顶部「使用场景速查」、疑难处置见其末尾「疑难问题 → 推荐处理」块：`just dev-check`（环境体检）、`just dev-backend start|update|stop|status`（共享后端）、`just db-seed` / `just db-reset`（dev 种子）、`just verify`（fmt-check + clippy + check 全套）。开工前先 `just dev-check` 一条命令自检环境，别再手工拼这些命令。
- **免登录调试前端（`debug-auto-login` feature，默认关）**：admin-web 编译期 dev 专用自动登录——无 token 且不在 `#login`/`#signup`/`#auth` 时静默登录 dev 种子账号 `admin_dev`（仅 dev 种子，生产构建不含此 feature），401 清会话后先尝试自动重登。开启：`just dev-web <port> debug` 或 `dx serve --features debug-auto-login`；要手动调登录页直接打开 `#login`（auth hash 不触发）；彻底关闭用不带 debug 的构建（`just dev-web <port>`）。主动「退出登录」不被自动重登顶掉。

### 测试分层与 CI 驱动原则（摘要）

- 本地只做 `cargo check -p <crate>`（编译验证）与极小的单用例调试（3 秒内跑完的 `cargo test -p <crate> -- <test_name>`）；一切 `cargo test` 交给 PR 的 CI 按 `git diff` 动态选包。本地内存常年 <2GB，**严禁**本地 `cargo test --all` / 全 workspace 构建（会假死）。
- 「通过」= CI 全绿；CI 未全绿不得 closeout / merge。本地 clippy 必须与 CI 同版本（改动前 `rustup update stable`）。
- 细则、动态选包原理与提 PR 前预览：读 `.agent/testing-ci.md`。

### gate（`.githooks/`，摘要）

- 钩子拦截信息必须逐条读完再修根因：禁止 `--no-verify`、禁止截断后忽略；FAIL 必须清零，WARN 说明理由可放行。
- 占位/TODO 一律 `todo!("TODO(#<number>): ...")`、`unimplemented!(...)`；TODO 注释必须带 issue 号。
- `gh` 操作在创建时即走 gate 校验：FAIL 直接拦截，WARN 逐条处理；操作前先跑预检，不截断输出。细则与 GitHub 侧校验清单：读 `.agent/gates.md`。
- `.githooks/` 结构与规则总览：读 `.githooks/GATE_HANDBOOK.md`（三层 SLA + 16 条规则表，一手文档）；规则对照清单在 `.githooks/spec/SPEC_OVERVIEW.md`。

### UI 验证约定（Dioxus web）

详见项目级 skill `.agent/skills/ui-validation/SKILL.md`（共享 OMP `ui-validate` skill）：

- 交互元素加 `data-testid`（用 `name` 属性值）；容器加 `role` + `aria-label`
- PR smoke 用 `tab.ariaSnapshot()` 验证 role+name+testid
- 截图仅作辅助（视觉风格/品牌），失败时附带

禁区：只用截图肉眼判断、用 class 选择器、不写 ui-spec.yaml 直接 PR。

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

### 八阶段工作流程

开 PR / 派子代理编排开发前必读 `.agent/pr-workflow.md`（setup → scope → break down → dev/audit → test → tool review → smoke → tidy → report）；`.agent/` 文档的按阶段阅读索引（哪个阶段读哪个文件）在 `pr-workflow.md` 文件头；`todo` / goal 登记与 PR body 任务清单全程同步。

## 目标约束

- `harness/core`、`harness/prompt`、`harness/tools` 必须支持 `wasm32-unknown-unknown`。
- `tavern-web/*` 和 `admin-web/*` 必须支持 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 使用 `#[cfg(test)]`。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members` 和对应域目录的 `README.md`。
