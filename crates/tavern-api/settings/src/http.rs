//! axum 路由。挂在 `/tavern/settings` 下。

use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tavern_storage::UserDirs;

use crate::{load, save};

#[derive(Clone)]
pub struct SettingsState {
    pub dirs: UserDirs,
}

pub fn router(state: SettingsState) -> Router {
    Router::new()
        .route("/", get(read).put(write))
        .with_state(Arc::new(state))
}

fn err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn read(State(st): State<Arc<SettingsState>>) -> impl IntoResponse {
    load(&st.dirs.settings_file()).map(Json).map_err(err)
}

async fn write(
    State(st): State<Arc<SettingsState>>,
    Json(value): Json<serde_json::Value>,
) -> impl IntoResponse {
    save(&st.dirs.settings_file(), &value)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}
