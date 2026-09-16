# `page-overview`

## 文件

- `src/lib.rs` — 导出 OverviewPanel、ModelsPanel、LeaderboardPanel。
- `src/api.rs` — 总览真实调用（`/api/dashboard`、`/api/log/top`、`/api/log/trend`、`/api/monitor`）与模型/排行榜数据请求。
- `src/overview.rs` — 用量趋势直方图、总览统计卡、消耗前十模型/用户。
- `src/health.rs` — 渠道健康度（`/api/monitor` + `/api/channel`）。
- `src/models.rs` — 模型卡片网格（`GET /api/models`，loading / 错误 / 空态，后端缺字段渲染「—」占位）。
- `src/leaderboard.rs` — 用户、模型和渠道排行面板。

