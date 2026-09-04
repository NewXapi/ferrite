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

use crate::{PresetError, delete, list, restore, save};

#[derive(Clone)]
pub struct PresetsState {
    pub dirs: UserDirs,
}

pub fn router(state: PresetsState) -> Router {
    Router::new()
        .route(
            "/",
            get(list_handler).put(save_handler).delete(delete_handler),
        )
        .route("/restore", post(restore_handler))
        .with_state(Arc::new(state))
}

#[derive(Deserialize)]
pub struct ListQuery {
    #[serde(rename = "apiId")]
    pub api_id: String,
}

#[derive(Deserialize)]
pub struct DeleteQuery {
    #[serde(rename = "apiId")]
    pub api_id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct SaveBody {
    #[serde(rename = "apiId")]
    pub api_id: String,
    pub name: String,
    pub preset: Value,
}

#[derive(Deserialize)]
pub struct RestoreBody {
    #[serde(rename = "apiId")]
    pub api_id: String,
    pub name: String,
}

#[derive(Serialize)]
pub struct SaveResponse {
    pub name: String,
}

/// 业务错误 → HTTP。
fn to_response(e: PresetError) -> (StatusCode, String) {
    match e {
        PresetError::NotFound(_) => (StatusCode::NOT_FOUND, e.to_string()),
        // 参数问题归客户端。
        PresetError::UnknownApiId(_) | PresetError::Storage(_) => {
            (StatusCode::BAD_REQUEST, e.to_string())
        }
        // 磁盘故障与盘上坏 JSON 是服务端问题，前端不该当成参数错误重试。
        PresetError::Io(_) | PresetError::Json(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    }
}

async fn list_handler(
    State(st): State<Arc<PresetsState>>,
    Query(q): Query<ListQuery>,
) -> impl IntoResponse {
    list(st.dirs.root(), &q.api_id)
        .map(Json)
        .map_err(|e| to_response(e))
}

async fn save_handler(
    State(st): State<Arc<PresetsState>>,
    Json(body): Json<SaveBody>,
) -> impl IntoResponse {
    match save(st.dirs.root(), &body.api_id, &body.name, &body.preset) {
        Ok(()) => Ok(Json(SaveResponse { name: body.name }).into_response()),
        Err(e) => Err(to_response(e)),
    }
}

async fn delete_handler(
    State(st): State<Arc<PresetsState>>,
    Query(q): Query<DeleteQuery>,
) -> impl IntoResponse {
    match delete(st.dirs.root(), &q.api_id, &q.name) {
        Ok(()) => Ok(StatusCode::NO_CONTENT.into_response()),
        Err(e) => Err(to_response(e)),
    }
}

async fn restore_handler(
    State(st): State<Arc<PresetsState>>,
    Json(body): Json<RestoreBody>,
) -> impl IntoResponse {
    Json(restore(st.dirs.root(), &body.api_id, &body.name)).into_response()
}
