# contract — 全 workspace 逻辑契约层

DTO / records / 错误码 / schema 的唯一事实来源。

## 职责

所有跨进程、跨端传输的数据形状都定义在这里。**不依赖** tokio/sqlx/axum/reqwest/
dioxus，必须过 `wasm32` check。SQL DDL 与存储编码**不在**本 crate。

## 硬约束（改 DTO 前必读）

- 所有 DTO 一律 `#[serde(rename_all = "camelCase")]`
- 列表端点统一包 `{"items": [...], "total": N}`；auth 系返回裸 JSON
- 计费单位 `500000 = ¥1`
- 状态词表：token/channel `1=启用 2=停用`；兑换码 `1=未用 2=停用 3=已核销`；`logType 2=消费`
- 错误语义：`AuthError::Jwt(_)` 必须 401，`Db/Crypto/Internal` 才是 500
- 已锁定的字段错位（有回归测试，改动前先读测试）：`UserDto.role` 是整数；
  `UserDto.requestCount` 后端不下发；`TokenDto` 列表字段直名 `key_preview`；
  创建 token 响应嵌套 `{plaintext, token}`；`UsageLogDto` 无 success/cost 列；
  兑换码 `DELETE = 停用`
- DTO 唯一事实来源是各 admin-api crate 的 `*View` 结构体；改后端序列化必须同步本
  crate，**先用真实抓包 JSON 写回归测试再改实现**
- 判断「某修复是否已在 main」必须 `git merge-base --is-ancestor`，`git log --all
  --grep` 会误报

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/lib.rs` | crate 导出面 |
| `src/api.rs` | API 层 DTO 汇总 |
| `src/api/admin.rs` | 管理面 DTO |
| `src/api/auth.rs` | 认证 DTO（login/register/refresh/self） |
| `src/api/billing.rs` | 计费 DTO |
| `src/api/token.rs` | Token DTO |
| `src/api/usage.rs` | 用量 DTO |
| `src/api/user.rs` | 用户 DTO |
| `src/records.rs` | records 汇总 |
| `src/records/billing.rs` | 计费记录 |
| `src/records/channel.rs` | 渠道记录 |
| `src/records/identity.rs` | 身份记录 |
| `src/records/routing.rs` | 路由记录 |
| `src/records/usage.rs` | 用量记录 |
| `src/error.rs` | 错误码词表 |
| `src/mutations.rs` | 变更请求形状 |
| `src/schema.rs` | schema 常量 |

## 验收

```bash
cargo check -p contract
cargo check --target wasm32-unknown-unknown -p contract
cargo test -p contract                     # CI（currency_records / login_response_shape / usage_wire）
```
