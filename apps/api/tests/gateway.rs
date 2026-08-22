/// load_channels 需要 PG 连接
/// 跳过，仅作结构验证
#[tokio::test]
async fn load_channels_requires_pool() {
    // 需要实际 PG 连接
    // cargo test --test gateway -- --ignored
    // 且需要 DATABASE_URL 环境变量
}

/// Gateway 构造
#[test]
fn gateway_constructs() {
    // 需要 PgPool
    // 跳过
}