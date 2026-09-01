//! # observe — 观测聚合域 (center)
//!
//! 原则: **edge 产生原始事件, center 只做聚合**。
//! usage_logs (分区原始表) 由 store 域持有; 本 crate 消费它产出四类视图:
//!
//! | 聚合物 | 粒度 | 参考 |
//! |--------|------|------|
//! | usage_hourly | (user, model, group, channel) × 小时: quota/tokens/请求数 | new-api store_usedata.go |
//! | model_rankings | 周期 (日/周/月): 模型 token 份额 + 与上期对比 | new-api rank_usage.go |
//! | perf_metrics | (model, group) × 时间桶: 请求数/成功率/延迟/TTFT | new-api record_perf.go |
//! | monitor_rollup | (probe, model) × 日: 可用率/延迟分布 | sub2api channel_monitor_daily_rollup |
//!
//! ## 聚合纪律 (来自 wildtoken 的取舍)
//!
//! - 原始 usage_logs 按**月分区**, 保留 N 个月后物理删除 (分区 drop, 非 DELETE);
//! - 聚合写入用 upsert (ON CONFLICT DO UPDATE), 幂等 — 重复上报不翻倍;
//! - 查询侧必须返回 as_of/新鲜度标记, 不伪造实时 (设计文档原则 7)。
//!
//! TODO(#700): 四张聚合表的 DDL 进 store/migrations; 水位线推进策略
//! (对齐 sub2api: watermark 只在成功后前进) 与回填策略 (首启回填 30 天)。
//!
//! TODO(#701): token 估算器的归属 — new-api 把 estimate_tokens 放 usage;
//! 我们放在 metering (请求侧近因) 还是这里 (离线补算)? 倾向 metering,
//! 本 crate 只消费已结算事件。定夺时同步 metering 注释。
