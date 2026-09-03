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

完整编排规范（主控/子代理拆分、审查轮次、PR comment 规则）见 `~/.config/deskctl/snippets/tasks/dev-implement`；本文件只列环境级约定。
