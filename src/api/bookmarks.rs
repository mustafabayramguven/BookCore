use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::db::queries::bookmarks::{create_bookmark, delete_bookmark, list_bookmarks};
use crate::error::{AppError, AppResult};
use crate::models::{Bookmark, CreateBookmarkDto};
use crate::state::AppState;

pub async fn list_bookmarks_handler(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
) -> AppResult<Json<Vec<Bookmark>>> {
    let bookmarks = list_bookmarks(&state.pool, book_id).await?;
    Ok(Json(bookmarks))
}

pub async fn create_bookmark_handler(
    State(state): State<AppState>,
    Path(book_id): Path<i64>,
    Json(dto): Json<CreateBookmarkDto>,
) -> AppResult<impl IntoResponse> {
    let bookmark = create_bookmark(&state.pool, book_id, &dto).await?;
    Ok((StatusCode::CREATED, Json(bookmark)))
}

pub async fn delete_bookmark_handler(
    State(state): State<AppState>,
    Path(bookmark_id): Path<i64>,
) -> AppResult<StatusCode> {
    let deleted = delete_bookmark(&state.pool, bookmark_id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!(
            "Bookmark {} not found",
            bookmark_id
        )))
    }
}
