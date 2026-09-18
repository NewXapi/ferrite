# 测试分层与 CI 调度

> **本文按「问题 → 意图 → 情况 → 约束」四段组织**，AGENTS.md 只留触发条件。

## 1. 遇到什么问题（含曾经踩过的）

| 问题 | 现象 | 根因（2026-09-18 实测） |
|---|---|---|
| 本地跑测试把机器跑假死 | 内存 <2GB，多 crate 编译耗尽内存 | 本地编译/测试多 crate 极易 OOM |
| 本地跑绿但 CI 红 | CI 用 `dtolnay/rust-toolchain@stable`，版本更新 | 实测 1.94 vs 1.98 差 `unnecessary_sort_by`、`result_large_err` |
| **feature-gated 测试从不执行** | 测试文件被拉起但报 `running 0 tests`，CI 仍全绿 | 无任何 CI 路径开启该 feature（详见 §3.3） |
| **e2e 测试在 CI 里是假绿** | 二进制显示 `ok. N passed`，耗时是 30s 整倍 | CI 无 postgres service，每个测试撞 30s 连接超时后 `return` 跳过，却被记为 passed（§3.4） |
| **61 个 `#[ignore]` 测试永不运行** | `--ignored` 在 CI 中零引用 | CI 跑裸 `cargo test`，`#[ignore]` 等同注释（§3.5） |

## 2. 维护者希望做什么事

- **所有测试放 CI**：本地只 `cargo check -p <crate>` 验证编译；一切 `cargo test` 交 PR 的 CI 按 `git diff` 动态选包。
- **「通过」= CI 全绿**；CI 未全绿不得 closeout / merge。
- 本地 clippy 必须与 CI 同版本：改动前 `rustup update stable`。
- **测试必须真的在跑**：新增测试要确认 CI 里能看到它执行（见 §3.3 的坑）；用 skip/ignore 绕过 PG 依赖的写法，要在测试名或注释里标明「CI 不验此断言」。

## 3. 可能的情况

### 3.1 本地能做什么

| 场景 | 命令 | 限制 |
|---|---|---|
| 编译验证 | `cargo check -p <crate>` | 唯一常规本地验证 |
| 调单个失败用例 | `cargo test -p <crate> -- <test_name>` | 仅调试，**不替代 CI 验收**；须套 cpulimit |
| 预览 CI 选包 | `bash scripts/ci-affected.sh --base newxapi/main --dry-run` | 提 PR 前可跑 |
| 全量/重测试 | ❌ 禁止 | 本地内存不足会假死 |

### 3.2 CI 调度规则（`scripts/ci-affected.sh`）

| 触发 | 行为 |
|---|---|
| PR | 按 `git diff` path scope 选包 + **反向依赖闭包** |
| push main | `cargo build --all-targets` + `cargo test --all` |
| 两者恒定 | `cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings` |

- 选包两步：路径前缀匹配得 seed 包 → 沿 workspace 内部依赖图（`cargo metadata` 带 `path` 的依赖）反向 BFS 补齐下游。
- 反向闭包**必需**：改 `tavern-storage` 只跑该包会漏掉 `api` 与 `tests-e2e`（实测影响面 11 个包）。
- 前端 crate（`crates/web/*`、`apps/{admin-web,tavern-web}`）走 wasm32 check，其余 native；有 `tests/*.rs` 的包额外跑 `cargo test -p`；纯文档改动秒级跳过。
- 升级全量的条件：`Cargo.toml`、`Cargo.lock`、`rust-toolchain.toml`、`.github/*`、`scripts/*`。
  `crates/contract` **不在**此列（普通 workspace 成员，闭包算出的 33 个包比全量 53 个更准）。
- 实测：PR #224 因动 `Cargo.lock` 触发 **FULL WORKSPACE**（33 native + 17 wasm check，42 包跑测试，1035 个测试）。

### 3.3 ⚠️ feature-gated 测试会静默不执行（PR #224 实测）

测试文件写了 `#![cfg(feature = "shuttle")]`（或 crate 级 `cfg(feature=...)`）时：

- **CI 无任何路径开启该 feature**：`ci-affected.sh` 跑裸 `cargo test -p <pkg>`，`ci.yml` 跑 `cargo test --all` / `clippy --all-targets`，均无 `--features`。
- 二进制**会被编译和拉起**，但输出 `running 0 tests ... 0 passed`，整体仍 **green**。
- 判据：`grep '<你的测试名>' <CI 日志>` = 0 次，而 `Running <file>.rs` 出现 → 测试没跑。

**新增 feature-gated 测试时**：必须在 `ci-affected.sh` 的 TEST 段为该包补 `--features <f>`，
或在测试文件头写明「本测试 CI 不执行」，否则等于没写。

### 3.4 ⚠️ e2e 在 CI 里是假绿（无 PG，30s 超时后跳过）

- **CI 无 postgres service**：`.github/workflows/*.yml` 里 `postgres` 出现 0 次。
- e2e 默认连 `postgres://ferrite:ferrite@127.0.0.1:5433/ferrite_e2e`（`FERRITE_E2E_DATABASE_URL` 可覆盖），连不上则
  `eprintln!("skipping e2e: postgres unreachable")` + `return` — **记为 passed**。
- 实测铁证（PR #224 CI 日志）：e2e 二进制耗时是 **30.00s / 60.02s / 90.01s 的精确整倍**（= 连接超时次数 × 30s），
  且 `skipping e2e` 在 CI 日志中出现 **0 次**（stderr 被吞）。
- 涉及文件（`tests/` 顶层 e2e 包）：`billing_lifecycle`、`web_wire_contract`、`users_wire_contract`、
  `manage_wire_contract`、`overview_wire_contract`、`network_write_path`、`usage_log_type`、`admin_gateway_flow`、
  `gateway_e2e`（后者用 MockEgress，不依赖 PG，正常跑）。
- **本地有 PG 时**：`uf-local-postgres` 映射 5433（`ferrite_e2e` 库已建），所以本地跑 e2e 是**真跑**；
  CI 不是。**要改 e2e 行为，本地验证有效、CI 验证无效。**

### 3.5 ⚠️ `#[ignore]` 测试在 CI 中永不运行

- CI 命令从不带 `--ignored`（`scripts/`、`.github/` grep = 0）。
- 全仓 **61 个 `#[ignore]`**，分布 18 个文件：`admin-catalog/tests/models_channels.rs`(15)、
  `auth/tests/integration.rs`(11)、`admin-catalog/tests/channels_groups.rs`(6)、`admin-observe/tests/logs.rs`(5) 等。
- 仓库里已有两套约定，**互相矛盾**：
  - admin-billing 系（`invitees_list.rs:14`、`topup_affiliate.rs:32`、`topup_provider.rs:16`）明确写
    「**skip 而非 `#[ignore]`**：CI 不带 `--ignored`，`#[ignore]` 等于没写」→ 用 `let Some(pool) = … else { return }`。
  - admin-catalog / auth 系用 `#[ignore]` + 注释「需要 `--ignored` 才跑」。
- 两类在 CI 里的**实际效果相同：都不验断言**（skip 派因无 PG 跳过，ignore 派因无 flag 跳过）。
  差异只在本地有 PG 时：skip 派会自动真跑，ignore 派仍需手加 `--ignored`。
- 现状统计：150 个测试文件里 **30 个是 DB 依赖**（skip 或 ignore）。
- **新写 DB 依赖测试**：优先 skip 模式（本地自动真跑），并注释说明 CI 不验此断言。

## 4. 约束事项（简略）

- 本地：只 `cargo check -p <crate>` + 极小单例调试（≤3s），一律套 `cpulimit -l 65 -i --`；
  严禁 `cargo test --all` / 全 workspace 构建（本机常 <2GB 内存，会假死）。
- CI 未全绿不得 closeout / merge；CI 失败提取云端日志回 dev 循环。
- 本地 clippy 与 CI 同版本（`rustup update stable`）。
- 新增测试必须能被 CI 真正执行（避开 §3.3/§3.4/§3.5 三个静默失效模式）。
