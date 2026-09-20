# e2e — 管理页浏览器 E2E（Playwright）

补 `tests/`（Rust wire 契约测试）覆盖不到的**浏览器层**回归：wasm 是否真的渲染出来、
行内 Popover 编辑是否可用、请求回路是否稳定。已实测的两类缺陷——DropdownMenu 忙轮询
白屏、users effect 自循环——都只在真实浏览器 + 真实后端里复现，单测与 wasm check 都拦不住。

## 本地跑

```bash
just dev-web 8095                                  # 共享后端 + 免登录 + 预览
cd e2e && bun install                             # 首次
E2E_BASE_URL=http://127.0.0.1:8095 bunx playwright test
```

- `E2E_BASE_URL`：被预览址，默认 `http://127.0.0.1:8095`。
- `E2E_CHROMIUM`：覆盖 chromium 可执行路径；默认用 `~/.cache/ms-playwright` 里已有的
  构建（本机 omp 浏览器缓存即命中），不为跑 smoke 额外下载。CI 上改成
  `npx playwright install --with-deps chromium` 后删掉 `playwright.config.ts` 的
  `executablePath` 即可。
- 依赖**真实 dev 后端**（3211 + dev 种子），不做 API mock：本仓库的 UI 缺陷恰恰生于
  真实请求回路。免登录靠 `debug-auto-login` feature（`just dev-web` 默认开）。

## 结构（POM）

```text
e2e/
  playwright.config.ts     # 单 worker、trace on-first-retry、失败截图
  pages/admin.page.ts      # 导航 + 响应计数助手
  pages/aliases.page.ts    # 别名卡 Popover 编辑
  specs/aliases-popover.spec.ts   # 保存刷新 / Esc+取消不提交 / tab 切换
  specs/users-stability.spec.ts   # 请求稳定性（#230 回归看门狗）
```

选择器约定（`.agent/skills/ui-validation`）：交互元素用 `data-testid`，结构断言用
role+name；不碰 class 选择器。

## 现有用例（5 条，本地全过，~28s）

| 用例 | 守住的回归 |
|---|---|
| 保存本地字段后行值即时刷新 | Popover 编辑闭环（#241） |
| Escape 与取消都只收关不提交 | WCAG dismissible + 不误提交（#241） |
| 定价 tab 露出价格编辑行 | ≤3 tab 结构（#241） |
| 稳定后静置不重拉，刷新只 +1 | users effect 自循环（#230） |
| 搜索是纯本地过滤 | 前端过滤不误发请求 |

## 接进 CI 的方案（待维护者确认后落地）

两个前置，一个 job：

1. **修 Rust e2e 假绿**（独立价值，先做）：CI 加 `services: postgres`，
   `scripts/ci-affected.sh` 前跑迁移 + `db/dev` 种子。现状是 8 个 DB 依赖的
   wire 契约测试连不上库就 30s 超时跳过、被记为 passed（`.agent/rules/testing-ci.md`
   §3.4 有完整判据）。
2. **浏览器 job**（`.github/workflows/ci.yml` 追加，PR 且 `crates/web/**` 或
   `apps/admin-web/**` 变动时触发）：

   ```yaml
   web-e2e:
     name: Web E2E (Playwright)
     if: >-
       github.event_name == 'pull_request' &&
       contains(github.event.pull_request.labels.*.name, 'admin-ui')
     runs-on: ubuntu-latest
     services:
       postgres:
         image: postgres:16
         env: { POSTGRES_USER: ferrite, POSTGRES_PASSWORD: ferrite, POSTGRES_DB: ferrite_e2e }
         ports: ['5433:5432']
     steps:
       - uses: actions/checkout@v4
       - uses: dtolnay/rust-toolchain@stable
         with: { targets: wasm32-unknown-unknown }
       - uses: oven-sh/setup-bun@v2
       - run: cargo build -p api                       # 共享后端二进制
       - run: bash scripts/dev-backend.sh start        # 3211 + 迁移 + 种子
       - run: just dev-web 8092 &                      # dx serve（wasm 约 6min）
       - run: sleep 1 && curl --retry 30 --retry-delay 10 -fsS http://127.0.0.1:8092/
       - run: cd e2e && bun install && bunx playwright install --with-deps chromium
       - run: cd e2e && E2E_BASE_URL=http://127.0.0.1:8092 bunx playwright test
         env: { CI: 'true' }
       - uses: actions/upload-artifact@v4
         if: failure()
         with: { name: playwright-report, path: e2e/playwright-report/ }
   ```

   成本量级：wasm 编译 + dx serve ≈ 8-10 min/job。用 label 门控（`admin-ui`）避免
   纯后端 PR 白跑；这也替换不了「按 diff 动态选包」——浏览器层没有包粒度可选，
   要么整页跑要么不跑。

## 已知边界

- **写路径不覆盖**：现有用例只断言不写后端的路径（本地字段、Esc/取消、Dialog 开消）。
   要测 rename/toggle/delete 真落库，需要「测试专用种子 + afterAll 还原」或独立
   隔离后端（`just dev-web <port> fresh` 的模式），属下一轮。
- **不做 API mock**：与仓库 e2e 哲学一致（真实回路），代价是依赖本地后端活着。
- **别用 `waitForTimeout` 拍固定时长**：现有等待都挂在 testid / `expect.poll` 上；
  稳定性用例里的 `waitForTimeout` 只用于「等启动期结束取基线」，是测量静置窗口，
  不是等元素——替换成事件钩子前先保证它不 flaky（`--repeat-each=10` 验证过再动）。
