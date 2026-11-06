use axum::{
    extract::{Path, State},
    Json,
};

use crate::db::queries::progress::{get_progress, upsert_progress};
use crate::error::AppResult;
use crate::models::{ProgressUpdateDto, ReadingProgress};
use crate::state::AppState;

pub async fn get_progress_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<ReadingProgress>> {
    let progress = get_progress(&state.pool, id).await?;

    let result = progress.unwrap_or(ReadingProgress {
        id: 0,
        book_id: id,
        current_page: 0,
        scroll_anchor: None,
        is_finished: false,
        total_reads: 0,
        last_read_at: String::new(),
    });

    Ok(Json(result))
}

pub async fn save_progress_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(update): Json<ProgressUpdateDto>,
) -> AppResult<Json<ReadingProgress>> {
    let result = upsert_progress(&state.pool, id, &update).await?;
    Ok(Json(result))
}
