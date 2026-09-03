//! axum 路由。挂在 `/tavern/chats` 下。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use tavern_storage::UserDirs;

use crate::{Message, delete, load, recent, save};

#[derive(Clone)]
pub struct ChatsState {
    pub dirs: UserDirs,
}

pub fn router(state: ChatsState) -> Router {
    Router::new()
        .route("/{character}", get(list_recent))
        .route("/{character}/{chat}", get(read).put(write).delete(remove))
        .with_state(Arc::new(state))
}

fn err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn list_recent(
    State(st): State<Arc<ChatsState>>,
    Path(character): Path<String>,
) -> impl IntoResponse {
    recent(&st.dirs.chats(), &character).map(Json).map_err(err)
}

async fn read(
    State(st): State<Arc<ChatsState>>,
    Path((character, chat)): Path<(String, String)>,
) -> impl IntoResponse {
    load(&st.dirs.chats(), &character, &chat)
        .map(Json)
        .map_err(err)
}

async fn write(
    State(st): State<Arc<ChatsState>>,
    Path((character, chat)): Path<(String, String)>,
    Json(messages): Json<Vec<Message>>,
) -> impl IntoResponse {
    save(&st.dirs.chats(), &character, &chat, &messages)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}

async fn remove(
    State(st): State<Arc<ChatsState>>,
    Path((character, chat)): Path<(String, String)>,
) -> impl IntoResponse {
    delete(&st.dirs.chats(), &character, &chat)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}
