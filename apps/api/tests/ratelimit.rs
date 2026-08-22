/// check_and_increment 需要 PG 连接
/// 跳过，仅作结构验证
#[tokio::test]
async fn check_and_increment_requires_pool() {
    // 需要实际 PG 连接
    // cargo test --test ratelimit -- --ignored
    // 且需要 DATABASE_URL 环境变量
}