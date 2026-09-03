//! axum 路由。挂在 `/tavern/secrets` 下。
//!
//! 只暴露「是否已配置」与写入删除，**永不回显明文**。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tavern_storage::UserDirs;

use crate::{remove, state as read_state, write};

#[derive(Clone)]
pub struct SecretsState {
    pub dirs: UserDirs,
}

pub fn router(state: SecretsState) -> Router {
    Router::new()
        .route("/", get(list_state))
        .route("/{key}", axum::routing::put(put_one).delete(delete_one))
        .with_state(Arc::new(state))
}

fn err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn list_state(State(st): State<Arc<SecretsState>>) -> impl IntoResponse {
    read_state(&st.dirs.secrets_file()).map(Json).map_err(err)
}

#[derive(Deserialize)]
pub struct PutBody {
    pub value: String,
}

async fn put_one(
    State(st): State<Arc<SecretsState>>,
    Path(key): Path<String>,
    Json(body): Json<PutBody>,
) -> impl IntoResponse {
    write(&st.dirs.secrets_file(), &key, &body.value)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}

async fn delete_one(
    State(st): State<Arc<SecretsState>>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    remove(&st.dirs.secrets_file(), &key)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}
