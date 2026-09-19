# 任务书：重构 web 页面 crate

**什么时候用**：把某个 `crates/web/*` 页面 crate 从平铺文件迁到 `tab-page-<name>/` 目录结构。
一个 crate 一份本任务书。

**先读**：`.agent/tasks/web-page-crate-spec.md` 的 §0 硬约束（H1-H4 不可协商）和 §1 目录结构，
再按本文操作。规范是权威，本文只摘操作要点。

---

## 填空

- **目标 crate**：<crate>（如 `admin-page-overview`）
- **worktree / 分支**：<name>（如 `.wt/overview-refactor` ↔ `feat/overview-refactor`）
- **tab → 目录映射**：<pairs>（如 `{"overview":"tab-page-overview", "health":"tab-page-health"}`）
- **base_sha**：<sha>（CRG diff 与核验脚本的基准，不要写死 `main`）

---

## 操作要点（细节见 spec）

1. **选参考** — 打开 `.wt/web-visual` 的 `admin-page-admin`，照 `tab-page-*/` + `lib.rs #[path]` 形态。
   ⚠️ 参考在 `admin-page-admin`，**不是** account（见 spec §10 坑 7）。
2. **定工作面** — 先 `git worktree list` 确认无其他会话在改同一 crate；从**仓库根** `cd` 后
   `git worktree add .wt/<name> -b feat/<name>`；自检路径不含第二个 `.wt/`。
3. **目录化**（spec §5 步骤 1）— 平铺 `<tab>.rs` 整体搬进 `tab-page-<name>/`，原样拆成
   `page.rs` + `shared.rs`，**不做任何抽象**，`cargo check` 绿了算这步完。`lib.rs` 只改 `#[path]`
   不改导出名；同步 crate `README.md` 文件清单。
4. **段落解耦**（spec §5 步骤 2）— `page.rs` 里的编号段落抽成 `stats.rs` / `toolbar.rs` / `list.rs`；
   每抽一段编译一次，逐行核对 testid / aria / class 未变。
5. **双重核验**（spec §7，缺一不可）— 符号级 + testid 级（脚本在 spec §7.2，把 main/ref/pairs 填上）。
   ⚠️ testid 为 0 不是通过，是没锚点，改人工 diff（spec §7.3）。
6. **命名对齐 + 清死导入** — 目录 kebab、页面层统一 `page.rs`；收尾跑一次
   `cargo clippy --all-targets -- -D warnings` 清 unused import（spec §10 坑 4）。
7. **文案 / 注释 / 文档** — 按 spec §3、§4、§5 步骤 5-7 补；与目录迁移不同阶段，可分开做。

**卡住怎么办**：每步必须能编译。编译不过或拆乱了 → `git reset --hard` 回上一个可编译提交重来，
不要在烂状态上硬修（spec §5）。

---

## 验收（spec §8 全量清单，勾完才算完）

- [ ] `cpulimit -l 65 -i -- cargo clippy -p <crate> --all-targets --target wasm32-unknown-unknown -- -D warnings` 零 warning
- [ ] `cargo fmt --all --check` 通过
- [ ] 符号级核验：main 每个平铺文件的符号在拆分后都存在
- [ ] testid 级核验：main 每个 `data-testid` 在拆分后都存在
- [ ] 公开面逐字不变（`apps/admin-web` 与本 crate `tests/` 不需改动）
- [ ] 页面层统一 `page.rs`；目录 kebab；`lib.rs` 用 `#[path]` 桥接
- [ ] rsx 内中文为 0；公开组件 6 要素注释齐全
- [ ] crate `README.md` 同步；`specs/ui/*.yaml` 断言未破坏
- [ ] `gate pre-push` ALL PASS；CI 全绿后 merge

---

## 推进状态

哪些 crate 已迁、剩余清单与各文件行数去向，见 `todo/web-page-refactor-rollout.md`（状态表，不进本任务书）。
