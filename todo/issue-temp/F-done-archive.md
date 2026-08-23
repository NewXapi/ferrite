# Ferrite 已完成任务归档（F1-F10.4）

> 2026-08-22 初版 / 2026-08-23 更新（并入波次 2/3/4）。本存档记录所有已完成任务；ROADMAP.md 只保留待开发。

## 基础层 (F1-F4)

| 编号 | 功能 | 完成日期 | 说明 |
|---|---|---|---|
| F1 | Stream SSE 转发 | 基线 | adapter.rs/gateway.rs |
| F2 | OpenAI 格式错误响应 | 基线 | gateway.rs |
| F3 | /v1/models | 基线 | gateway.rs |
| F4 | 渠道热重载 | 基线 | kv_store→ArcSwap |

## 波次 1：代码修正 + 唯一前置

| 编号 | 功能 | 说明 |
|---|---|---|
| F1.1 | Stream 状态前置检查 | ensure_stream_ok。非 2xx 读 body (max 256KiB) → AdapterError::Upstream |
| G1 | 管理面认证 | is_admin + require_admin |

## 波次 2：最小可用

| 编号 | 功能 | 说明 |
|---|---|---|
| F5.1 | 日志滚动 JSON 文件 | tracing-appender daily rolling，零自研 |
| F5.2 | TraceLayer + span record | business fields 声明 Empty，handler 内 record |
| F5.3 | GET /admin/logs | 读 logs/ferrite.log.*，filter_log_lines 纯函数 |
| F6.1 | POST /admin/tokens | sk-+32hex key，user_id max+1，deny_unknown_fields |
| F7.1 | POST /admin/channels | name 唯一、channel_type 限定、base_url/keys/models 校验，id=timestamp_millis，kv_store channel:{id} |

## 波次 3：管理面补全（2026-08-23）

| 编号 | 功能 | 说明 |
|---|---|---|
| F6.2 | GET /admin/tokens | 超集拉取 + 内存过滤 (user_id/enabled) + 分页；key 掩码前 8 位 + ... |
| F6.3 | DELETE /admin/tokens/:key | 软删 enabled=false，幂等 204，不存在 404 |
| F7.2 | GET /admin/channels | kv_store channel:%，channel_type 过滤，keys 掩码 |
| F7.3 | PUT/DELETE /admin/channels/:id | UpdateChannelReq 全 Option (deny_unknown_fields)，merge_channel_config 合并 + 改名查重 + 全量 validate |

E2E: F6.2 掩码+过滤+分页+403；F6.3 204→403→幂等204→404；F7.2 掩码+过滤；F7.3 PUT-reload 可路由/409/400/404。
Review: FAIL → PASS（mask_key 短 key 泄露修复 → "***"）。

## 波次 4：计费系统（2026-08-23, billing.rs 新建）

| 编号 | 功能 | 说明 |
|---|---|---|
| F10.1 | 倍率配置 | ModelPricing {input_per_1k, output_per_1k, multiplier} 存 kv_store `pricing:{model}`；tokens_to_quota 未配置 1:1 / 已配置 ceil((p*in+c*out)*mult/1000) + f64 饱和护栏 |
| F10.2 | 预扣 | reserve_quota 原子 UPDATE...RETURNING，剩余 ≥1000 才扣，None → 402 insufficient_quota |
| F10.3 | 结算 | settle_quota delta = actual - reserve，GREATEST 防负数；仅上游 2xx 且预扣成功才结算 |
| F10.4 | 充值 | POST /admin/recharge {token_key, amount}；amount<=0 → 400，不存在 → 404 |

决策 C：上游失败预扣消耗不回滚（tracing::warn billing_reserve_consumed）。
E2E 计费链路：预扣 1000 → usage(100,50) → 结算退 999 留 actual=1；quota=500 → 402；充值 1000 → quota=1500 → 再请求 200。
Review: FAIL → PASS（settle 顺序颠倒、reserve 失败伪退款、f64 溢出、multiplier=0 免费模型 四项修复）。

## 关键技术决策（全量）

1. **日志统一走 tracing**：`tracing + tracing-subscriber + tracing-appender + tower-http TraceLayer + tracing-log`。不入 PG kv_store（用户否决）。
2. **零自研日志设施**：全用现成 crate。
3. **随行情怀**：`ponytail:` 注释标记所有已知 ceiling。
4. **`cpulimit -l 60 -i --`**：包裹所有重命令。
5. **设计原则**：无 sqlx migrate；k/v 用 kv_store (jsonb)；lazy。
6. **lib/bin 拆拆** 自 F5 之后完成，`pub mod` 导出供测试。

## 遗留 issue（From review）

| 问题 | 固定决策 | 原因 |
|---|---|---|
| `ensure_stream_ok` 截断在 256KiB | 保持 | 超限错误信息被截断但无害 |
| `list_logs` 100k 行扫描上限 | 保持 | 日志成问题→升到 Loki |
| TraceLayer SSE 无法测 on_eos | 不补 | SSE 无 trailers，协议限制 |
| Rust 2024 `gen` keyword | 改用 `rand::random::<[u8;16]>()` | 新版编译错误 |
| mask_key 短 key 原样返回 | 改 "***" | list_channels 会泄露上游短密钥 |
| delete_token 重复删除 404？ | 实测 204 幂等 | PG rows_affected 按 WHERE 命中行计 |
| settle 顺序/伪退款/f64 溢出/multiplier=0 | 已修 (84abbab) | 见波次 4 记录 |

## 文件清单（当前）

```
apps/api/
  src/lib.rs         # pub mod 导出
  src/main.rs        # 入口：telemetry → PG → RouteIndex → serve
  src/config.rs      # TOML 配置
  src/dispatch.rs    # RouteIndex / load_channels / ChannelConfig
  src/gateway.rs     # HTTP handler (F1-F10.4 路由+函数)
  src/identity.rs    # Pass / require_admin / authenticate
  src/adapter.rs     # OpenAIAdapter / ensure_stream_ok / forward_stream
  src/billing.rs     # F10: pricing/reserve/settle (波次 4)
  src/ratelimit.rs   # 固定窗口限流
  tests/adapter.rs   # ensure_stream_ok / forward_stream mock 测试
  tests/dispatch.rs  # RouteIndex 单测
  tests/gateway.rs   # token/channel/billing 纯函数 + deny_unknown_fields
  tests/identity.rs  # require_admin happy/fail 路径
config/config.toml
todo/issue-temp/F-done-archive.md  # 本文档
todo/issue-temp/ROADMAP.md          # 待开发剩余
```
