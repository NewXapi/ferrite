//! billing::affiliate 被邀人列表测试 — `GET /api/affiliate/invitees`。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p billing --test invitees_list`
//!
//! 覆盖场景：
//! - DTO 序列化形状（离线全跑，无 PG）：InviteeView → camelCase JSON，键名
//!   与前端 wire.rs 镜像对账——任一字段改名会让奖励面板渲染 undefined。
//! - SQL 形状对账（离线全跑，无 PG）：list_invitees 的查询串含契约要求的
//!   三表 JOIN + GROUP BY + ORDER BY DESC + LIMIT。
//! - 真实列表行为（PG，不可达自动 skip）：绑 2 个被邀人 + 对其中 1 人发奖，
//!   断言倒序 / name 回落 / reward 求和 / 不泄漏他人的被邀人。
//!
//! skip 而非 #[ignore]：CI 跑 `cargo test -p <pkg>` 不带 --ignored（见
//! scripts/ci-affected.sh），#[ignore] 在 CI 上等于没写；skip 模式让有 PG
//! 的环境真跑断言、无 PG 的环境静默跳过。与 topup_affiliate.rs 同款约定。

use billing::{AffiliateService, WalletService};
use contract::api::billing::InviteeView;
use serde_json::json;
use sqlx::postgres::PgPoolOptions;
use sqlx::types::chrono::{DateTime, FixedOffset};
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

/// 被测源码（编译期 include：路径错位直接编译失败，而非测试时才炸）。
const AFFILIATE_SRC: &str = include_str!("../src/affiliate.rs");

/// InviteeView 序列化形状：camelCase 键 + reward 为数字 + joinedAt 为字符串。
///
/// 对账点：前端 wire.rs 的 InviteeView 镜像按 userKey/name/joinedAt/reward
/// 取值；DTO 的 `#[serde(rename_all = "camelCase")]` 是唯一保证两侧同步的
/// 机制，这里钉死形状——防有人把字段名改成 snake_case，或把 reward 改成
/// Option/字符串（前端做数字格式化，字符串会让它变 NaN 或拼接）。
#[test]
fn invitee_view_dto_shape() {
    let v = InviteeView {
        user_key: "11111111-1111-1111-1111-111111111111".into(),
        name: "alice".into(),
        joined_at: "2026-09-15T00:00:00+00:00".into(),
        reward: 1_000_000,
    };
    let ser = serde_json::to_value(&v).expect("serialize");
    assert_eq!(
        ser,
        json!({
            "userKey": "11111111-1111-1111-1111-111111111111",
            "name": "alice",
            "joinedAt": "2026-09-15T00:00:00+00:00",
            "reward": 1_000_000
        }),
        "camelCase 形状必须与前端 wire.rs 镜像一致"
    );
    assert!(ser["reward"].is_number(), "reward 必须是数字");
    assert!(ser["joinedAt"].is_string(), "joinedAt 必须是字符串");
    // 往返：wire.rs 拿到 JSON 要反序列化，derive 的 Deserialize 必须能还原。
    assert_eq!(
        serde_json::from_value::<InviteeView>(ser).expect("deserialize roundtrip"),
        v
    );
}

/// list_invitees 的 SQL 形状对账（离线，不需要 PG）。
///
/// 这里证的是查询形状与契约一致，不是「能连库」。每条断言钉一个会
/// 静默回归的点——这些错都不报错，只让前端列表慢慢变坏：
/// - 缺 LEFT JOIN affiliate_rewards → reward 恒 0（奖励列全 0 且无异常）；
/// - 缺 GROUP BY → 一个被邀人按奖励笔数展开成 N 行（列表重复渲染）；
/// - 缺 COALESCE(NULLIF(...)) → display_name 空的用户显示空名；
/// - 缺 ORDER BY ... DESC / LIMIT → 顺序不定 / 无封顶。
#[test]
fn list_invitees_sql_shape() {
    let q = AFFILIATE_SRC;
    assert!(
        q.contains("FROM affiliate_links l"),
        "主表必须是 affiliate_links（inviter 视角）"
    );
    assert!(
        q.contains("JOIN auth_users u ON u.key = l.invitee_key"),
        "必须 join auth_users 取展示名"
    );
    assert!(
        q.contains("LEFT JOIN affiliate_rewards r ON r.invitee_key = l.invitee_key"),
        "必须 LEFT JOIN affiliate_rewards——缺它 reward 恒 0"
    );
    assert!(
        q.contains("COALESCE(NULLIF(u.display_name, ''), u.username)"),
        "展示名必须 display_name 空时回落 username"
    );
    assert!(
        q.contains("GROUP BY l.invitee_key"),
        "必须 GROUP BY——缺它 LEFT JOIN 会把被邀人放大成重复行"
    );
    assert!(
        q.contains("ORDER BY l.created_at DESC"),
        "必须按 created_at 倒序"
    );
    assert!(q.contains("LIMIT $2"), "必须有 limit 封顶");
}

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

/// 建服务与池；**PG 不可达返回 None 让调用方 skip**（理由见文件头）。
async fn make_svc() -> Option<(AffiliateService, sqlx::PgPool)> {
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
    Some((
        AffiliateService::new(pool.clone(), WalletService::new(pool.clone())),
        pool,
    ))
}

/// 直插 auth_users；`display_name=None` 走 username 回落分支（不走注册
/// seed hook，避免默认值干扰回落断言）。
async fn make_user(pool: &sqlx::PgPool, display_name: Option<&str>) -> Uuid {
    let key = Uuid::new_v4();
    sqlx::query(
        r#"INSERT INTO auth_users (key, username, display_name, email, password_hash, role, status, quota, used_quota, group_id, auth_version)
           VALUES ($1, $2, $3, NULL, 'x', 1, 1, 0, 0, 'default', 1)"#,
    )
    .bind(key)
    .bind(format!("il_user_{}", key.simple()))
    .bind(display_name)
    .execute(pool)
    .await
    .expect("insert user");
    key
}

/// 清理某用户全部 affiliate/余额痕迹（邀请两表按 inviter/invitee 双角色删——
/// 测试里同一用户常兼两角；auth_users 行保留，用户名带 uuid 不冲突）。
async fn cleanup(pool: &sqlx::PgPool, user: Uuid) {
    sqlx::query("DELETE FROM user_balances WHERE user_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM affiliate_links WHERE inviter_key = $1 OR invitee_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
    sqlx::query("DELETE FROM affiliate_rewards WHERE inviter_key = $1 OR invitee_key = $1")
        .bind(user)
        .execute(pool)
        .await
        .ok();
}

/// 真实列表行为（PG，不可达 skip）。
///
/// 场景：inviter 绑 2 个被邀人（a 有展示名且被发奖；b 无展示名且无奖），
/// 另一个 stranger 绑自己的被邀人 c。断言链：
/// - 恰好 2 条且只含 inviter 的人（c 不泄漏——`WHERE l.inviter_key` 是
///   唯一隔离边界，跨用户查询是越权 bug）；
/// - created_at 倒序：后绑的 b 排第一；
/// - name 回落：b 的 display_name 为 NULL → 显示 username；
/// - reward：a = 实际入账额（SUM 聚合真跑过），b = 0（LEFT JOIN 缺行 +
///   COALESCE 兜底，而非 NULL/缺字段）；
/// - userKey 可 parse 回 Uuid、joinedAt 可 parse 回 RFC3339（DTO 字符串口径）。
#[tokio::test]
async fn list_invitees_real_shape() {
    let Some((aff, pool)) = make_svc().await else {
        return;
    };
    let inviter = make_user(&pool, Some("Alice Display")).await;
    let a = make_user(&pool, Some("Bob Display")).await;
    let b = make_user(&pool, None).await;
    let stranger = make_user(&pool, None).await;
    let c = make_user(&pool, None).await;

    aff.bind_inviter(inviter, a).await.expect("bind a");
    aff.bind_inviter(inviter, b).await.expect("bind b");
    aff.bind_inviter(stranger, c)
        .await
        .expect("bind c to stranger");
    let rewarded = aff
        .reward_invite_referral(inviter, a)
        .await
        .expect("reward a");
    assert!(rewarded > 0, "发奖必须真入账，否则下面 reward 断言无意义");

    let items = aff.list_invitees(inviter, 50).await.expect("list");

    assert_eq!(
        items.len(),
        2,
        "只返回 inviter 的被邀人；stranger 的 c 不算"
    );
    assert_eq!(
        items[0].user_key,
        b.to_string(),
        "created_at DESC → 后绑的 b 在前"
    );
    assert_eq!(items[1].user_key, a.to_string());
    assert_eq!(
        items[0].name,
        format!("il_user_{}", b.simple()),
        "display_name 为 NULL → 回落 username"
    );
    assert_eq!(items[1].name, "Bob Display");
    assert_eq!(items[1].reward, rewarded, "reward = 该被邀人的 SUM(amount)");
    assert_eq!(
        items[0].reward, 0,
        "无奖励行 = 0（LEFT JOIN 缺行 + COALESCE 兜底，不是 NULL）"
    );
    assert!(
        items.iter().all(|i| Uuid::parse_str(&i.user_key).is_ok()),
        "userKey 必须是可解析的 UUID 字符串"
    );
    assert!(
        items
            .iter()
            .all(|i| DateTime::<FixedOffset>::parse_from_rfc3339(&i.joined_at).is_ok()),
        "joinedAt 必须是可解析的 RFC3339 字符串"
    );

    for u in [inviter, a, b, stranger, c] {
        cleanup(&pool, u).await;
    }
}
