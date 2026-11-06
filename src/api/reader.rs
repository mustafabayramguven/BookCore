use axum::{
    body::Body,
    extract::{Path as AxPath, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::db::queries::books::{get_book_by_id, get_book_chapters};
use crate::error::{AppError, AppResult};
use crate::models::BookChapter;
use crate::reader::cbz_reader::get_cbz_page;
use crate::reader::epub_reader::{get_epub_asset, get_epub_chapter_html};
use crate::reader::pdf_reader::{read_pdf_range, PdfStreamResponse};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AssetQuery {
    pub file: String,
}

pub async fn get_cover_handler(
    State(state): State<AppState>,
    AxPath(id): AxPath<i64>,
) -> AppResult<Response> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    if let Some(ref cover_path) = book.cover_image_path {
        let path = Path::new(cover_path);
        if path.exists() {
            let bytes = tokio::fs::read(path).await?;
            let mime = mime_guess::from_path(path)
                .first_or_octet_stream()
                .to_string();

            return Ok((
                StatusCode::OK,
                [
                    (header::CONTENT_TYPE, mime),
                    (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
                ],
                bytes,
            )
                .into_response());
        }
    }

    // Default SVG placeholder cover
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 300 450" width="300" height="450">
            <rect width="300" height="450" fill="#1e293b"/>
            <rect x="15" y="15" width="270" height="420" rx="8" fill="#0f172a" stroke="#334155" stroke-width="2"/>
            <text x="150" y="200" fill="#94a3b8" font-family="sans-serif" font-size="20" font-weight="bold" text-anchor="middle">{}</text>
            <text x="150" y="240" fill="#64748b" font-family="sans-serif" font-size="14" text-anchor="middle">{}</text>
        </svg>"##,
        quick_xml_escape(&book.title),
        quick_xml_escape(&book.format)
    );

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "image/svg+xml".to_string()),
            (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
        ],
        svg,
    )
        .into_response())
}

pub async fn get_page_handler(
    State(state): State<AppState>,
    AxPath((id, page_num)): AxPath<(i64, usize)>,
) -> AppResult<Response> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    let file_path = PathBuf::from(book.file_path);
    let (bytes, mime) = tokio::task::spawn_blocking(move || get_cbz_page(&file_path, page_num))
        .await
        .map_err(|e| AppError::Internal(format!("Spawn error: {}", e)))??;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
        ],
        bytes,
    )
        .into_response())
}

pub async fn get_chapters_handler(
    State(state): State<AppState>,
    AxPath(id): AxPath<i64>,
) -> AppResult<Json<Vec<BookChapter>>> {
    let chapters = get_book_chapters(&state.pool, id).await?;
    Ok(Json(chapters))
}

pub async fn get_chapter_content_handler(
    State(state): State<AppState>,
    AxPath((id, chapter_idx)): AxPath<(i64, i64)>,
) -> AppResult<Html<String>> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    let chapters = get_book_chapters(&state.pool, id).await?;
    let chapter = chapters
        .iter()
        .find(|c| c.chapter_index == chapter_idx)
        .ok_or_else(|| AppError::NotFound(format!("Chapter index {} not found", chapter_idx)))?;

    let file_path = PathBuf::from(book.file_path);
    let content_href = chapter.content_href.clone();

    let html =
        tokio::task::spawn_blocking(move || get_epub_chapter_html(&file_path, &content_href, id))
            .await
            .map_err(|e| AppError::Internal(format!("Spawn error: {}", e)))??;

    Ok(Html(html))
}

pub async fn get_asset_handler(
    State(state): State<AppState>,
    AxPath(id): AxPath<i64>,
    Query(query): Query<AssetQuery>,
) -> AppResult<Response> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    let file_path = PathBuf::from(book.file_path);
    let asset_file = query.file;

    let (bytes, mime) =
        tokio::task::spawn_blocking(move || get_epub_asset(&file_path, &asset_file))
            .await
            .map_err(|e| AppError::Internal(format!("Spawn error: {}", e)))??;

    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, mime),
            (header::CACHE_CONTROL, "public, max-age=86400".to_string()),
        ],
        bytes,
    )
        .into_response())
}

pub async fn get_raw_pdf_handler(
    State(state): State<AppState>,
    AxPath(id): AxPath<i64>,
    headers: HeaderMap,
) -> AppResult<Response> {
    let book = get_book_by_id(&state.pool, id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Book {} not found", id)))?;

    let file_path = PathBuf::from(book.file_path);
    let range_header = headers
        .get(header::RANGE)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let stream_res =
        tokio::task::spawn_blocking(move || read_pdf_range(&file_path, range_header.as_deref()))
            .await
            .map_err(|e| AppError::Internal(format!("Spawn error: {}", e)))??;

    match stream_res {
        PdfStreamResponse::Full { bytes, total_size } => Ok((
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/pdf".to_string()),
                (header::ACCEPT_RANGES, "bytes".to_string()),
                (header::CONTENT_LENGTH, total_size.to_string()),
            ],
            Body::from(bytes),
        )
            .into_response()),
        PdfStreamResponse::Partial {
            bytes,
            start,
            end,
            total_size,
        } => {
            let content_range = format!("bytes {}-{}/{}", start, end, total_size);
            Ok((
                StatusCode::PARTIAL_CONTENT,
                [
                    (header::CONTENT_TYPE, "application/pdf".to_string()),
                    (header::ACCEPT_RANGES, "bytes".to_string()),
                    (header::CONTENT_RANGE, content_range),
                    (header::CONTENT_LENGTH, bytes.len().to_string()),
                ],
                Body::from(bytes),
            )
                .into_response())
        }
    }
}

fn quick_xml_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
