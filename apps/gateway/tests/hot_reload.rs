use dispatch::Dispatch;
use dispatch::{Dispatcher, MemoryHealthTable, Snapshot};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test]
async fn dispatcher_set_snapshot_applies_immediately() {
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Dispatcher::new(None, health.clone());
    let err = dispatcher.select("g", "gpt-4o", &[]).unwrap_err();
    assert!(matches!(err, dispatch::DispatchError::SnapshotNotReady));

    let mut channels = std::collections::HashMap::new();
    channels.insert(
        "ch1".into(),
        contract::records::ChannelRecord {
            meta: contract::records::SyncMeta {
                key: "ch1".into(),
                schema_version: 1,
                logical_version: 1,
                origin: "test".into(),
                updated_at: chrono::Utc::now(),
            },
            name: "ch1".into(),
            provider_type: "openai".into(),
            base_url: "https://api.example".into(),
            keys: vec![contract::records::ChannelKey {
                index: 0,
                secret: "sk-test".into(),
                rpm_limit: 0,
            }],
            max_concurrency: 10,
            status: 1,
            groups: vec!["default".into()],
            settings: serde_json::Value::Null,
        },
    );
    let snapshot = Snapshot {
        units: vec![contract::records::RouteUnitRecord {
            meta: contract::records::SyncMeta {
                key: "u1".into(),
                schema_version: 1,
                logical_version: 1,
                origin: "test".into(),
                updated_at: chrono::Utc::now(),
            },
            group: "default".into(),
            public_model: "gpt-4o".into(),
            channel_key: "ch1".into(),
            key_index: 0,
            upstream_model: "gpt-4o".into(),
            priority: 10,
            weight: 10,
            status: 1,
        }],
        channels,
    };
    dispatcher.set_snapshot(Arc::new(snapshot));
    let candidate = dispatcher.select("default", "gpt-4o", &[]).unwrap();
    assert_eq!(candidate.base_url, "https://api.example");
    assert_eq!(candidate.secret, "sk-test");
}

#[tokio::test]
async fn reload_replaces_pipeline_state_without_restart() {
    let health = Arc::new(MemoryHealthTable::new());
    let dispatcher = Arc::new(Dispatcher::new(None, health.clone()));
    let d1 = dispatcher.clone();
    let writer = tokio::spawn(async move {
        for i in 0..50 {
            let mut channels = std::collections::HashMap::new();
            channels.insert(
                "ch1".into(),
                contract::records::ChannelRecord {
                    meta: contract::records::SyncMeta {
                        key: "ch1".into(),
                        schema_version: 1,
                        logical_version: 1,
                        origin: "test".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    name: "ch1".into(),
                    provider_type: "openai".into(),
                    base_url: "https://api.example".into(),
                    keys: vec![contract::records::ChannelKey {
                        index: 0,
                        secret: "sk".into(),
                        rpm_limit: 0,
                    }],
                    max_concurrency: 10,
                    status: 1,
                    groups: vec!["default".into()],
                    settings: serde_json::Value::Null,
                },
            );
            let snap = Snapshot {
                units: vec![contract::records::RouteUnitRecord {
                    meta: contract::records::SyncMeta {
                        key: format!("u{}", i),
                        schema_version: 1,
                        logical_version: 1,
                        origin: "test".into(),
                        updated_at: chrono::Utc::now(),
                    },
                    group: "default".into(),
                    public_model: "gpt-4o".into(),
                    channel_key: "ch1".into(),
                    key_index: 0,
                    upstream_model: "gpt-4o".into(),
                    priority: 10,
                    weight: 10,
                    status: 1,
                }],
                channels,
            };
            d1.set_snapshot(Arc::new(snap));
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    });
    let d2 = dispatcher.clone();
    let reader = tokio::spawn(async move {
        let mut ok = 0;
        for _ in 0..100 {
            if d2.select("default", "gpt-4o", &[]).is_ok() {
                ok += 1;
            }
            tokio::time::sleep(Duration::from_micros(500)).await;
        }
        ok
    });
    let (r1, r2) = tokio::join!(writer, reader);
    r1.unwrap();
    let successes = r2.unwrap();
    assert!(
        successes > 0,
        "concurrent reload should serve requests, got {successes}"
    );
}
