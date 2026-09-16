# `page-account`

## 文件

- `src/lib.rs` — 导出 KeysPanel、UsageLogsPanel、RewardsPanel。
- `src/api.rs` — 账户页数据来源:全部 `*_api` 真实调用 (走 `client::ApiClient`),覆盖密钥 / 用量 / 用户信息 / 会话 / 设置 / 钱包 / 拉人统计 / 充值开单。
- `src/keys.rs` — 用户 API Key 创建、编辑 (分组/额度/无限额度/过期时间)、列举、删除和状态切换;个人资料区与统计卡。
- `src/sessions.rs` — 会话面板:列表 / 吊销 / 吊销其它设备,当前设备吊销带确认弹窗。
- `src/usage_logs.rs` — 用量日志筛选与分页。
- `src/usage_support.rs` — 呈现层纯函数共用库:时间窗换算、$ 额度格式化 (`QUOTA_PER_USD=500_000≈$1`)、UA 归纳 (`summarize_ua`)、短 ID (`short_key`)、日期 ↔ RFC3339 (UTC 口径)。
- `src/rewards.rs` — 奖励面板:钱包 / 拉人统计 / 兑换码 / 充值开单。
- `tests/` — `edit_token_wire` (编辑密钥 wire 形状 + 日期换算) / `keys_display` (短 ID / 进度百分比) / `sessions_format` (UA 归纳 / 分钟时间) / `invite_link` / `rewards_wire` / `rewards_lists_wire` / `usage_format` / `wire_shapes`。

## 密钥·资料呈现口径 (#206)

- 额度一律 $ 口径 (`fmt_quota`,`500_000 ≈ $1`);KeyCard 有限额走 `used_pct` 进度条 (≥90% 红 / ≥70% 琥珀 / 其余绿),无限额度显示「无限」徽标不渲染进度条。
- 密钥明文只在创建响应出现一次,KeyCard 仅展示掩码 `sk-ab****ef`;**没有「复制掩码」按钮** (复制掩码是废串),复制明文能力只存在于创建成功的 `CreatedKeyView` (`复制明文密钥`)。
- 编辑弹窗 (非抽屉) 的 `group` / `expires_at` 走「留空 = 保持不变」语义:契约 `UpdateTokenRequest` 缺省字段不发 (skip_serializing_if),后端按缺省不改;后端双层 `Option` (跟随用户组 / 永不过期) 在契约层不可表达,故 UI 不提供「清空」。
- 日期换算两端统一 UTC 口径:所选日期 → UTC 当天 23:59:59Z,回显取 UTC 日期段,避免本地时区回环漂移。
- 用户 ID 主键是 `auth_users.key` UUID;资料区短显 (`short_key` 前4…后4) + `copy_value` 复制完整值,完整值挂 `title`。

## 会话面板 (#206)

时间经 `fmt_time_minute` 本地化 (原始值挂 `title`);UA 经 `summarize_ua` 归纳为「浏览器 · OS」短标签 (品牌英文、兜底中文,完整 UA 挂 `title`);吊销当前设备先弹确认 (`role=dialog`,吊销即本机登出),其他设备维持行内直接吊销。

## 奖励面板接线状态 (#179/#187)

真实端点:钱包 `GET /api/user/wallet`、拉人统计 `GET /api/affiliate/overview`、
兑换码 `POST /api/user/topup` (核销入账后刷新钱包)、充值开单
`POST /api/user/topup/orders` (仅建 pending 单,支付 provider 占位,admin settle 后入账)。

上述区块均已接真实端点:充值记录 `GET /api/user/topup/orders`、被邀人 `GET /api/affiliate/invitees`;邀请链接由钱包 `user_key` 现拼 (见 rewards.rs),无需后端链接端点。空数组是正常空态。

交互元素 `data-testid`:`wallet-available`、`wallet-balance-<code>`、`wallet-empty`、
`wallet-skeleton`、`wallet-error`、`topup-currency`、`topup-amount`、`topup-order-submit`、
`topup-order-result`、`topup-order-error`、`topup-code`、`topup-submit`、`topup-result`、
`topup-error`、`affiliate-invite-count`、`affiliate-total-reward`、`affiliate-skeleton`、
`affiliate-error`、`invite-link`、`invite-copy`、`invitee-count`。容器带 `role`+`aria-label`
(钱包 / 拉人统计 region,兑换码充值 / 充值开单 group)。`specs/ui/` 本地契约目录不存在,未落 yaml。
