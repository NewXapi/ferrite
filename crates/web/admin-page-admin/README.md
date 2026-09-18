# `page-admin`

## 文件

- `src/lib.rs` — 导出渠道、别名、分组、网络、兑换、订阅、货币和系统页面。
- `src/entities.rs` — 渠道、模型、Token 和路由单元编辑字段。
- `src/groups.rs` — 用户组管理页面；筛选结果存在时在旧卡片网格前展示首条真实分组数据的新卡示例，编辑入口复用现有弹窗。
- `src/gateway.rs` — 网关渠道健康面板(实时冷却/慢启动观测,5s 条件轮询)。
- `src/currency.rs` — 货币管理面板（定义列表 + 新增/编辑/停用；`kind` 切 points/fiat，fiat 必带 symbol，USD 基准锁 rate=1）。
- `src/network.rs` — 网络、代理与探活页面。
- `src/pages.rs` — 管理页导航和各管理面板（订阅页直接对接 `/api/subscriptions`：列表由 store hydrate 灌入，增删改调真实端点并按 key 刷新）。
- `src/state.rs` — 管理实体列表和编辑状态（`SubscriptionView → PlanRow` 映射保证 quota 展示口径不换算）。

## 路由入口

管理区各页通过 `apps/admin-web/src/lib.rs` 的 Manage tab 标签数组进入，tab index 与面板匹配臂 `(Section::Manage, <index>)` 一一对应，`#<hash>` 同名映射走 `get_initial_route()`。货币页入口：管理区「货币」tab 或 `#currency` hash（Manage index 9）。UI 契约见 `specs/ui/currency.yaml`，可达性回归由 `tests/route_currency_wire.rs` 钉死。

