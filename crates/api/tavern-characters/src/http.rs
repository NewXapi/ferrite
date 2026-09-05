//! axum 路由。挂在 `/tavern/characters` 下。

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tavern_storage::UserDirs;

use crate::{Character, delete, get as get_card, list, png, save};

#[derive(Clone)]
pub struct CharactersState {
    pub dirs: UserDirs,
}

pub fn router(state: CharactersState) -> Router {
    Router::new()
        .route("/", get(list_all).post(create))
        .route("/{name}", get(read_one).put(update).delete(remove))
        .with_state(Arc::new(state))
}

fn err(e: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::BAD_REQUEST, e.to_string())
}

async fn list_all(State(st): State<Arc<CharactersState>>) -> impl IntoResponse {
    list(&st.dirs.characters()).map(Json).map_err(err)
}

async fn read_one(
    State(st): State<Arc<CharactersState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    get_card(&st.dirs.characters(), &name)
        .map(Json)
        .map_err(err)
}

#[derive(Deserialize)]
pub struct CreateBody {
    pub file_name: String,
    #[serde(flatten)]
    pub character: Character,
}

async fn create(
    State(st): State<Arc<CharactersState>>,
    Json(body): Json<CreateBody>,
) -> impl IntoResponse {
    save(
        &st.dirs.characters(),
        &body.file_name,
        &body.character,
        Some(&png::minimal_png()),
    )
    .map(|_| StatusCode::CREATED)
    .map_err(err)
}

async fn update(
    State(st): State<Arc<CharactersState>>,
    Path(name): Path<String>,
    Json(character): Json<Character>,
) -> impl IntoResponse {
    save(&st.dirs.characters(), &name, &character, None)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}

async fn remove(
    State(st): State<Arc<CharactersState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    delete(&st.dirs.characters(), &name)
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(err)
}
