# `page-overview`

## 文件

一个 tab = 一个 `tab-page-*` 目录；目录内文件名不带前缀（照 `admin-page-admin` 的拆分约定）。

- `src/lib.rs` — 导出 OverviewPanel、ModelsPanel、LeaderboardPanel 与 `insights` 子模块；声明三个 tab 目录与 `shared`。
- `src/api.rs` — 总览真实调用（`/api/dashboard`、`/api/log/top`、`/api/log/trend`、`/api/monitor`、`/api/log/errors`）与页面级纯函数（`fmt_usd` 货币折算、sparkline 重切与归一化、增长率三态、份额、时间窗文案、悬浮卡排序折叠）。
- `src/shared.rs` — 跨 tab 共用：模型调色板 `MODEL_COLORS` 与 tokens 紧凑格式 `fmt_raw`。

### `src/tab-page-overview/`

- `page.rs` — 状态 + 两个拉取 effect + 组件组合；`dashboard_stats` 派生统计卡列表。
- `trend.rs` — 用量趋势直方图（时间窗副标题、按模型堆叠、两级悬浮卡）。
- `stats.rs` — 统计卡网格与额度余量/runway 卡。
- `sparkline.rs` — 统计卡底部 12 点迷你面积线（手绘 SVG）。
- `top_lists.rs` — 消耗前十模型/用户榜的行视图与行组件。
- `health.rs` — 渠道健康度（`/api/monitor` + `/api/channel`）。
- `errors.rs` — 近 24 小时错误卡（`/api/log/errors` 聚合呈现，log_type=5 流水的唯一出口）。

### `src/tab-page-models/`

- `page.rs` — 模型卡片网格页（`GET /api/models`，loading / 错误 / 空态）。
- `card.rs` — `ModelCardView` → `StatTabsCard` 的字段映射；后端缺字段渲染「—」占位。

### `src/tab-page-leaderboard/`

- `page.rs` — 排行榜页：状态 + 拉取 effect + 两区块组合。
- `demo_board.rs` — 模型实力榜（演示）区块组合（头牌卡 + 海报阵列 + 三张图表）。
- `rank_board.rs` — 真实用量榜三口径卡（Tokens/Calls/Quota 的取数口径与单卡）。
- `cards.rs` — 演示实力榜的立绘/翻牌卡映射层。
- `charts.rs` — 三张演示图表卡（均带（演示）徽标）。
- `data.rs` — 演示数值层（六维数据与派生，待真实源替换）。
- `insights.rs` — 上升/下跌最快双卡与厂商 100% 份额条（厂商按模型名前缀推断）。
