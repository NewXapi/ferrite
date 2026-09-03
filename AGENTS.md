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
| **应用** | `apps/<name>/` | 有 `main.rs` 的可执行程序，负责配置、状态和路由组装。例：`apps/api`、`apps/web`、`apps/tavern-web`。 |
| **模块** | `src/<name>.rs` 或 `src/<name>/` | 功能 crate 内部实现文件，不单独进 workspace。 |

## 依赖和组装

- 功能 crate 只提供 library API；不定义进程入口。
- `apps/api` 组装 `gateway/*`、`admin-api/*`、`tavern-api/*` 和 `harness/runtime`。
- `apps/web` 组装 `admin-web/*`。
- `apps/tavern-web` 组装 `tavern-web/*` 和 `harness/ui`。
- 域目录不放 `Cargo.toml`。
- 每个功能 crate 都要有 `README.md`：目录树、要实现的功能、参考实现。
- 每个功能 crate 都要有 `Cargo.toml`、`src/lib.rs` 和 workspace member。

## 目标约束

- `harness/core`、`harness/prompt`、`harness/tools` 必须支持 `wasm32-unknown-unknown`。
- `tavern-web/*` 和 `admin-web/*` 必须支持 `wasm32-unknown-unknown`。
- 测试放同层 `tests/`，不在 `src/` 使用 `#[cfg(test)]`。
- 新增或移动功能 crate 时，更新根 `Cargo.toml` 的 `workspace.members` 和对应域目录的 `README.md`。
