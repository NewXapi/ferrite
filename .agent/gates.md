# gate 操作细则（`.githooks/`）

> 从 `AGENTS.md` 抽离的操作手册。总规则（读全 FAIL 清零、禁止 `--no-verify`）仍保留在根 `AGENTS.md`。

## 本地钩子

- pre-commit / pre-push / merge 的拦截信息必须逐条读完再修根因；禁止 `--no-verify`、禁止 `| head -5` 之类截断后忽略。
- FAIL 条目（`checklist.*` / `WS-*` 格式）必须清零，WARN 说明理由后可放行。
- 占位用 Rust 原生宏：未实现的函数/trait 写 `todo!("TODO(#<issue>): 说明")` 或 `unimplemented!(...)`；TODO/FIXME 注释必须带 issue 号（`TODO(#123): ...`），这是 `rust_todo_needs_issue` 检查项。

## GitHub 侧规范校验

- **gate 会对 GitHub 侧做规范校验**：`gh` 操作（建 issue / 建 PR / 关 issue）在**创建时**就走 gate 校验（issue 必填 heading、PR 标题/body 结构、label 完整性、`Fixes #` 关联等），并输出 `FAIL` / `WARN` 行。
  - `FAIL` 行会**直接拦截**该 gh 操作（建 PR 返回「闸门: 校验 FAIL，拒绝创建」）；必须逐条修到 FAIL 清零再重试，不能只看到报错就放弃或绕过。
  - `WARN` 行不拦截但**不要忽略**：每条都说明缺什么（如「缺 type label」、「关键词建议也挂某 label」、「缺 `Fixes #`」）。能补的就补（label 用 `gh pr edit --label` / `gh issue edit --label`），不能补的在 PR body 写明理由。
  - 创建前先跑 `gate check` 对应清单或 `gate issue` / `gate pr` 预检，**不要等 push 才撞墙**；`head` / `tail` / `grep -v` 这类过滤会吞掉部分提示行，看 gate 输出时务必**完整**读，不要截断。
- gate 会检查 GitHub 侧规范（issue 关联、PR 结构、label 完整性、CRG 审查记录，见 `.githooks/spec/github_pr_gates.yaml` 与 `checklist_pr_*.yaml`）；`gh` 操作前先跑对应检查，不要等 push 才发现。

## 文档位置

- `.githooks/GATE_HANDBOOK.md`：gate 一手总览——三层 SLA（l1 结构 / l2 语义 / l3 LLM）、16 条规则表（触发点 + 严重度）、加规则只改 yaml 不改二进制的约定。
- `.githooks/spec/SPEC_OVERVIEW.md`：规则对照清单（新增/修改规则后必须同步更新该文件）；手动跑 `gate check [names...] --sla {l1|l2|l3} [--json]`。
