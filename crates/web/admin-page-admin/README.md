# `page-admin`

## 文件

顶层：

- `src/lib.rs` — 导出渠道、别名、分组、网络、兑换、订阅、货币和系统页面。
- `src/api.rs` — 管理 API 数据层（渠道/分组/别名/兑换/系统/网络的真实调用）。
- `src/state.rs` — 管理实体列表和编辑状态。
- `src/drawer_write.rs` — 抽屉写操作辅助。

每个 tab 页独占一个 `src/tab-page-<name>/` 目录，目录内按职责分层：

| 文件 | 职责 |
|---|---|
| `page.rs` | 页面入口组件（应用壳层面板，组装 stats / toolbar / list） |
| `stats.rs` | 顶部统计卡组 |
| `toolbar.rs` | 筛选、搜索、批量操作栏 |
| `list.rs` | 列表区（卡片网格或空态/错误态） |
| `shared.rs` | 该 tab 独有的共享类型、常量与文案 |
| `modal.rs` | 该 tab 的编辑弹窗（部分 tab 尚未拆分） |

现有 tab 目录：

- `tab-page-aliases/` — 别名管理：比率语义色卡、定价模式切换（每 token / 每次调用）、四态列表。
- `tab-page-channels/` — 渠道 CRUD（凭据掩码、测试按钮）。
- `tab-page-currency/` — 货币管理面板（定义列表 + 新增/编辑/停用；`kind` 切 points/fiat，fiat 必带 symbol，USD 基准锁 rate=1）。
- `tab-page-entities/` — 实体设置页：分组 / 模型别名 / 渠道三张可折叠卡（`cards.rs` + `channels.rs`；别名卡为演示态本地行）。
- `tab-page-gateway/` — 网关渠道健康面板（实时冷却/慢启动观测，5s 条件轮询）。
- `tab-page-groups/` — 用户组管理页面；编辑入口复用现有弹窗。
- `tab-page-network/` — 网络、代理与探活页面。
- `tab-page-redemptions/` — 兑换码管理。
- `tab-page-subscriptions/` — 订阅套餐管理。
- `tab-page-system/` — 系统设置页。

## 路由入口

管理区各页通过 `apps/admin-web/src/lib.rs` 的 Manage tab 标签数组进入，tab index 与面板匹配臂 `(Section::Manage, <index>)` 一一对应，`#<hash>` 同名映射走 `get_initial_route()`。货币页入口：管理区「货币」tab 或 `#currency` hash（Manage index 9）。可达性回归由 `tests/route_currency_wire.rs` 钉死。

## UI 契约

`specs/ui/*.yaml` 按 tab 记录断言（testid / aria 角色 / 关键文案）。新增或改动 tab 的可交互元素时同步更新对应 yaml；`tests/` 下的契约测试会按 yaml 校验可达性。
