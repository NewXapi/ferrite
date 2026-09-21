# gate

网关准入过滤链：请求在到达 dispatch 选路之前必须逐层通过的检查。

## 职责

按 `chain` 里注册的顺序跑准入 Stage。任一层拒绝即返回对应错误（401/429/403/400），
请求不会进入选路。

## 文件清单

| 文件 | 职责 |
|---|---|
| `src/chain.rs` | 准入链执行器 |
| `src/auth.rs` | Bearer token 鉴权 |
| `src/state.rs` | 渠道/用户启用状态检查 |
| `src/quota.rs` | 额度预估——不足即拒（无 reserve/settle，预估由 metering 侧补） |
| `src/ratelimit.rs` | 限流（RPM / TPM） |
| `src/model.rs` | 模型可用性 / 分组可见性 |
| `src/graylist.rs` | 灰名单 |
| `src/concurrency.rs` | 并发槽位 |
| `src/rewrite.rs` | 请求改写 |
| `src/snapshot.rs` | 准入快照（`adapt.rs` 做 snapshot → chain 的适配） |
| `src/error.rs` | 准入错误 → HTTP 状态映射 |
| `src/lib.rs` | crate 导出面 |

## 依赖

只依赖 `contract`。

## 验收

```bash
cargo check -p gateway-gate
cargo test -p gateway-gate                 # CI
```
