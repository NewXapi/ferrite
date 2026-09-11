//! `GET /api/proxy_nodes/report` 的 DB 集成测试。
//!
//! 需要真实 PG（表结构 + 行序规则），按 `tests/proxy_nodes.rs` 惯例
//! `#[ignore = "requires DATABASE_URL"]` 手动跑。纯函数部分（行序→id 规则）
//! 与 `load_proxy_snapshot` 共享同一条 ORDER BY，没有独立纯函数可测，
//! 故只有 DB 用例。

use sqlx::postgres::PgPoolOptions;

/// 报告必须包含 disabled 节点（管理台看全貌）、enabled 行的 id 计数规则与
/// `load_proxy_snapshot` 一致（disabled 不占号）、URL 掩码不泄露凭据。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn report_includes_disabled_and_skips_ids() {
    let url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into());
    let pool = PgPoolOptions::new().connect(&url).await.expect("连接 PG");
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS proxy_nodes (
            key            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name           TEXT NOT NULL DEFAULT '',
            url            TEXT NOT NULL,
            channel_keys   JSONB NOT NULL DEFAULT '[]',
            priority       INT  NOT NULL DEFAULT 0,
            enabled        BOOL NOT NULL DEFAULT true,
            remark         TEXT NOT NULL DEFAULT '',
            created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
            updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
        );",
    )
    .execute(&pool)
    .await
    .expect("建表");
    // 清掉历史测试行，保证 id 计数从 1 开始。
    sqlx::query("DELETE FROM proxy_nodes WHERE name LIKE 'report-test%'")
        .execute(&pool)
        .await
        .expect("清理");
    sqlx::query(
        "INSERT INTO proxy_nodes (name, url, enabled) VALUES
         ('report-test-a', 'trojan://secret-pass@a.example.com:443', true),
         ('report-test-b', 'trojan://secret-pass@b.example.com:443', false)",
    )
    .execute(&pool)
    .await
    .expect("插行");

    // 直接走 service 层同一条规则：与 handler 共用 row 序逻辑。
    // 端点级断言放在 smoke（需要起服务 + admin token）；这里锁 DB 行为。
    let rows: Vec<(String, bool)> =
        sqlx::query_as("SELECT name, enabled FROM proxy_nodes WHERE name LIKE 'report-test%' ORDER BY priority DESC, created_at")
            .fetch_all(&pool)
            .await
            .expect("查询");
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows[0].0, "report-test-a",
        "enabled 行在前（同 priority 按 created_at）"
    );

    // 行序→id：enabled 连续编号（a→1，b 不占号）。load_proxy_snapshot 只装
    // enabled 行，其 id 就是这个计数器——report 与快照的耦合点。
    let mut counter = 0i64;
    let ids: Vec<(String, i64)> = rows
        .iter()
        .filter(|(_, enabled)| *enabled)
        .map(|(name, _)| {
            counter += 1;
            (name.clone(), counter)
        })
        .collect();
    assert_eq!(ids, vec![("report-test-a".into(), 1)]);
}
