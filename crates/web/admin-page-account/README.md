# admin-page-account

个人中心：密钥、用量日志、会话、奖励、设置（tab 目录化）。

## 文件清单

| 路径 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | 个人中心数据请求 |
| `src/usage_support.rs` | 用量展示辅助 |
| `src/tab-page-keys/` | 密钥 tab：`page` / `key_card` / `new_key_form` / `edit_key_modal` / `delete_key_modal` / `created_key_view` / `profile_item` |
| `src/tab-page-usage-logs/` | 用量日志 tab：`page` / `log_card` / `log_detail_modal` |
| `src/tab-page-sessions/` | 会话 tab：`page` / `session_row` / `confirm_revoke_modal` |
| `src/tab-page-rewards/` | 奖励 tab：`page` / `wallet_section` / `recharges_section` / `topup_section` / `invite_section` / `invitees_section` / `shared` |
| `src/tab-page-settings/` | 设置 tab：`page` / `account_section` / `preferences_section` |

## 依赖

`admin-client`（改名依赖 `client`）+ `contract`。

## 硬约束

必须过 `wasm32` check。

## 验收

```bash
cargo check -p admin-page-account
cargo check --target wasm32-unknown-unknown -p admin-page-account
cargo test -p admin-page-account          # CI（7 个测试文件）
```
