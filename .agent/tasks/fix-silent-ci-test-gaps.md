# 测试静默失效模式修复（示例任务书）

> 这是任务书格式的**实例**（同时是真实待办）。模板与约定见 `tasks/README.md`。

- 状态：todo
- 所属 PR / 分支：待创建
- 工作目录：`/home/hathaway/projects/ferrite/.wt/<name>/`（**全局绝对路径**，创建后填实际值）

## 做什么（Goal）

消除三个「测试写了但 CI 从不校验」的静默失效模式，让 CI 的 green 真正代表有效验证。

依据：`.agent/playbook/testing-ci.md` §3.3–3.5（2026-09-18 在 PR #224 的 CI 上实测）。

## 在什么地方

| 层 | 文件 | 动作 |
|---|---|---|
| CI 选包 | `scripts/ci-affected.sh` | 对新增 feature-gated 测试的包补 `--features`（先做 §3.3） |
| e2e | `.github/workflows/ci.yml` | 加 `services: postgres` + `FERRITE_E2E_DATABASE_URL`（治 §3.4） |
| 仓库惯例 | `crates/api/admin-catalog/tests/*.rs`、`crates/api/auth/tests/integration.rs` | `#[ignore]` → skip 模式统一（治法待定，见非目标） |

## 验收标准（done when）

- [ ] §3.3：`cargo test -p metering --features shuttle` 在 CI 中真实执行（CI 日志能 grep 到三个测试名，且不再是 `running 0 tests`）
- [ ] §3.4：e2e 在 CI 中连得上 PG——日志无 30s 整倍耗时、`skipping e2e` 不再出现（或在 CI 里显式失败而非静默跳过）
- [ ] §3.5：至少给出决策——61 个 `#[ignore]` 是「统一改 skip 模式」还是「CI 增加 `--ignored` 单列 job」，并在 AGENTS/playbook 里钉下唯一约定
- [ ] CRG + gate 过；CI 全绿

## 非目标（明确不做）

- ❌ 给 61 个 `#[ignore]` 测试逐个补 PG——先定约定，再分批迁移。
- ❌ 改本地开发流程（本地已有 `uf-local-postgres:5433`，e2e 本地是真跑的）。
- ❌ 引入新测试框架。

## 备注

- 判据速查：CI 里 `grep '<测试名>' <job 日志>`；若为 0 而 `Running <file>.rs` 存在 → 测试没跑。
- e2e 铁证特征：耗时是 30.00s 的精确整倍（连接超时次数 × 30s）。
- 相关 playbook：`testing-ci.md`（§3.3–3.5 详解）、`pr-workflow.md`（阶段 4 test / 阶段 7 tidy）。
