# `page-overview`

## 文件

一个 tab = 一个 `tab-page-*` 目录；目录内文件名不带前缀（照 `admin-page-admin` 的拆分约定）。
第二轮又在每个目录内按「页面 / 区块 / 组件 / 共享」再切一层：中文文案收进各 tab 的
`shared.rs`，`rsx!` 里聚合度高的渲染块抽成独立组件。

- `src/lib.rs` — 导出 OverviewPanel、ModelsPanel、LeaderboardPanel 与 `insights` 子模块；声明三个 tab 目录与 `shared`。
- `src/api.rs` — 总览真实调用（`/api/dashboard`、`/api/log/top`、`/api/log/trend`、`/api/monitor`、`/api/log/errors`）与页面级纯函数（`fmt_usd` 货币折算、sparkline 重切与归一化、增长率三态、份额、时间窗文案、悬浮卡排序折叠）。
- `src/shared.rs` — 跨 tab 共用：时间窗档位 `TIMEFRAMES`、`TimeframeTabs` 组件、模型调色板 `MODEL_COLORS` 与 tokens 紧凑格式 `fmt_raw`。

### `src/tab-page-overview/`

- `page.rs` — 状态 + 两个拉取 effect + 组件组合；`dashboard_stats` 派生统计卡列表。
- `shared.rs` — 本 tab 文案常量（i18n）与共享类型 `TrendTip` / `StatCardView` / `QuotaView`。
- `trend.rs` — 用量趋势面板骨架（标题 + 时间窗 + 四态分支 + 组合）。
- `histogram.rs` — 趋势左栏堆叠直方图（Y 轴虚网格 + 柱 + 两级悬浮事件 + X 轴标签）。
- `summary.rs` — 趋势右栏数据位四宫格 + 主力模型 Top5 图例。
- `tooltip.rs` — 两级悬浮卡（通用外框 + 整列/单段两种卡体）。
- `stats.rs` — 统计区外壳（区头 + asOf）与统计卡、额度余量/runway 卡。
- `sparkline.rs` — 统计卡底部 12 点迷你面积线（手绘 SVG）。
- `top_lists.rs` — 消耗前十模型/用户榜的行视图、行组件与共用卡外壳。
- `health.rs` — 渠道健康度（`/api/monitor` + `/api/channel`）。
- `errors.rs` — 近 24 小时错误卡（`/api/log/errors` 聚合呈现，log_type=5 流水的唯一出口）。

### `src/tab-page-models/`

- `page.rs` — 模型卡片网格页（`GET /api/models`，loading / 错误 / 空态）。
- `card.rs` — `ModelCardView` → `StatTabsCard` 的字段映射；后端缺字段渲染「—」占位。
- `shared.rs` — 本 tab 文案常量（页头、四态、卡面标签与状态值）。

### `src/tab-page-leaderboard/`

- `page.rs` — 排行榜页：状态 + 拉取 effect + 三区块组合。
- `shared.rs` — 本 tab 文案常量（i18n）与 `slug` 纯函数。
- `toolbar.rs` — 真实用量榜工具条（标题 + 口径副标题 + 时间窗切换）。
- `demo_board.rs` — 模型实力榜（演示）区块组合（头牌卡 + 海报阵列 + 三张图表）。
- `rank_board.rs` — 真实用量榜三口径卡（Tokens/Calls/Quota 的取数口径与单卡）。
- `cards.rs` — 演示实力榜的立绘/翻牌卡映射层。
- `charts.rs` — 三张演示图表卡（均带（演示）徽标）。
- `data.rs` — 演示数值层（六维数据与派生，待真实源替换）。
- `prev_window.rs` — 上一等长窗起点推导与 `[start, end)` 区间取数 `top_usage_between`。
- `movers.rs` — 名次变动纯判定 + 上升/下跌最快双卡。
- `vendors.rs` — 厂商前缀推断 + 份额聚合 + 100% 堆叠条卡。
- `insights.rs` — 洞察区公开门面：再导出上述三个模块，保持 `admin_page_overview::insights::*` 路径（页面与测试依赖）。
