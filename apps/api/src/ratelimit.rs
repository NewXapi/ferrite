//! 限流：基于 PG UNLOGGED rate_buckets 的固定窗口计数

use crate::config::PgPool;
use crate::identity::Pass;
use axum::http::StatusCode;

/// 限流配置：每分钟最多 N 次请求
const WINDOW_SECONDS: i64 = 60;
const MAX_REQUESTS: i64 = 20;

/// 检查并递增计数器。返回 Ok 允许，Err 拒绝。
pub async fn check_and_increment(pool: &PgPool, pass: &Pass) -> Result<(), (StatusCode, String)> {
    let now_secs = chrono::Utc::now().timestamp();
    let window_start = now_secs - (now_secs % WINDOW_SECONDS);

    // ponytail: UPSERT + 条件检查在一条 SQL 里完成，避免两步竞态
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        WITH upserted AS (
            INSERT INTO rate_buckets (token_key, window_start, count)
            VALUES ($1, $2, 1)
            ON CONFLICT (token_key, window_start)
            DO UPDATE SET count = rate_buckets.count + 1
            RETURNING count
        )
        SELECT count FROM upserted
        "#,
    )
    .bind(&pass.token_key)
    .bind(window_start)
    .fetch_optional(pool)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("rate limit db error: {e}"),
        )
    })?;

    let count = row.map(|r| r.0).unwrap_or(0);

    if count > MAX_REQUESTS {
        tracing::warn!(
            token_key = %pass.token_key,
            user = %pass.username,
            count,
            max = MAX_REQUESTS,
            "rate limited"
        );
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            format!("rate limit exceeded: {count}/{MAX_REQUESTS} per {WINDOW_SECONDS}s"),
        ));
    }

    tracing::debug!(
        token_key = %pass.token_key,
        count,
        max = MAX_REQUESTS,
        "rate limit ok"
    );

    Ok(())
}
