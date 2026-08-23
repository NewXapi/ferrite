# ROADMAP — 待开发剩余任务

生成日期: 2026-08-22 / 更新: 2026-08-23

## 已完成（细节见 F-done-archive.md）

| 波次 | 任务 | 完成提交 |
|---|---|---|
| 基线 | F1-F4 (SSE/错误格式/models/热重载) | d04846a |
| 1 | F1.1 stream 检查, G1 admin 认证 | 6dcafcd |
| 2 | F5.1-F5.3 日志, F6.1 token 创建, F7.1 渠道创建 | 0a17ec9 前各 commit |
| 3 | F6.2/F6.3 token 列表禁用, F7.2/F7.3 渠道列表更新删除 | a44ddf2 |
| 4 | F10.1-F10.4 计费（倍率/预扣/结算/充值） | 84abbab |

**进度**: 20 子任务中 17 已完成。测试 38 全绿零警告。

---

## 测试策略

每个 feature 补测试放 `apps/api/tests/`，按源文件分文件，一个 happy path + 一个错误路径。

---

## 待开发

## F8 — 多协议适配（波次 5-7, 最后一个大重构）

拆 5 个子任务：

### F8.1 — 协议适配 trait
- `ProtocolAdapter` trait: build_url/build_headers/build_body/classify_error
- OpenAI 迁移进 trait；channel_type 决定 adapter；dispatch 记录 channel_type
- 验收：现有 OpenAI 透传行为不变（回归测试保护）

### F8.2 — Claude Messages API 适配
- URL `{base}/v1/messages`, x-api-key + anthropic-version, system 独立字段
- SSE 事件流转换 (`content_block_delta`)
- 错误分类：400 不重试, 429/5xx 重试, 401/403 熔断

### F8.3 — Gemini generateContent 适配
- `{base}/v1beta/models/{model}:generateContent?key=`, contents 数组, candidates 结构
- 流式 streamGenerateContent + SSE

### F8.4 — OpenAI ↔ Claude 协议互转
- 请求/响应体互转 (messages 映射, max_tokens 补默认, usage 字段映射)
- 流式逐 chunk 转换

### F8.5 — 协议路由
- dispatch 按 channel_type 选 adapter; 客户端格式按 body 结构判定
- 格式不匹配触发转换；同格式透传

依赖链：F8.1 → {F8.2, F8.3}; F8.2 → F8.4 → F8.5

注：F10.3 流式 settle 目前跳过（SSE 无 usage），F8 落地后接入。

---

## Deferred

### 路由调度（G3 failover + F9 健康状态机）
new-api 试点验证后再迁入 ferrite。触发条件：单 model 多 channel 需求出现。

### 数据库正式化（sqlx migrate + UNLOGGED 表）
触发条件：kv_store 扫描成瓶颈 / 多副本部署需求 / 事务一致性需求。
