# `gateway-gate`

## 文件

- `src/lib.rs` — 公开 GateChain、各 Gate 和快照类型。
- `src/chain.rs` — 按顺序执行所有准入检查。
- `src/auth.rs` — 提取 Authorization、x-api-key、x-goog-api-key 并认证。
- `src/state.rs` — 检查 Token 启用、过期、IP 和模型白名单。
- `src/quota.rs` — 检查可用额度。
- `src/ratelimit.rs` — 按 Token 或渠道限制请求频率。
- `src/model.rs` — 校验模型许可。
- `src/graylist.rs` — 记录失败并临时阻断异常 key。
- `src/concurrency.rs` — 按渠道获取和释放并发槽。
- `src/snapshot.rs` — 保存 Token、用户和模型授权内存快照。
- `src/error.rs` — 准入错误和 HTTP 状态映射。

