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

- `.wt/<name>/` 是开发工作目录：每个开发会话用 `git worktree add .wt/<name> -b <branch>` 挂独立分支，代码改动只在对应 worktree 里做；仓库根目录只读（除根 `Cargo.toml` 的 workspace member 变更）。
- CPU-heavy 命令必须套 `cpulimit -l 70 -i --`：`cargo build` / `cargo test` / `cargo clippy` / `npm` / `bun` 等编译、测试、装包类命令一律不许裸跑；`git`、`grep`、文件读写等轻量命令不需要。
- `.githooks/` 的 pre-commit / pre-push / merge 拦截信息必须读，按输出修根因；禁止 `--no-verify`、禁止绕过 gate。绕过会让 PR 侧 `gate merge --dry-run` 与 CI 失败。
- 完整编排规范（主控/子代理拆分、CRG + ocr 双层审查、PR comment 规则）见 `~/.config/deskctl/snippets/tasks/dev-implement`，本文件只列环境级约定。
