# ROADMAP — 待开发剩余任务

生成日期: 2026-08-22 / 更新: 2026-08-23

## 已完成（见 F-done-archive.md）
- F1.1, G1, F5.1, F5.2, F5.3, F6.1, F7.1

## 待开发
- F6.2, F6.3, F7.2, F7.3 （管理面补全）
- F8.x (多协议）
- F10.x （计费）

---

## 测试策略

每个 feature 实现后补测试代码，放在 `apps/api/tests/` 目录下，按源文件分文件。
每个文件一个 happy path + 一个错误路径。

---

## 1. F6 — Token 管理 API（剩余）

全部需 G1。依赖：已完成部分。

### F6.2 — GET /admin/tokens（列表）

**范围**: 列出 token。

**实现**: `SELECT key, user_id, username, quota, used_quota, "group", enabled, created_at, is_admin FROM tokens LIMIT $1 OFFSET $2`；key 显示掩码 `sk-xxx…`; `user_id`, `enabled` query 过滤。

**返回**: `{"object":"list","total":N,"data":[...]}`（与 /admin/logs 风格一致）。

**验收**:
- 创建 3 个 token → list 应含它们
- `?enabled=false` → 只有禁用
- `?user_id=X` → 过滤正确
- key 显示掩码（前 8 位 + `...`)
- 分页：`limit=1`→total 正确；`limit=999`→clamp 500

**测试**:
1. 纯函数 `token_query_filters()` 生成 SQL 注入参数（user_id/enabled/bind 顺序）
2. E2E（见验收）

**依赖**: G1

---

### F6.3 — DELETE /admin/tokens/:key（软删除）

**范围**: 禁用（不是删除）。

**实现**: `UPDATE tokens SET enabled = false WHERE key = $1 RETURNING id` → 200 / 404 NotFound。identity.rs authenticate 已检查 enabled=false → 403，无需改。

**验收**:
- 创建 token → 用新 token 请求 /v1/models 200
- DELETE → 204（axum 惯例）
- 同一 token → 403
- 再 DELETE 同 key → 404

**测试**:
1. E2E（见验收）
2. DELETE 异常：不存在的 key → 404；非 admin → 403

**依赖**: F6.1, G1

---

## 2. F7 — 渠道管理 API（剩余）

全部需 G1。渠道存 `kv_store`	key `channel:{id}`,value json 已序列化为 ChannelConfig。

### F7.2 — GET /admin/channels（列表）

**范围**: 列出所有渠道。

**实现**: `SELECT key, value FROM kv_store WHERE key LIKE 'channel:%'` → filter_map 反序列化为 `ChannelConfig` → 数组返回。`channel_type` query 过滤。

**返回**: `{"object":"list","total":N,"data":[{ id, name, base_url, channel_type, keys: ["sk-…"], models: [...] }]}`（`keys` 掩码）

**验收**:
- 创建 2 渠道 → list 应含
- `?channel_type=openai` → 只有 openai
- key 掩码验证
- 空 kv_store → total=0

**依赖**: G1

---

### F7.3 — PUT/DELETE /admin/channels/:id（更新+删除）

**范围**: 更新 + 删除。

**PUT 实现**:
- `SELECT value FROM kv_store WHERE key = 'channel:{id}'` → 反序列化 ChannelConfig
- 应用 merge: 传什么改什么（None=保持不变）。重新 validate_channel（用合并后的结果）
- PUT 与 F7.1 字段结构共用（避免重复定义）：`CreateChannelReq` 字段全 Option
- kv_store UPDATE

**DELETE 实现**:
- `DELETE FROM kv_store WHERE key = 'channel:{id}'` → 204 / 404

**验收**:
- POST 创建 → PUT 修改 models → reload → 新 alias 出现在 /v1/models
- PUT 错误字段 → 400
- DELETE → reload → alias 消失
- DELETE 不存在 → 404

**安全**:
- 从 input JSON 到上游调用没有 shell（只进 PG jsonb)
- deny_unknown_fields 均有

**测试**:
1. `merge_channel_config()` 纯函数：传入 ChannelConfig + 部分 JSON → 验证结果
2. E2E

**依赖**: F7.1, G1

---

## 3. F8 — 多协议适配（下一个波次）

见 F-done-archive 已完成的 F1-F7.1。待开发主体在 §4。

## 4. F10 — 计费系统（下一波次之后）

见 ROADMAP-deferred。

---

## 5. 计划顺序（推荐执行波次）

| 波次 | 子任务 | 状态 |
|---|---|---|
| 3 | F6.2, F6.3, F7.2, F7.3 | 进行中 |
| 4 | F10.1-F10.4 | 待开发 |
| 5-7 | F8.x | 待开发 |
| 8+ | 路由调度，数据库正式化 | deferred |

**依赖关系**:
- F6.2/F6.3 独立
- F7.2/F7.3 依赖 F7.1（已完成）
- F10.2 依赖 F5.2(token 使用日志）
- F10.3 依赖 F10.2
- F8.5 依赖所有协议

---

## 6. Deferred — 路由调度与数据库正式化（不变）

见 §7（同上打包）。触发条件保持。

---

## 7. 子任务总表

| 编号 | 状态 |
|---|---|
| F1.1, G1, F5.1, F5.2, F5.3, F6.1, F7.1 | ✅ 完成 |
| F6.2, F6.3, F7.2, F7.3 | 待开发 |
| F8.1-F8.5 | 待开发 |
| F10.1-F10.4 | 待开发 |

**Active**: 15 子任务 / 5 已完成。
