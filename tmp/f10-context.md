# F10.2 + F10.3 实现上下文

## 目标
波次 4 的两个任务，同文件簇，一起派：
- **F10.2 预扣**：请求前占额度 (`tokens.used_quota += estimate WHERE key=$1 AND quota-used_quota>=estimate`)，0行→402
- **F10.3 结算**：响应后按实际 token 数校正差额（settle = actual - reserve）

## 全程要改的 only 文件
1. `apps/api/src/billing.rs` （继续）
2. `apps/api/src/gateway.rs`(`chat_completions` 函数内嵌调用）
3. `apps/api/src/lib.rs`（无需改，已经 pub mod billing)

## 已存在（F10.1 完成的 billing.rs)
- `pub struct ModelPricing { input_per_1k: f64, output_per_1k: f64, multiplier: f64 }`
- `pub async fn read_pricing(pool, model) -> Result<Option<ModelPricing>, sqlx::Error>`
- `pub async fn write_pricing(...)`
- `pub fn tokens_to_quota(prompt, completion, Option<&ModelPricing>) -> u64`
- `pub fn validate_pricing(...) -> Result<(), String>`

## 流位：chat_completions 框架

```rust
// ratelimit 通过后 (已存在)
// 1. 读取 pricing（失败不阻断，更旧：None → 1:1）
let pricing = billing::read_pricing(&state.pool, model).await.ok().flatten();

// 2. 计算 reserve = 1000 tokens 的口径（FIX_ESTIMATE = 1000u64）
// 3. 预扣 SQL:
let row: Option<(i64,)> = sqlx::query_as(r#"
    UPDATE tokens SET used_quota = used_quota + $1
    WHERE key = $2 AND (quota - used_quota) >= $1
    RETURNING used_quota
"#) ...
// None → Err(402 ... "insufficient_quota")

// 4. 转发 + 检查（已有 tracing-span fill usage）

// 5. 结算（收到 usage 后）:
//    if let Some(usage) = usage { tokens_to_quota(p, c, pricing) → 差额 = actual as i64 - estimate
//      SQL: UPDATE tokens SET used_quota = used_quota - $差额$.max(0) WHERE key=...
//      负差额 → 不会执行.jpg （0 行为
//    }

// P2 注意事项 （reveiw 预期十字架):
// - 响应后、流断前完成 settle；如果这个请求未完成（on_eos 未触发）Reserve 不退
// - 上行 5xx: 决策 C（失败消耗 reserve） 不退款，但需要一致性注释 + trace event
// - span.record("prompt_tokens", ...) 已存在
```

## 测试
纯函数：

1. reserve 固定 1000: `RESERVE_ESTIMATE`. const = 1000
2. tokens_to_quota 已有测试
3. 建议写 `estimate_tokens` 纯函数 = 总是 1000 （将来估计在 KV_STORE 经历后面接，这是 policy seam)
4. 错误路径： 量和修改计算 （实际 < reserved，应检，**结算失败，错误不要被吞**)

测试不要写 PG 依赖 (kv_store 读写留给 E2E)，单测只测预算/댄충.

## 上路避免踩到 F6/F7 的坑
- Pyro 模式：所有 admin handler 需要 authenticate_request → require_admin
- error_response(s, msg, ty) 统一，上面写着 decision C
- 别让 settle 的 UPDATE 失败变成 500 抛出下层 （响应已发），essaging via tracing::warn
- SQL UPDATE 都是固定参数化

## 完成标准 = 测 bonus: 6 测试全绿包括 (texttt{cargo test --lib billing/ascr},新加的 2 个）

有错误要把问题返回来，立即返回，不准尝试悄悄绕过 githooks。不要 push 到分支。
