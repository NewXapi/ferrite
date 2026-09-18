# 测试分层与 CI 调度

> 从 `AGENTS.md` 抽离的过程性约定。硬约束摘要（本地只 check、全量测试交 CI）仍保留在根 `AGENTS.md`「安全与环境约定」。

## 测试分层与 CI 驱动原则

- **所有测试放 CI**：`cargo test` 一律在 CI 上跑，本地只跑 `cargo check -p <crate>` 验证编译通过。
- **严禁本地滥跑全量与重型测试**：本地开发机常年可用内存不足 2GB，本地编译或测试多 crate 极易耗尽内存导致机器假死。禁止本地执行 `cargo test --all` 或全 workspace 构建。
- 只有当改动的核心逻辑有单体单测且能在 3 秒内跑完时，才允许本地单跑：`cargo test -p <crate> -- <test_name>`。仅用于调试，不作为验收手段。
- **CI 测试全部 green 才算通过**，CI 未跑完前不得 closeout / merge。
- **本地 clippy 必须与 CI 同版本**：CI 用 `dtolnay/rust-toolchain@stable`。版本落后时本地跑绿仍会被 CI 的新 lint 拦下（实测 1.94 vs 1.98 差 `unnecessary_sort_by`、`result_large_err`）。改动前先 `rustup update stable`，否则只能靠 CI 往返试错。

## CI 调度规则（`scripts/ci-affected.sh`）

PR 跑动态范围，合并到 main 跑全量兜底：

| 触发 | 行为 |
|---|---|
| PR | 按 `git diff` 的 path scope 选包 + 反向依赖闭包 |
| push main | `cargo build --all-targets` + `cargo test --all` |
| 两者恒定 | `cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings` |

动态选包分两步：先用路径前缀匹配得到直接改动的包（seed），再沿 workspace 内部依赖图（`cargo metadata` 里带 `path` 的依赖）反向 BFS，补齐所有依赖它的下游包。

反向闭包是必需的，不是优化：改 `crates/api/tavern-storage` 若只跑该包，会漏掉 `api` 与 `tests-e2e`——它们依赖它，编译能过但测试断言可能已破。实测该改动的真实影响面是 11 个包。

前端 crate（`crates/web/*`、`apps/{admin-web,tavern-web}`）走 wasm32 check，其余走 native check；有 `tests/*.rs` 的包额外跑 `cargo test -p`；纯文档改动秒级跳过。

只有影响面无法从依赖图推导的改动才升级为全量：`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`.github/*`、`scripts/*`。`crates/contract` **不在此列**——它是普通 workspace 成员，闭包能精确算出受影响的 33 个包，比全量 53 个更准。

提 PR 前可本地预览选包结果：

```sh
bash scripts/ci-affected.sh --base newxapi/main --dry-run
```
