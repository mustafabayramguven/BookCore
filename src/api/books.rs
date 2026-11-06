use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use std::path::PathBuf;

use crate::db::queries::books::{
    delete_book, get_book_by_id, get_book_chapters, get_default_library, list_books,
};
use crate::db::queries::progress::get_progress;
use crate::error::{AppError, AppResult};
use crate::models::{BookDetailDto, BookFilterQuery, BookSummaryDto, ScanResultDto};
use crate::scanner::scan_library;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ScanRequest {
    pub library_path: Option<String>,
    pub force_refresh: Option<bool>,
}

pub async fn scan_handler(
    State(state): State<AppState>,
    body: Option<Json<ScanRequest>>,
) -> AppResult<Json<ScanResultDto>> {
    let default_lib = get_default_library(&state.pool).await?;

    let (lib_id, lib_path) = if let Some(Json(req)) = body {
        if let Some(custom_path) = req.library_path {
            (default_lib.id, PathBuf::from(custom_path))
        } else {
            (default_lib.id, PathBuf::from(default_lib.path))
        }
    } else {
        (default_lib.id, PathBuf::from(default_lib.path))
    };

    let result = scan_library(&state.pool, lib_id, &lib_path, &state.config.covers_dir()).await?;

    Ok(Json(result))
}

pub async fn list_books_handler(
    State(state): State<AppState>,
    Query(filter): Query<BookFilterQuery>,
) -> AppResult<Json<Vec<BookSummaryDto>>> {
    let books = list_books(&state.pool, &filter).await?;
    Ok(Json(books))
}

pub async fn get_book_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<Json<BookDetailDto>> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    let chapters = get_book_chapters(&state.pool, id).await?;
    let progress = get_progress(&state.pool, id).await?;

    let (current_page, scroll_anchor, is_finished) = if let Some(p) = progress {
        (p.current_page, p.scroll_anchor, p.is_finished)
    } else {
        (0, None, false)
    };

    let progress_percent = if is_finished {
        100.0
    } else if book.page_count > 0 {
        ((current_page as f64 / book.page_count as f64) * 100.0).clamp(0.0, 100.0)
    } else {
        0.0
    };

    Ok(Json(BookDetailDto {
        book,
        chapters,
        current_page,
        scroll_anchor,
        is_finished,
        progress_percent: (progress_percent * 100.0).round() / 100.0,
    }))
}

pub async fn delete_book_handler(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> AppResult<StatusCode> {
    let deleted = delete_book(&state.pool, id).await?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AppError::NotFound(format!("Book {} not found", id)))
    }
}
