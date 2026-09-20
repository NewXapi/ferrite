# e2e — 管理页浏览器 E2E（Playwright）

补 `tests/`（Rust wire 契约测试）覆盖不到的**浏览器层**回归：wasm 是否真的渲染出来、
行内 Popover 编辑是否可用、分页切片与请求回路是否稳定。已实测的两类缺陷——DropdownMenu
忙轮询白屏、users effect 自循环——都只在真实浏览器 + 真实后端里复现，单测与 wasm check 都拦不住。

**本套件不在当前 CI 里**：没有 browser job，也没有待落地的 CI 草案；只在本地对着 debug
预览手跑。要接 CI 需先解决真实后端（迁移 + dev 种子）与 chromium 安装两个前置，属维护者
决策，本文档不代拟实现。

## 本地跑

```bash
just dev-web 8095 debug                           # debug 档：debug-auto-login 免登录（admin_dev）
cd e2e && bun install                             # 首次
E2E_BASE_URL=http://127.0.0.1:8095 bunx playwright test
```

- **必须 debug 档**：`just dev-web <port> debug` 启用 debug-auto-login feature，无 token
  自动登录 dev 种子账号；普通档没有自动登录，页面停在登录/注册页，全部用例假失败
  （旧文档漏写 `debug`，踩过这个坑）。
- debug 档首拉 wasm 是 CPU-heavy 构建（首次数百秒），按仓库约定重命令套
  `cpulimit -l 65 -i --`（见 `.agent/rules/testing-ci.md`）。
- `E2E_BASE_URL`：被预览址，默认 `http://127.0.0.1:8095`。
- `E2E_CHROMIUM`：覆盖 chromium 可执行路径；默认用 `~/.cache/ms-playwright` 里已有的
  构建，不为跑 smoke 额外下载。
- 依赖**真实 dev 后端**（3211 + dev 种子），不做 API mock：本仓库的 UI 缺陷恰恰生于
  真实请求回路。

## 结构（POM）

```text
e2e/
  playwright.config.ts            # 单 worker、trace on-first-retry、失败截图
  pages/admin.page.ts             # 导航 + 认证就绪等待 + 响应计数/负向观察窗
  pages/aliases.page.ts           # 别名卡 Popover 编辑
  specs/aliases-popover.spec.ts   # 保存刷新 / Esc+取消不提交 / tab 切换
  specs/pager.spec.ts             # 卡片网格分页器（期望值取自真实响应）
  specs/users-stability.spec.ts   # 请求稳定性（#230 回归看门狗）
```

选择器约定（`.agent/skills/ui-validation`）：交互元素用 `data-testid`，结构断言用
role+name；不碰 class 选择器。卡片/列表/分页器 testid（维护者确认的最终态）：

| 列表 | 容器 | 卡片 | 分页条 |
|---|---|---|---|
| users | `users-list` | `user-card` | `users-pager` |
| aliases | `aliases-list` | `alias-card-new` | `aliases-pager` |
| channels | `channels-list` | `channel-card-new` | `channels-pager` |
| groups | `groups-list` | `group-card` | `groups-pager` |
| redemptions | `redemptions-list` | `redemption-card` | `redemptions-pager` |
| subscriptions | `subscriptions-list` | `subscription-card` | `subscriptions-pager` |

页码按钮 `<prefix>-pager-page-{i}`（0 基），当前页带 `aria-current="page"`，上/下页
箭头 `<prefix>-pager-prev` / `-next`。

## 等待约定（无 waitForTimeout）

- 认证就绪 = 导航路径上出现**首个 2xx 列表响应**（`AdminPage.gotoTab` 的 `apiPath`
  参数，监听挂在 `goto` 之前）；401/5xx 不兜底，免登录链路真断了就超时失败。
- 负向断言（翻页 / 静置 / 搜索不发请求）用 `AdminPage.requestArrivedWithin(path, windowMs)`
  的有界观察窗：窗口内出现该路径响应返回 true，调用方断言 false。窗口先于交互挂上，
  点击瞬间偷发的请求不会漏检。
- 期望值（总条数、页数、搜索命中数）一律从真实响应或渲染数据推导，不写死种子数据
  （旧版写死 "users 38 条"、"henry_gao" 已移除）。

## 现有用例

| 用例 | 守住的回归 |
|---|---|
| 保存本地字段后行值即时刷新 | Popover 编辑闭环（#241） |
| Escape 与取消都只收关不提交 | WCAG dismissible + 不误提交（#241） |
| 定价 tab 露出价格编辑行 | ≤3 tab 结构（#241） |
| 各列表分页条出现性与真实条数一致，翻页只换可见卡片 | 固定页大小 15 + 纯前端切片（不发请求） |
| 稳定后静置不重拉，刷新只 +1 | users effect 自循环（#230） |
| 搜索是纯本地过滤 | 前端过滤不误发请求 |

用例状态：本地 debug 预览手跑的浏览器 smoke；未接 CI，无全绿声明。

## 已知边界

- **写路径不覆盖**：现有用例只断言不写后端的路径（本地字段、Esc/取消、Dialog 开消、
  翻页、搜索）。rename/toggle/delete 真落库需要隔离种子或独立后端，属下一轮。
- **不做 API mock**：与仓库 e2e 哲学一致（真实回路），代价是依赖本地后端活着。
- **卡片计数圈在列表容器内**：查询形如 `users-list > user-card`；列表网格外的元素
  （历史上有原型卡区）不纳入计数。
