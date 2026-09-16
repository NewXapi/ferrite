//! aff_code 邀请短码（0013）集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test aff_code -- --ignored`
//!
//! 覆盖验收条件（todo/billing-followups 项 2）：
//! - 存量回填：迁移的 DO 块把 aff_code IS NULL 的行全部填上唯一码；
//! - 唯一索引：重复 aff_code 被库拒绝（0013 的稀疏唯一索引生效）；
//! - 生成与解析 roundtrip：generate_aff_code → resolve_invite_code(短码分支)；
//! - 注册流 mock：带短码的 invite 经 WalletSeedHook 走通归属绑定；
//! - 向后兼容：旧 UUID 链接仍能绑定（resolve 的 UUID 分支先兜底）。
//!
use auth::routes::OnUserRegistered;
use billing::currency::{
    AFF_CODE_LEN, WalletSeedHook, generate_aff_code, random_base62, resolve_invite_code,
};

use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 建 pool + 跑迁移（0013 建列/索引并回填当时存量行）。
async fn make_pool() -> Option<sqlx::PgPool> {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .map_err(|e| eprintln!("skipping: postgres unreachable at {}: {e}", db_url()))
        .ok()?;
    db_bootstrap::run_migrations(&pool)
        .await
        .expect("migrations must apply once PG is reachable");
    Some(pool)
}

/// 建测试用户（aff_code 列缺省 NULL = 模拟 0013 之前的存量用户）。
async fn make_user(pool: &sqlx::PgPool) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, 'ac test', NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("ac_user_{}", key.simple()))
    .execute(pool)
    .await
    .expect("insert user");
    key
}

/// 删用户的归属/奖励/余额/用户行（测试自建用户，互不影响）。
async fn cleanup(pool: &sqlx::PgPool, keys: &[Uuid]) {
    for k in keys {
        sqlx::query("DELETE FROM affiliate_rewards WHERE inviter_key = $1 OR invitee_key = $1")
            .bind(k)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM affiliate_links WHERE inviter_key = $1 OR invitee_key = $1")
            .bind(k)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
            .bind(k)
            .execute(pool)
            .await
            .ok();
        sqlx::query("DELETE FROM auth_users WHERE key = $1")
            .bind(k)
            .execute(pool)
            .await
            .ok();
    }
}

/// 存量回填：0013 的 DO 块逻辑（迁移已在空库上跑过，无法重跑已应用版本，
/// 只能逐字复制 SQL 验证它对「NULL 存量行」的回填行为）。
const BACKFILL_SQL: &str = r#"
DO $$
DECLARE
    rec     RECORD;
    code    TEXT;
    attempt INT;
BEGIN
    FOR rec IN SELECT key FROM auth_users WHERE aff_code IS NULL LOOP
        attempt := 0;
        LOOP
            code := substr(md5(random()::text), 1, 8);
            BEGIN
                UPDATE auth_users SET aff_code = code
                WHERE key = rec.key AND aff_code IS NULL;
                EXIT WHEN FOUND;
            EXCEPTION WHEN unique_violation THEN
                attempt := attempt + 1;
                IF attempt > 16 THEN
                    RAISE EXCEPTION 'aff_code backfill: 16 retries exhausted for user %', rec.key;
                END IF;
            END;
        END LOOP;
    END LOOP;
END $$;
"#;

/// 存量用户回填后 aff_code 全部非空且唯一（验收条件 3）。
#[tokio::test]
async fn backfill_fills_existing_users_uniquely() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let mut users = Vec::new();
    for _ in 0..24 {
        users.push(make_user(&pool).await);
    }
    // 前置断言：这批确实是「无短码的存量态」。
    let null_before: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth_users WHERE key = ANY($1) AND aff_code IS NULL",
    )
    .bind(&users)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        null_before,
        users.len() as i64,
        "fresh test users must start without aff_code"
    );

    sqlx::query(BACKFILL_SQL)
        .execute(&pool)
        .await
        .expect("backfill must run");

    let still_null: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth_users WHERE key = ANY($1) AND aff_code IS NULL",
    )
    .bind(&users)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still_null, 0, "backfill must leave no NULL aff_code behind");

    // 唯一性：非空码的 distinct 数 = 用户数（库唯一索引是该不变量的护栏）。
    let distinct: i64 =
        sqlx::query_scalar("SELECT count(DISTINCT aff_code) FROM auth_users WHERE key = ANY($1)")
            .bind(&users)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        distinct,
        users.len() as i64,
        "backfilled aff_codes must be unique"
    );

    cleanup(&pool, &users).await;
}

/// 重复 aff_code 被库拒绝：0013 的稀疏唯一索引真的生效（不是只在迁移文件里写了）。
#[tokio::test]
async fn duplicate_aff_code_rejected_by_index() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let a = make_user(&pool).await;
    let b = make_user(&pool).await;
    sqlx::query("UPDATE auth_users SET aff_code = 'duptest' WHERE key = $1")
        .bind(a)
        .execute(&pool)
        .await
        .unwrap();
    let err = sqlx::query("UPDATE auth_users SET aff_code = 'duptest' WHERE key = $1")
        .bind(b)
        .execute(&pool)
        .await
        .expect_err("duplicate aff_code must violate the unique index");
    assert!(
        err.to_string().to_lowercase().contains("unique"),
        "expected unique violation, got: {err}"
    );
    // NULL 不受唯一索引约束（稀疏唯一）：多行 NULL 是合法存量态。
    sqlx::query("UPDATE auth_users SET aff_code = NULL WHERE key = $1")
        .bind(b)
        .execute(&pool)
        .await
        .expect("NULL aff_code must be allowed multiple times");
    cleanup(&pool, &[a, b]).await;
}

/// 生成 → 解析 roundtrip：短码分支查表回到 user_key。
#[tokio::test]
async fn generate_then_resolve_roundtrip() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let user = make_user(&pool).await;

    generate_aff_code(&pool, user)
        .await
        .expect("generation must succeed on a NULL user");

    let code: String = sqlx::query_scalar("SELECT aff_code FROM auth_users WHERE key = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!code.is_empty(), "generated code must be non-empty");

    // 短码分支：非 UUID 文本 → 查 auth_users.aff_code。
    assert_eq!(
        resolve_invite_code(&pool, Some(&code)).await,
        Some(user),
        "short code must resolve back to the inviter user_key"
    );
    // UUID 分支（旧链接）仍是纯解析，同一 user_key 直通。
    assert_eq!(
        resolve_invite_code(&pool, Some(&user.to_string())).await,
        Some(user),
        "UUID invite must still resolve without DB"
    );
    // 脏输入静默丢弃，不报错（注册旁路语义）。
    assert_eq!(resolve_invite_code(&pool, Some("no-such-code")).await, None);
    assert_eq!(resolve_invite_code(&pool, Some("")).await, None);
    assert_eq!(resolve_invite_code(&pool, None).await, None);

    // 幂等：重复生成不换码、不报错。
    generate_aff_code(&pool, user)
        .await
        .expect("idempotent re-generate");
    let code2: String = sqlx::query_scalar("SELECT aff_code FROM auth_users WHERE key = $1")
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(code2, code, "existing aff_code must not be overwritten");

    cleanup(&pool, &[user]).await;
}

/// 等待 spawn 出的归属绑定落库（hook 是 fire-and-forget，注册不阻塞）。
async fn wait_for_bind(pool: &sqlx::PgPool, inviter: Uuid, invitee: Uuid) -> bool {
    for _ in 0..100 {
        let bound: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM affiliate_links WHERE inviter_key = $1 AND invitee_key = $2)",
        )
        .bind(inviter)
        .bind(invitee)
        .fetch_one(pool)
        .await
        .unwrap();
        if bound {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

/// 注册流 mock：带短码 invite 的注册经 WalletSeedHook 绑定归属（验收条件 1）。
#[tokio::test]
async fn on_registered_with_short_code_binds_attribution() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let invitee = make_user(&pool).await;
    generate_aff_code(&pool, inviter)
        .await
        .expect("inviter needs an aff_code to be resolvable");
    let code: String = sqlx::query_scalar("SELECT aff_code FROM auth_users WHERE key = $1")
        .bind(inviter)
        .fetch_one(&pool)
        .await
        .unwrap();

    // hook 内部 spawn：生成被邀人短码 + 解析邀请人短码 + 绑定归属，全不阻塞注册。
    WalletSeedHook::new(pool.clone()).on_registered(invitee, Some(&code));

    assert!(
        wait_for_bind(&pool, inviter, invitee).await,
        "short-code invite must bind attribution via the register hook"
    );
    // 被邀人的短码也由同一 spawn 生成（链接面板下次加载即用短码）。
    let invitee_code: Option<String> =
        sqlx::query_scalar("SELECT aff_code FROM auth_users WHERE key = $1")
            .bind(invitee)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        invitee_code.is_some(),
        "hook must also generate the invitee's own aff_code"
    );

    cleanup(&pool, &[inviter, invitee]).await;
}

/// 向后兼容：旧链接的 UUID 形态 invite 仍能绑定归属（验收条件 1）。
#[tokio::test]
async fn on_registered_with_uuid_invite_still_binds() {
    let Some(pool) = make_pool().await else {
        return;
    };
    let inviter = make_user(&pool).await;
    let invitee = make_user(&pool).await;

    WalletSeedHook::new(pool.clone()).on_registered(invitee, Some(&inviter.to_string()));

    assert!(
        wait_for_bind(&pool, inviter, invitee).await,
        "legacy UUID invite must still bind attribution"
    );

    cleanup(&pool, &[inviter, invitee]).await;
}

// ---- 纯逻辑（无 PG）：短码形状与熵 ----

/// 长度恒定 + 字符全落 base62：这是「不可枚举 + URL 安全」的落点，顺带挡住
/// 未来有人把 alphabet 换成含 `&`/`=` 的字符（落地页 query 按 &/= 切分会被劈断）。
#[test]
fn random_base62_shape_stays_in_base62() {
    for _ in 0..128 {
        let code = random_base62(AFF_CODE_LEN);
        assert_eq!(code.len(), AFF_CODE_LEN, "aff_code length must be constant");
        assert!(
            code.bytes().all(|b| b.is_ascii_alphanumeric()),
            "aff_code must stay in base62 alphabet: {code}"
        );
    }
}

/// 6 位 base62 ≈ 568 亿空间，8 个码全相同是生日碰撞的反例；生成器退化成
/// 常量（种子写死/取模错位）在这里被抓住。
#[test]
fn random_base62_has_entropy() {
    let codes: Vec<String> = (0..8).map(|_| random_base62(AFF_CODE_LEN)).collect();
    assert!(
        codes.iter().any(|c| c != &codes[0]),
        "generator must have entropy: {codes:?}"
    );
}
