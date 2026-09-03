//! 酒馆域装配
//!
//! 把 `crates/tavern-api/*` 的路由聚到 `/tavern` 下。crate 只提供 `router()`，
//! 数据根目录与上游地址由本进程决定。

use axum::Router;
use tavern_storage::{DataRoot, StorageError};

/// 酒馆装配参数。
pub struct TavernConfig {
    /// 数据根目录，默认 `data/`。
    pub data_root: String,
    /// generate 转发的 OpenAI 兼容上游。
    pub upstream: String,
}

impl Default for TavernConfig {
    fn default() -> Self {
        Self {
            data_root: "data".into(),
            upstream: "http://127.0.0.1:3000".into(),
        }
    }
}

/// 建目录并组装 `/tavern` 路由。
pub fn router(cfg: &TavernConfig) -> Result<Router, StorageError> {
    let root = DataRoot::new(&cfg.data_root);
    let identity = tavern_auth::Identity::default_user();
    let dirs = identity.dirs(&root);
    dirs.ensure()?;

    let generate_state = tavern_generate::GenerateState::new(
        dirs.clone(),
        tavern_generate::GenerateConfig {
            upstream: cfg.upstream.clone(),
        },
    );

    Ok(Router::new().nest(
        "/tavern",
        Router::new()
            .nest(
                "/characters",
                tavern_characters::router(tavern_characters::CharactersState { dirs: dirs.clone() }),
            )
            .nest(
                "/chats",
                tavern_chats::router(tavern_chats::ChatsState { dirs: dirs.clone() }),
            )
            .nest(
                "/settings",
                tavern_settings::router(tavern_settings::SettingsState { dirs: dirs.clone() }),
            )
            .nest(
                "/secrets",
                tavern_secrets::router(tavern_secrets::SecretsState { dirs: dirs.clone() }),
            )
            .merge(tavern_generate::router(generate_state)),
    ))
}
