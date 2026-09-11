//! admin-proxy 集成测试 — 需要 PG (DATABASE_URL)。
//!
//! 跑：`DATABASE_URL=postgres://ferrite:ferrite@127.0.0.1:5433/ferrite \
//!      cargo test -p admin-proxy -- --ignored`
//!
//! 验证三件事（全走真表）：
//! 1. CRUD 往返 + 凭据掩码（原始 URL 不回传）
//! 2. channel_keys 引用完整性（不存在的渠道名被拒）
//! 3. `load_proxy_snapshot` 产出与 `ProxyNode` 对齐的快照（M1-2 数据面消费入口）

use admin_proxy::ProxyNodeService;
use sqlx::postgres::PgPoolOptions;
use std::time::Duration;
use uuid::Uuid;

static INIT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn db_url() -> String {
    std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ferrite:ferrite@127.0.0.1:5433/ferrite".into())
}

async fn make_svc() -> (ProxyNodeService, sqlx::PgPool) {
    let _guard = INIT.lock().await;
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&db_url())
        .await
        .expect("PG connect");
    db_bootstrap::run_migrations(&pool).await.expect("migrations");
    // 渠道表是 channel_keys 引用完整性的校验对象，一并建
    db_bootstrap::run_migrations(&pool).await.expect("migrations");
    (ProxyNodeService::new(pool.clone()), pool)
}

/// 唯一渠道名，避免并行测试互踩
async fn unique_channel(pool: &sqlx::PgPool, name: &str) {
    sqlx::query("DELETE FROM api_channels WHERE name = $1")
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO api_channels (key, name) VALUES ($1, $2)")
        .bind(Uuid::now_v7())
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
}

/// CRUD 往返：create → list（掩码）→ update → delete；原始 URL 永不回传。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn crud_roundtrip_masks_credentials() {
    let (svc, pool) = make_svc().await;
    unique_channel(&pool, "crud-test-ch").await;

    let v = svc
        .create(
            "测试节点",
            "vless://11111111-2222-3333-4444-555555555555@example.com:443?flow=xtls-rprx-vision&pbk=AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8&sid=01ab&sni=cdn.example.com",
            &["crud-test-ch".into()],
            5,
            "",
        )
        .await
        .expect("create");

    assert!(
        v.url_masked.starts_with("vless://***@example.com:443?***"),
        "掩码形状: {}",
        v.url_masked
    );
    assert!(!v.url_masked.contains("11111111"), "UUID 不能出现在掩码里");
    assert_eq!(v.channel_keys, vec!["crud-test-ch".to_string()]);

    let list = svc.list(false).await.expect("list");
    assert!(list.iter().any(|n| n.key == v.key), "list 必须含新节点");

    let v2 = svc
        .update(
            Uuid::parse_str(&v.key).unwrap(),
            &admin_proxy::NodeRequest {
                name: "改名".into(),
                url: "socks5://127.0.0.1:17890".into(),
                channel_keys: vec!["crud-test-ch".into()],
                priority: 9,
                enabled: false,
                remark: "updated".into(),
            },
        )
        .await
        .expect("update");
    assert_eq!(v2.name, "改名");
    assert!(!v2.enabled);

    svc.delete(Uuid::parse_str(&v2.key).unwrap())
        .await
        .expect("delete");
    let after = svc.list(false).await.expect("list after delete");
    assert!(!after.iter().any(|n| n.key == v2.key));
    sqlx::query("DELETE FROM api_channels WHERE name = 'crud-test-ch'")
        .execute(&pool)
        .await
        .unwrap();
}

/// channel_keys 引用完整性：不存在的渠道名被拒（HTTP 400 语义）。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn create_rejects_unknown_channel() {
    let (svc, _pool) = make_svc().await;
    let err = svc
        .create(
            "坏绑定",
            "socks5://127.0.0.1:17890",
            &["不存在的渠道-xyz".into()],
            0,
            "",
        )
        .await;
    assert!(err.is_err(), "未知渠道必须被拒");
    assert!(
        err.err().unwrap().to_string().contains("不存在"),
        "错误信息要指明渠道不存在"
    );
}

/// load_proxy_snapshot：enabled 的节点进快照、disabled 不进、id 连续编号。
#[tokio::test]
#[ignore = "requires DATABASE_URL"]
async fn load_proxy_snapshot_filters_enabled() {
    let (svc, pool) = make_svc().await;
    unique_channel(&pool, "snap-test-ch").await;

    let on = svc
        .create(
            "开",
            "socks5://127.0.0.1:17891",
            &["snap-test-ch".into()],
            1,
            "",
        )
        .await
        .expect("create on");
    let _off = svc
        .create(
            "关",
            "socks5://127.0.0.1:17892",
            &["snap-test-ch".into()],
            2,
            "",
        )
        .await
        .expect("create off");

    let snap = admin_proxy::load_proxy_snapshot(&pool).await.expect("load");
    assert!(
        snap.nodes
            .iter()
            .all(|n| n.channel_keys.contains(&"snap-test-ch".into()))
    );
    // enabled=false 的（priority 2, 关）不能出现在快照里——按 URL 端口区分
    assert!(
        snap.nodes.iter().all(|n| n.port != 17892),
        "disabled 节点不得进快照"
    );
    assert!(
        snap.nodes.iter().any(|n| n.port == 17891),
        "enabled 节点必须在"
    );

    // 清理
    for k in [on.key, _off.key] {
        svc.delete(Uuid::parse_str(&k).unwrap()).await.unwrap();
    }
    sqlx::query("DELETE FROM api_channels WHERE name = 'snap-test-ch'")
        .execute(&pool)
        .await
        .unwrap();
}
