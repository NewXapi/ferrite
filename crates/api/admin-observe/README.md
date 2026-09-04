# `observe`

## 文件

- `src/lib.rs` — 模块地图与 Freshness 标记（聚合域设计态说明）。
- `src/logs.rs` — **已实现**：`usage_logs` 平表（BIGSERIAL，log_type 1=topup
  2=consume 3=manage 4=system），`LogService::record(&UsageEvent)` 供网关写入，
  admin/self 查询（log_type/username/token_name/model_name/时间范围 + 分页）、
  stat（今日 quota/requests + 60s 窗口 rpm/tpm）、`/api/dashboard` 汇总端点（5 路由）。
- `src/monitor.rs` — **已实现**：`monitor_history` 平表，`record_probe`/
  `history`/`availability`/`availability_all`，`MonitorDeps` 封装 +
  admin 路由 `/api/monitor/{key}`、`/api/monitor`（2 路由）。
  探活执行在 `catalog::channels::test_channel`（reqwest 真调上游）。
- `src/hourly.rs` — 小时聚合 upsert（**骨架**，SQL 形状已写，等数据量）。
- `src/rankings.rs` — 模型排行（**骨架**）。
- `src/perf.rs` — TTFT/延迟/成功率聚合（**骨架**）。
- `src/retention.rs` — 分区 drop/批量清理（**骨架**）。

## 表（loose，无 FK）

- `usage_logs` — id(BIGSERIAL)/log_type/user_key/username/token_key/token_name/
  channel_key/channel_name/model_name/prompt_tokens/completion_tokens/quota/
  use_time_ms/is_stream/ip/request_id/content/created_at
  索引：created_at、(user_key, created_at)、token_name、model_name
- `monitor_history` — id(BIGSERIAL)/channel_key/channel_name/model/ok/status_code/
  latency_ms/error_kind/message/created_at，索引 (channel_key, created_at)

## 路由

| 方法 | 路径 | 鉴权 |
|------|------|------|
| GET | `/api/log`、`/api/log/stat` | admin |
| GET | `/api/log/self`、`/api/log/self/stat` | user |
| GET | `/api/dashboard` | admin |
| GET | `/api/monitor/{key}`、`/api/monitor` | admin |

## 验证

```sh
DATABASE_URL=postgres://<user>:<pass>@127.0.0.1:5433/<db> \
    cpulimit -l 70 -i -- cargo test -p observe --tests -- --include-ignored
cargo clippy -p observe --all-targets
```
