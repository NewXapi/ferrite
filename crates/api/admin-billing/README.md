# `billing`

商业化域：多货币钱包、兑换码、充值、拉人奖励。

## 文件

- `src/lib.rs` — 模块导出与域边界文档。
- `src/currency.rs` — `CurrencyService`：注册 seed（幂等，仅 `kind='points'`）、折算综合可用值 `available_i64`、货币定义 admin CRUD、`convert(amount, from, to)` 通用换算（内部单位中转）；`BillingErr` 域错误。
- `src/wallet.rs` — `WalletService`：多货币扣费（rate 降序 + FOR UPDATE + 不足 clamp）、兑换码/充值/奖励三个入账入口（ON CONFLICT 叠加）、余额视图。
- `src/redeem.rs` — 兑换码生成与核销（CAS）；入账走 `WalletService::credit_redeem_in_tx`（`user_balances(FREE)`，不再写 `auth_users.quota`）。
- `src/topup.rs` — `TopupService`：开单 pending + admin 手工 settle（CAS pending→settling→paid 幂等）；`TopupProvider` 渠道抽象（开单/回调验签）+ provider 注入表（默认 `ManualProvider`，验签恒拒——manual 单只能走 admin settle 入金）；`POST /api/topup/webhook/{provider_id}` 无鉴权端点，验签即鉴权，重放已入账单据回执 200 + 已入账额。
- `src/affiliate.rs` — `AffiliateService`：邀请归属绑定（自邀请拒绝、一人一主）、invite 领奖（关系校验 + DB 级幂等，入账与审计行同事务）、真实统计（`user_overview`）；金额读 `options` 表 `site.affiliate_reward`，回落常量。

## 表

- `currency_defs(code PK, name, internal_rate, enabled, symbol, kind, precision)` — 货币注册表，seed `FREE`(points) / `USD`(fiat 基准 rate=1) / `CNY`(fiat)；`kind` = `points`(可扣费余额) | `fiat`(仅计价展示)
- `user_balances(user_key, currency_code, amount)` — PK 复合，稀疏桶
- `billing_topups(key PK, user_key, currency, amount, state, provider)` — 充值订单
- `affiliate_links(invitee_key PK, inviter_key)` — 邀请归属，一人一主防重复绑定/领奖
- `affiliate_rewards(key PK, inviter_key, invitee_key, kind, amount)` — 奖励入账审计；局部唯一索引 `(invitee_key) WHERE kind='invite'` 做领奖幂等护栏
- 迁移：`db/migrations/0007_currency_wallet.sql`、`0008_topups.sql`、`0010_affiliate_links.sql`、`0014_currency_display.sql`

## 边界

- 网关计量零改动：`UsageEventRecord.cost: i64` 是唯一货币无关输出，货币换算全在本域（`available_i64 = Σ amount × internal_rate`，仅 `kind='points'` 计入）
- 换算基准是内部单位（500_000 = $1，对齐 `pricing.rs`）：`internal_rate` = 1 该货币单位值多少内部单位，任意两货币经内部单位中转换算。`USD` 是基准本身，`internal_rate` 锁 1（`upsert_def` 拒绝改动）；`fiat` 货币必带 `symbol`、`precision ≥ 1`，且不进 `user_balances`、不参与 seed/扣费
- settle 闭环已接（`PgSettleSink` → `deduct_by_cost`，#187）；邀请码字符串 → `inviter_key` 解析待 auth 域 `aff_code` 列（#188），`bind_inviter` 只收已解析 UUID
- 支付协议与账务解耦：provider 实现只管外部协议（`create`/`verify_callback`），落库/入金只归 `TopupService`；真接渠道（epay/stripe）= 新增 impl + `with_provider` 注入，webhook/settle 幂等链路零改动（独立 PR）
