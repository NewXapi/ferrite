# `page-overview`

## 文件

- `src/lib.rs` — 导出 OverviewPanel、ModelsPanel、LeaderboardPanel；声明 errors/leaderboard 模块。
- `src/api.rs` — 总览真实调用（`/api/dashboard`、`/api/log/top`、`/api/log/trend`、`/api/monitor`、`/api/log/errors`）与页面级纯函数（`fmt_usd` 货币折算、sparkline 重切与归一化、增长率三态、份额、时间窗文案、悬浮卡排序折叠）。
- `src/overview.rs` — 用量趋势直方图（含时间窗副标题与双层悬浮卡）、总览统计卡（$ 折算 + 卡内 sparkline）、额度余量/runway 卡、消耗前十模型/用户（行内环比与份额）、数据新鲜度裸时间展示。
- `src/health.rs` — 渠道健康度（`/api/monitor` + `/api/channel`）。
- `src/errors.rs` — 近 24 小时错误卡（`/api/log/errors` 聚合呈现，log_type=5 流水的唯一出口）。
- `src/models.rs` — 模型和模型分布面板。
- `src/leaderboard/` — `mod.rs` 真实用量榜（Tokens/Calls/Quota 三口径，行内环比+份额，双列摊开）；`charts.rs` 三张演示图表卡（均带（演示）徽标）；`insights.rs` 上升/下跌最快双卡与厂商 100% 份额条（厂商按模型名前缀推断）。
