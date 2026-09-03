//! axum 路由。挂在 `/presets` 下：列表/保存/删除都走 `/`，恢复走 `/restore`。

use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tavern_storage::UserDirs;

use crate::{delete, list, restore, save, PresetError};

#[derive(Clone)]
pub struct PresetsState {
    pub dirs: UserDirs,
}

pub fn router(state: PresetsState) -> Router {
    Router::new()
        .route("/", get(list_handler).put(save_handler).delete(delete_handler))
        .route("/restore", post(restore_handler))
        .with_state(Arc::new(state))
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub apiId: String,
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    pub apiId: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct SaveBody {
    pub apiId: String,
    pub name: String,
    pub preset: Value,
}

#[derive(Deserialize)]
pub struct RestoreBody {
    pub apiId: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub name: String,
}

#[derive(Serialize)]
pub struct RestoreResponse {
    pub isDefault: bool,
    pub preset: Value,
}

/// 业务错误 → HTTP。
fn to_response(e: PresetError) -> (StatusCode, String) {
    match e {
        PresetError::NotFound(_) => (StatusCode::NOT_FOUND, e.to_string()),
        _ => (StatusCode::BAD_REQUEST, e.to_string()),
    }
}

async fn list_handler(
    State(st): State<Arc<PresetsState>>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    list(st.dirs.root(), &q.apiId).map(Json).map_err(|e| to_response(e))
}

async fn save_handler(
    State(st): State<Arc<PresetsState>>,
    Json(body): Json<SaveBody>,
) -> impl IntoResponse {
    match save(st.dirs.root(), &body.apiId, &body.name, &body.preset) {
        Ok(()) => Ok(Json(SaveResponse { name: body.name }).into_response()),
        Err(e) => Err(to_response(e)),
    }
}

async fn delete_handler(
    State(st): State<Arc<PresetsState>>,
    Query(q): Query<DeleteQuery>,
) -> impl IntoResponse {
    match delete(st.dirs.root(), &q.apiId, &q.name) {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(e) => Err(to_response(e)),
    }
}

async fn restore_handler(
    State(st): State<Arc<PresetsState>>,
    Json(body): Json<RestoreBody>,
) -> impl IntoResponse {
    let out = restore(st.dirs.root(), &body.apiId, &body.name);
    Json(RestoreResponse {
        isDefault: out["isDefault"].as_bool().unwrap_or(false),
        preset: out["preset"].clone(),
    })
    .into_response()
}