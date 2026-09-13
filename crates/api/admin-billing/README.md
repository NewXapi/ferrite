# `billing`

商业化域：多货币钱包、兑换码、充值、拉人奖励。

## 文件

- `src/lib.rs` — 模块导出与域边界文档。
- `src/currency.rs` — `CurrencyService`：注册 seed（幂等）、折算综合可用值 `available_i64`、货币定义 admin CRUD；`BillingErr` 域错误。
- `src/wallet.rs` — `WalletService`：多货币扣费（rate 降序 + FOR UPDATE + 不足 clamp）、兑换码/充值/奖励三个入账入口（ON CONFLICT 叠加）、余额视图。
- `src/redeem.rs` — 兑换码生成与核销（CAS）；入账走 `WalletService::credit_redeem_in_tx`（`user_balances(FREE)`，不再写 `auth_users.quota`）。
- `src/topup.rs` — `TopupService`：开单 pending（provider 占位）+ admin 手工 settle（CAS pending→paid 幂等）。
- `src/affiliate.rs` — `AffiliateService`：拉人奖励入 FREE（金额读 `options` 表 `site.affiliate_reward`，回落常量）；统计占位待 affiliate_links 表。

## 表

- `currency_defs(code PK, name, internal_rate, enabled)` — 货币注册表，seed `FREE`
- `user_balances(user_key, currency_code, amount)` — PK 复合，稀疏桶
- `billing_topups(key PK, user_key, currency, amount, state, provider)` — 充值订单
- 迁移：`db/migrations/0007_currency_wallet.sql`、`0008_topups.sql`

## 边界

- 网关计量零改动：`UsageEventRecord.cost: i64` 是唯一货币无关输出，货币换算全在本域（`available_i64 = Σ amount × internal_rate`）
- settle 闭环（`PgSettleSink` → `deduct_by_cost`）与余额快照源切换属后续 PR（见 todo/billing-implementation.md T3）
